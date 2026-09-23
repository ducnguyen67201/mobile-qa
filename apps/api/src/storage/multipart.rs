//! Narrow S3-compatible multipart control protocol. Object bytes never pass through
//! these requests. The existing Loco adapter remains responsible for object IO.
//! Credentials and signed capabilities deliberately have no Debug implementation.
use crate::errors::{ApiFailure, ApiResult};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use quick_xml::{events::Event, Reader};
use reqwest::{Client, Method, StatusCode};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, time::Duration};
use url::Url;

pub const PART_BYTES: i64 = 16 * 1024 * 1024;
pub const MAX_PARTS: u16 = 128;
const URL_SECONDS: i64 = 600;
const CONTROL_RESPONSE_LIMIT: usize = 2 * 1024 * 1024;

pub struct MultipartStart {
    pub upload_id: String,
}

pub struct MultipartPart {
    pub part_number: u16,
    pub etag: String,
}

pub struct MultipartPartUrl {
    pub url: String,
    pub expires_at: DateTime<Utc>,
    /// Content-Length is signed but omitted here: browsers set it from the Blob.
    pub headers: BTreeMap<String, String>,
}

pub struct StoredPart {
    pub part_number: u16,
    pub etag: String,
    pub bytes: i64,
}

pub struct MultipartUpload {
    pub key: String,
    pub upload_id: String,
    pub created_at: DateTime<Utc>,
}

pub(super) struct MultipartClient {
    endpoint: Url,
    bucket: String,
    region: String,
    access_key: String,
    secret_key: String,
    http: Client,
}

fn unavailable() -> ApiFailure {
    ApiFailure::new(
        503,
        "storage_unavailable",
        "Object storage is unavailable. Retry the upload",
    )
}

fn expired() -> ApiFailure {
    ApiFailure::new(
        409,
        "multipart_expired",
        "The object storage upload is no longer active",
    )
}

impl MultipartClient {
    pub(super) fn from_env() -> loco_rs::Result<Self> {
        let var = |name: &str| {
            std::env::var(name).map_err(|_| loco_rs::Error::string(&format!("{name} is required")))
        };
        let endpoint = Url::parse(&var("ARTIFACT_S3_ENDPOINT")?)
            .map_err(|_| loco_rs::Error::string("Invalid S3 endpoint"))?;
        let bucket = var("ARTIFACT_S3_BUCKET")?;
        if endpoint.scheme() != "https"
            || endpoint.host_str().is_none()
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
            || endpoint.path() != "/"
            || bucket.is_empty()
            || !bucket
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b".-".contains(&c))
        {
            return Err(loco_rs::Error::string(
                "S3 requires an HTTPS origin and a bucket name",
            ));
        }
        let http = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|_| loco_rs::Error::string("Cannot configure object storage transport"))?;
        Ok(Self {
            endpoint,
            bucket,
            region: var("ARTIFACT_S3_REGION")?,
            access_key: var("ARTIFACT_S3_ACCESS_KEY_ID")?,
            secret_key: var("ARTIFACT_S3_SECRET_ACCESS_KEY")?,
            http,
        })
    }

    fn object_url(&self, key: &str) -> ApiResult<Url> {
        if !valid_key(key, 4) {
            return Err(ApiFailure::internal());
        }
        let mut url = self.endpoint.clone();
        url.set_path(&format!("/{}/{}", self.bucket, key));
        Ok(url)
    }

    fn bucket_url(&self) -> Url {
        let mut url = self.endpoint.clone();
        url.set_path(&format!("/{}", self.bucket));
        url
    }

    /// Query signing binds every query parameter and required header. The part's
    /// actual payload hash is used instead of UNSIGNED-PAYLOAD so a renewed URL
    /// cannot replace an already authorized part with different bytes.
    fn presign(
        &self,
        method: &Method,
        mut url: Url,
        headers: &BTreeMap<String, String>,
        payload_hash: &str,
        now: DateTime<Utc>,
        seconds: i64,
    ) -> ApiResult<Url> {
        let date = now.format("%Y%m%d").to_string();
        let timestamp = now.format("%Y%m%dT%H%M%SZ").to_string();
        let scope = format!("{date}/{}/s3/aws4_request", self.region);
        let mut signed_headers = headers.clone();
        signed_headers.insert(
            "host".into(),
            url[url::Position::BeforeHost..url::Position::AfterPort].into(),
        );
        let header_names = signed_headers.keys().cloned().collect::<Vec<_>>().join(";");
        let canonical_headers = signed_headers
            .iter()
            .map(|(name, value)| {
                format!(
                    "{name}:{}\n",
                    value.split_whitespace().collect::<Vec<_>>().join(" ")
                )
            })
            .collect::<String>();
        let mut query = url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect::<Vec<_>>();
        query.extend([
            ("X-Amz-Algorithm".into(), "AWS4-HMAC-SHA256".into()),
            (
                "X-Amz-Credential".into(),
                format!("{}/{}", self.access_key, scope),
            ),
            ("X-Amz-Date".into(), timestamp.clone()),
            ("X-Amz-Expires".into(), seconds.to_string()),
            ("X-Amz-SignedHeaders".into(), header_names.clone()),
        ]);
        let query = canonical_query(&query);
        let canonical = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            method,
            url.path(),
            query,
            canonical_headers,
            header_names,
            payload_hash
        );
        let to_sign = format!(
            "AWS4-HMAC-SHA256\n{timestamp}\n{scope}\n{}",
            hex::encode(Sha256::digest(canonical.as_bytes()))
        );
        let date_key = hmac(
            format!("AWS4{}", self.secret_key).as_bytes(),
            date.as_bytes(),
        )?;
        let region_key = hmac(&date_key, self.region.as_bytes())?;
        let service_key = hmac(&region_key, b"s3")?;
        let signing_key = hmac(&service_key, b"aws4_request")?;
        let signature = hex::encode(hmac(&signing_key, to_sign.as_bytes())?);
        url.set_query(Some(&format!("{query}&X-Amz-Signature={signature}")));
        Ok(url)
    }

    async fn control(
        &self,
        method: Method,
        url: Url,
        body: String,
    ) -> ApiResult<(StatusCode, String)> {
        let digest = hex::encode(Sha256::digest(body.as_bytes()));
        let headers = BTreeMap::from([("x-amz-content-sha256".into(), digest.clone())]);
        let url = self.presign(&method, url, &headers, &digest, Utc::now(), URL_SECONDS)?;
        let mut response = self
            .http
            .request(method, url)
            .header("x-amz-content-sha256", digest)
            .header("content-type", "application/xml")
            .body(body)
            .send()
            .await
            .map_err(|_| unavailable())?;
        let status = response.status();
        if response
            .content_length()
            .is_some_and(|size| size > CONTROL_RESPONSE_LIMIT as u64)
        {
            return Err(unavailable());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| unavailable())? {
            if bytes.len() + chunk.len() > CONTROL_RESPONSE_LIMIT {
                return Err(unavailable());
            }
            bytes.extend_from_slice(&chunk);
        }
        let text = String::from_utf8(bytes).map_err(|_| unavailable())?;
        // Do not include provider bodies/errors/URLs: they can contain capabilities.
        Ok((status, text))
    }

    pub(super) async fn start(&self, key: &str) -> ApiResult<MultipartStart> {
        let mut url = self.object_url(key)?;
        url.set_query(Some("uploads="));
        let (status, body) = self.control(Method::POST, url, String::new()).await?;
        let parsed = parse_xml(&body, None)?;
        if !status.is_success() || parsed.root != "InitiateMultipartUploadResult" {
            return Err(unavailable());
        }
        if parsed.fields.get("Key").is_some_and(|value| value != key) {
            return Err(unavailable());
        }
        let upload_id = field(&parsed.fields, "UploadId")?.to_owned();
        validate_upload_id(&upload_id)?;
        Ok(MultipartStart { upload_id })
    }

    pub(super) fn part_url(
        &self,
        key: &str,
        upload_id: &str,
        part_number: u16,
        bytes: i64,
        sha256: &str,
    ) -> ApiResult<MultipartPartUrl> {
        validate_upload_id(upload_id)?;
        if !(1..=MAX_PARTS).contains(&part_number)
            || !(1..=PART_BYTES).contains(&bytes)
            || sha256.len() != 64
            || !sha256
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(ApiFailure::invalid("Invalid multipart part"));
        }
        let mut url = self.object_url(key)?;
        url.query_pairs_mut()
            .append_pair("partNumber", &part_number.to_string())
            .append_pair("uploadId", upload_id);
        let mut headers = BTreeMap::from([
            ("content-length".into(), bytes.to_string()),
            ("x-amz-content-sha256".into(), sha256.into()),
        ]);
        let now = Utc::now();
        let url = self.presign(&Method::PUT, url, &headers, sha256, now, URL_SECONDS)?;
        headers.remove("content-length");
        Ok(MultipartPartUrl {
            url: url.into(),
            expires_at: now + chrono::Duration::seconds(URL_SECONDS),
            headers,
        })
    }

    pub(super) fn download_url(&self, key: &str) -> ApiResult<MultipartPartUrl> {
        let now = Utc::now();
        let headers = BTreeMap::new();
        let url = self.presign(
            &Method::GET,
            self.object_url(key)?,
            &headers,
            "UNSIGNED-PAYLOAD",
            now,
            URL_SECONDS,
        )?;
        Ok(MultipartPartUrl {
            url: url.into(),
            expires_at: now + chrono::Duration::seconds(URL_SECONDS),
            headers,
        })
    }

    pub(super) async fn complete(
        &self,
        key: &str,
        upload_id: &str,
        parts: &[MultipartPart],
    ) -> ApiResult<()> {
        validate_upload_id(upload_id)?;
        if parts.is_empty() || parts.len() > usize::from(MAX_PARTS) {
            return Err(ApiFailure::invalid("Invalid multipart part count"));
        }
        let mut body = String::from("<CompleteMultipartUpload>");
        for (index, part) in parts.iter().enumerate() {
            if usize::from(part.part_number) != index + 1 || !valid_etag(&part.etag) {
                return Err(ApiFailure::invalid(
                    "Multipart parts must be complete and ordered",
                ));
            }
            body.push_str(&format!(
                "<Part><PartNumber>{}</PartNumber><ETag>{}</ETag></Part>",
                part.part_number,
                quick_xml::escape::escape(&part.etag)
            ));
        }
        body.push_str("</CompleteMultipartUpload>");
        let mut url = self.object_url(key)?;
        url.query_pairs_mut().append_pair("uploadId", upload_id);
        let (status, body) = self.control(Method::POST, url, body).await?;
        if status == StatusCode::NOT_FOUND {
            return Err(expired());
        }
        // S3 can send HTTP 200 before discovering a completion error. Require the
        // success XML root as well as status; a 200 <Error> is not publication.
        if !status.is_success() {
            return Err(unavailable());
        }
        let parsed = parse_xml(&body, None)?;
        if parsed.root != "CompleteMultipartUploadResult"
            || !valid_etag(field(&parsed.fields, "ETag")?)
            || parsed.fields.get("Key").is_some_and(|value| value != key)
        {
            return Err(unavailable());
        }
        Ok(())
    }

    pub(super) async fn abort(&self, key: &str, upload_id: &str) -> ApiResult<()> {
        validate_upload_id(upload_id)?;
        let mut url = self.object_url(key)?;
        url.query_pairs_mut().append_pair("uploadId", upload_id);
        let (status, _) = self.control(Method::DELETE, url, String::new()).await?;
        if status.is_success() || status == StatusCode::NOT_FOUND {
            Ok(())
        } else {
            Err(unavailable())
        }
    }

    pub(super) async fn parts(&self, key: &str, upload_id: &str) -> ApiResult<Vec<StoredPart>> {
        validate_upload_id(upload_id)?;
        let mut url = self.object_url(key)?;
        url.query_pairs_mut()
            .append_pair("uploadId", upload_id)
            .append_pair("max-parts", "1000");
        let (status, body) = self.control(Method::GET, url, String::new()).await?;
        if status == StatusCode::NOT_FOUND {
            return Err(expired());
        }
        if !status.is_success() {
            return Err(unavailable());
        }
        let parsed = parse_xml(&body, Some("Part"))?;
        if parsed.root != "ListPartsResult"
            || field(&parsed.fields, "IsTruncated")? != "false"
            || parsed.rows.len() > usize::from(MAX_PARTS)
        {
            return Err(unavailable());
        }
        let mut result = Vec::with_capacity(parsed.rows.len());
        for row in parsed.rows {
            let part_number = field(&row, "PartNumber")?
                .parse::<u16>()
                .map_err(|_| unavailable())?;
            let bytes = field(&row, "Size")?
                .parse::<i64>()
                .map_err(|_| unavailable())?;
            let etag = field(&row, "ETag")?.to_owned();
            if !(1..=MAX_PARTS).contains(&part_number)
                || !(1..=PART_BYTES).contains(&bytes)
                || !valid_etag(&etag)
                || result
                    .last()
                    .is_some_and(|previous: &StoredPart| previous.part_number >= part_number)
            {
                return Err(unavailable());
            }
            result.push(StoredPart {
                part_number,
                etag,
                bytes,
            });
        }
        Ok(result)
    }

    pub(super) async fn uploads(&self, prefix: &str) -> ApiResult<Vec<MultipartUpload>> {
        if !valid_key(prefix, 3) {
            return Err(ApiFailure::internal());
        }
        let prefix = format!("{prefix}/");
        let mut result = Vec::new();
        let mut markers: Option<(String, String)> = None;
        // Cleanup is bounded; a pathological provider response cannot keep a
        // process alive forever. Failure leaves the durable sweep eligible again.
        for _ in 0..10 {
            let mut url = self.bucket_url();
            url.query_pairs_mut()
                .append_pair("uploads", "")
                .append_pair("prefix", &prefix)
                .append_pair("max-uploads", "1000");
            if let Some((key, upload)) = &markers {
                url.query_pairs_mut()
                    .append_pair("key-marker", key)
                    .append_pair("upload-id-marker", upload);
            }
            let (status, body) = self.control(Method::GET, url, String::new()).await?;
            if !status.is_success() {
                return Err(unavailable());
            }
            let parsed = parse_xml(&body, Some("Upload"))?;
            if parsed.root != "ListMultipartUploadsResult" {
                return Err(unavailable());
            }
            for row in parsed.rows {
                let key = field(&row, "Key")?.to_owned();
                if !key.starts_with(&prefix) || !valid_key(&key, 4) {
                    return Err(unavailable());
                }
                let upload_id = field(&row, "UploadId")?.to_owned();
                validate_upload_id(&upload_id)?;
                let created_at = DateTime::parse_from_rfc3339(field(&row, "Initiated")?)
                    .map_err(|_| unavailable())?
                    .with_timezone(&Utc);
                result.push(MultipartUpload {
                    key,
                    upload_id,
                    created_at,
                });
            }
            match field(&parsed.fields, "IsTruncated")? {
                "false" => return Ok(result),
                "true" => {}
                _ => return Err(unavailable()),
            }
            let next = (
                field(&parsed.fields, "NextKeyMarker")?.to_owned(),
                field(&parsed.fields, "NextUploadIdMarker")?.to_owned(),
            );
            if markers.as_ref() == Some(&next) {
                return Err(unavailable());
            }
            markers = Some(next);
        }
        Err(unavailable())
    }
}

fn valid_key(key: &str, segments: usize) -> bool {
    key.split('/').count() == segments
        && key
            .split('/')
            .all(|s| uuid::Uuid::parse_str(s).is_ok_and(|id| id.to_string() == s))
}

fn validate_upload_id(upload_id: &str) -> ApiResult<()> {
    if upload_id.is_empty() || upload_id.len() > 2048 || upload_id.chars().any(char::is_control) {
        return Err(unavailable());
    }
    Ok(())
}

fn valid_etag(etag: &str) -> bool {
    !etag.is_empty() && etag.len() <= 256 && !etag.chars().any(char::is_control)
}

fn hmac(key: &[u8], value: &[u8]) -> ApiResult<Vec<u8>> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).map_err(|_| ApiFailure::internal())?;
    mac.update(value);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn uri_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn canonical_query(pairs: &[(String, String)]) -> String {
    let mut encoded = pairs
        .iter()
        .map(|(k, v)| (uri_encode(k), uri_encode(v)))
        .collect::<Vec<_>>();
    encoded.sort();
    encoded
        .into_iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&")
}

struct ParsedXml {
    root: String,
    fields: BTreeMap<String, String>,
    rows: Vec<BTreeMap<String, String>>,
}

/// Parse only the fixed S3 control envelope. Reject DTDs and bound XML depth.
/// Owner subtrees and unknown future fields are ignored, never interpreted.
fn parse_xml(body: &str, row_tag: Option<&str>) -> ApiResult<ParsedXml> {
    if body.len() > CONTROL_RESPONSE_LIMIT {
        return Err(unavailable());
    }
    let mut reader = Reader::from_str(body);
    let mut path = Vec::<String>::new();
    let mut parsed = ParsedXml {
        root: String::new(),
        fields: BTreeMap::new(),
        rows: Vec::new(),
    };
    let mut row = BTreeMap::<String, String>::new();
    loop {
        match reader.read_event().map_err(|_| unavailable())? {
            Event::Start(start) => {
                let name = String::from_utf8(start.local_name().as_ref().to_vec())
                    .map_err(|_| unavailable())?;
                if path.is_empty() {
                    if !parsed.root.is_empty() {
                        return Err(unavailable());
                    }
                    parsed.root = name.clone();
                }
                path.push(name);
                if path.len() > 8 {
                    return Err(unavailable());
                }
            }
            Event::Empty(start) => {
                let name = String::from_utf8(start.local_name().as_ref().to_vec())
                    .map_err(|_| unavailable())?;
                if path.is_empty() {
                    if !parsed.root.is_empty() {
                        return Err(unavailable());
                    }
                    parsed.root = name;
                } else if path.len() == 1 && Some(name.as_str()) == row_tag {
                    parsed.rows.push(BTreeMap::new());
                }
            }
            Event::Text(text) => {
                let decoded = text.decode().map_err(|_| unavailable())?;
                let value = quick_xml::escape::unescape(&decoded).map_err(|_| unavailable())?;
                if path.len() == 2 && Some(path[1].as_str()) != row_tag {
                    parsed
                        .fields
                        .entry(path[1].clone())
                        .or_default()
                        .push_str(&value);
                } else if path.len() == 3 && Some(path[1].as_str()) == row_tag {
                    row.entry(path[2].clone()).or_default().push_str(&value);
                }
            }
            Event::End(_) => {
                if path.len() == 2 && Some(path[1].as_str()) == row_tag {
                    parsed.rows.push(std::mem::take(&mut row));
                }
                path.pop().ok_or_else(unavailable)?;
            }
            Event::GeneralRef(reference) => {
                let decoded = reference.decode().map_err(|_| unavailable())?;
                let escaped = format!("&{decoded};");
                let value = quick_xml::escape::unescape(&escaped).map_err(|_| unavailable())?;
                if path.len() == 2 && Some(path[1].as_str()) != row_tag {
                    parsed
                        .fields
                        .entry(path[1].clone())
                        .or_default()
                        .push_str(&value);
                } else if path.len() == 3 && Some(path[1].as_str()) == row_tag {
                    row.entry(path[2].clone()).or_default().push_str(&value);
                }
            }
            Event::DocType(_) | Event::CData(_) => return Err(unavailable()),
            Event::Eof => break,
            _ => {}
        }
    }
    if parsed.root.is_empty() || !path.is_empty() {
        return Err(unavailable());
    }
    Ok(parsed)
}

fn field<'a>(fields: &'a BTreeMap<String, String>, key: &str) -> ApiResult<&'a str> {
    fields
        .get(key)
        .filter(|value| !value.is_empty())
        .map(String::as_str)
        .ok_or_else(unavailable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body, extract::State, http::Request, response::IntoResponse, routing::any, Router,
    };
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
    };

    const KEY: &str = "aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa/bbbbbbbb-bbbb-4bbb-bbbb-bbbbbbbbbbbb/cccccccc-cccc-4ccc-cccc-cccccccccccc/dddddddd-dddd-4ddd-dddd-dddddddddddd";
    const HASH: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    fn client() -> MultipartClient {
        MultipartClient {
            endpoint: Url::parse("https://storage.example.com/").unwrap(),
            bucket: "private-apks".into(),
            region: "us-east-1".into(),
            access_key: "AKIAIOSFODNN7EXAMPLE".into(),
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".into(),
            http: Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap(),
        }
    }

    #[test]
    fn signature_matches_published_aws_s3_query_vector() {
        // AWS SigV4 query authentication example, 2013-05-24. These are public
        // example credentials, not a project credential or a self-derived vector.
        let url = client()
            .presign(
                &Method::GET,
                Url::parse("https://examplebucket.s3.amazonaws.com/test.txt").unwrap(),
                &BTreeMap::new(),
                "UNSIGNED-PAYLOAD",
                DateTime::parse_from_rfc3339("2013-05-24T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                86400,
            )
            .unwrap();
        assert_eq!(
            url.query_pairs()
                .find(|(key, _)| key == "X-Amz-Signature")
                .unwrap()
                .1,
            "aeeed9bbccd4d02ee5c0109b86d86835f995330da4c265957d157751f604d404"
        );
    }

    #[test]
    fn part_capability_binds_payload_length_and_exact_part() {
        let store = client();
        let part = store
            .part_url(KEY, "provider+/= id", 2, 1024, HASH)
            .unwrap();
        let url = Url::parse(&part.url).unwrap();
        let query = url.query_pairs().collect::<BTreeMap<_, _>>();
        assert_eq!(query["partNumber"], "2");
        assert_eq!(query["uploadId"], "provider+/= id");
        assert_eq!(
            query["X-Amz-SignedHeaders"],
            "content-length;host;x-amz-content-sha256"
        );
        assert_eq!(query["X-Amz-Expires"], "600");
        assert_eq!(part.headers["x-amz-content-sha256"], HASH);
        assert!(!part.headers.contains_key("content-length"));
        assert!(!part.url.contains(&store.secret_key));
        assert_eq!(url.path(), format!("/private-apks/{KEY}"));
        let changed = store
            .part_url(KEY, "provider+/= id", 2, 1025, HASH)
            .unwrap();
        assert_ne!(part.url, changed.url);
        assert!(store.part_url(KEY, "id", 0, 1, HASH).is_err());
        assert!(store.part_url(KEY, "id", MAX_PARTS + 1, 1, HASH).is_err());
        assert!(store.part_url(KEY, "id", 1, PART_BYTES + 1, HASH).is_err());
        assert!(store.part_url(KEY, "id", 1, 1, "not-a-hash").is_err());
        assert!(store.part_url("../../private", "id", 1, 1, HASH).is_err());
        assert!(store.part_url(KEY, "id\r\n", 1, 1, HASH).is_err());
    }

    #[test]
    fn download_capability_has_no_upload_authority() {
        let signed = client().download_url(KEY).unwrap();
        let url = Url::parse(&signed.url).unwrap();
        let params = url.query_pairs().collect::<BTreeMap<_, _>>();
        assert_eq!(params["X-Amz-SignedHeaders"], "host");
        assert!(!params.contains_key("uploadId"));
        assert!(signed.headers.is_empty());
        assert!(client().download_url("../../secret").is_err());
    }

    #[test]
    fn xml_rejects_external_entities_and_handles_opaque_etags() {
        let parsed = parse_xml("<ListPartsResult><IsTruncated>false</IsTruncated><Part><PartNumber>1</PartNumber><ETag>&quot;opaque&amp;etag&quot;</ETag><Size>5</Size><Owner><ID>ignored</ID></Owner></Part></ListPartsResult>", Some("Part")).unwrap();
        assert_eq!(parsed.rows[0]["ETag"], "\"opaque&etag\"");
        assert_eq!(parsed.rows[0]["PartNumber"], "1");
        assert!(!parsed.rows[0].contains_key("ID"));
        assert!(parse_xml(
            "<!DOCTYPE x [<!ENTITY x SYSTEM 'file:///etc/passwd'>]><x>&x;</x>",
            None
        )
        .is_err());
        assert!(parse_xml("<x><y></x>", None).is_err());
        assert!(parse_xml("<x></x><y></y>", None).is_err());
    }

    #[test]
    fn query_encoding_preserves_reserved_provider_ids() {
        assert_eq!(
            canonical_query(&[("z".into(), "a b+/%=".into()), ("a".into(), "~".into())]),
            "a=~&z=a%20b%2B%2F%25%3D"
        );
    }

    struct RecordedRequest {
        method: Method,
        uri: String,
        body: String,
        digest: String,
    }

    #[derive(Clone)]
    struct FixtureState {
        replies: Arc<Mutex<VecDeque<(StatusCode, String)>>>,
        requests: Arc<Mutex<Vec<RecordedRequest>>>,
    }

    struct Fixture {
        client: MultipartClient,
        requests: Arc<Mutex<Vec<RecordedRequest>>>,
        server: tokio::task::JoinHandle<()>,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            self.server.abort();
        }
    }

    async fn receive(
        State(state): State<FixtureState>,
        request: Request<Body>,
    ) -> impl IntoResponse {
        let (parts, body) = request.into_parts();
        let bytes = axum::body::to_bytes(body, CONTROL_RESPONSE_LIMIT)
            .await
            .unwrap();
        state.requests.lock().unwrap().push(RecordedRequest {
            method: parts.method,
            uri: parts.uri.to_string(),
            body: String::from_utf8(bytes.to_vec()).unwrap(),
            digest: parts.headers["x-amz-content-sha256"]
                .to_str()
                .unwrap()
                .into(),
        });
        let mut replies = state.replies.lock().unwrap();
        replies.pop_front().expect("fixture request count")
    }

    async fn fixture(replies: Vec<(StatusCode, &str)>) -> Fixture {
        let state = FixtureState {
            replies: Arc::new(Mutex::new(
                replies
                    .into_iter()
                    .map(|(status, text)| (status, text.into()))
                    .collect(),
            )),
            requests: Arc::new(Mutex::new(Vec::new())),
        };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut client = client();
        // Loopback HTTP exists only in this test constructor. Production accepts
        // HTTPS origins exclusively and does not read any test override flag.
        client.endpoint =
            Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
        let router = Router::new()
            .fallback(any(receive))
            .with_state(state.clone());
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        Fixture {
            client,
            requests: state.requests,
            server,
        }
    }

    #[tokio::test]
    async fn multipart_control_lifecycle_uses_bounded_fake_provider() {
        let fixture = fixture(vec![
            (StatusCode::OK, "<InitiateMultipartUploadResult><UploadId>provider+/=</UploadId></InitiateMultipartUploadResult>"),
            (StatusCode::OK, "<ListPartsResult><IsTruncated>false</IsTruncated><Part><PartNumber>1</PartNumber><ETag>&quot;opaque&quot;</ETag><Size>5</Size></Part></ListPartsResult>"),
            (StatusCode::OK, "<CompleteMultipartUploadResult><ETag>opaque</ETag></CompleteMultipartUploadResult>"),
            (StatusCode::NOT_FOUND, "<Error><Code>NoSuchUpload</Code></Error>"),
        ]).await;
        let started = fixture.client.start(KEY).await.unwrap();
        assert_eq!(started.upload_id, "provider+/=");
        let parts = fixture.client.parts(KEY, &started.upload_id).await.unwrap();
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].bytes, 5);
        fixture
            .client
            .complete(
                KEY,
                &started.upload_id,
                &[MultipartPart {
                    part_number: 1,
                    etag: parts[0].etag.clone(),
                }],
            )
            .await
            .unwrap();
        fixture.client.abort(KEY, &started.upload_id).await.unwrap();
        let requests = fixture.requests.lock().unwrap();
        assert_eq!(requests.len(), 4);
        assert_eq!(requests[0].method, Method::POST);
        assert!(requests[0].uri.contains("uploads="));
        assert!(requests[1].uri.contains("uploadId=provider%2B%2F%3D"));
        assert!(requests[2].body.contains("<ETag>&quot;opaque&quot;</ETag>"));
        for request in requests.iter() {
            assert_eq!(
                request.digest,
                hex::encode(Sha256::digest(request.body.as_bytes()))
            );
        }
    }

    #[tokio::test]
    async fn completion_http_200_error_is_never_success() {
        let fixture = fixture(vec![(
            StatusCode::OK,
            "<Error><Code>InvalidPart</Code></Error>",
        )])
        .await;
        assert!(fixture
            .client
            .complete(
                KEY,
                "id",
                &[MultipartPart {
                    part_number: 1,
                    etag: "etag".into()
                }]
            )
            .await
            .is_err());
        assert!(fixture
            .client
            .complete(
                KEY,
                "id",
                &[MultipartPart {
                    part_number: 2,
                    etag: "etag".into()
                }]
            )
            .await
            .is_err());
        assert_eq!(fixture.requests.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn truncated_parts_and_redirects_fail_closed() {
        let fixture = fixture(vec![
            (
                StatusCode::OK,
                "<ListPartsResult><IsTruncated>true</IsTruncated></ListPartsResult>",
            ),
            (
                StatusCode::TEMPORARY_REDIRECT,
                "<Error><Code>TemporaryRedirect</Code></Error>",
            ),
        ])
        .await;
        assert!(fixture.client.parts(KEY, "id").await.is_err());
        assert!(fixture.client.start(KEY).await.is_err());
        assert_eq!(fixture.requests.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn orphan_multipart_listing_is_scoped_and_paginated() {
        let prefix = KEY.rsplit_once('/').unwrap().0;
        let first = format!("<ListMultipartUploadsResult><IsTruncated>true</IsTruncated><NextKeyMarker>{KEY}</NextKeyMarker><NextUploadIdMarker>one</NextUploadIdMarker><Upload><Key>{KEY}</Key><UploadId>one</UploadId><Initiated>2026-09-22T10:00:00Z</Initiated></Upload></ListMultipartUploadsResult>");
        let second = format!("<ListMultipartUploadsResult><IsTruncated>false</IsTruncated><Upload><Key>{KEY}</Key><UploadId>two</UploadId><Initiated>2026-09-22T10:00:01Z</Initiated></Upload></ListMultipartUploadsResult>");
        let fixture = fixture(vec![(StatusCode::OK, &first), (StatusCode::OK, &second)]).await;
        let uploads = fixture.client.uploads(prefix).await.unwrap();
        assert_eq!(uploads.len(), 2);
        assert_eq!(uploads[0].key, KEY);
        assert_eq!(uploads[1].upload_id, "two");
        assert_eq!(
            uploads[0].created_at.to_rfc3339(),
            "2026-09-22T10:00:00+00:00"
        );
        assert!(fixture.requests.lock().unwrap()[1]
            .uri
            .contains("upload-id-marker=one"));
    }
}
