//! Lease authorization belongs to the caller. This boundary exposes only the authorized immutable build.
use crate::{
    config::{Setup, MAX_APK},
    errors::{ApiFailure, ApiResult},
};
use axum::{body::Body, response::Response};
use chrono::{Duration, Utc};
use futures_util::StreamExt;
use mobile_qa_contracts::artifacts_api::{BuildDelivery, DeliveryAuthentication};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use uuid::Uuid;

pub fn require_active_attempt(state: &str) -> ApiResult<()> {
    if !matches!(
        state,
        "leased" | "running" | "cancel_requested" | "finalizing"
    ) {
        return Err(ApiFailure::new(
            409,
            "lease_inactive",
            "Build delivery requires an active execution lease",
        ));
    }
    Ok(())
}
pub fn grant(
    setup: &Setup,
    key: &str,
    backend: &str,
    app: Uuid,
    size: i64,
    sha256: String,
    local_path: String,
) -> ApiResult<BuildDelivery> {
    if !(1..=MAX_APK).contains(&size)
        || sha256.len() != 64
        || !sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ApiFailure::internal());
    }
    if (backend == "pilot") != setup.store.is_remote() {
        return Err(ApiFailure::new(
            503,
            "storage_backend_unavailable",
            "Build storage is not configured",
        ));
    }
    let (url, expires_at, headers, authentication) = if setup.store.is_remote() {
        let signed = setup.store.presign_download(key)?;
        (
            signed.url,
            signed.expires_at,
            signed.headers,
            DeliveryAuthentication::SignedUrl,
        )
    } else {
        (
            local_path,
            Utc::now() + Duration::minutes(10),
            BTreeMap::new(),
            DeliveryAuthentication::WorkerLease,
        )
    };
    Ok(BuildDelivery {
        url,
        expires_at,
        headers,
        authentication,
        app_id: app,
        byte_size: size as u32,
        sha256,
    })
}
/// Legacy/local download remains bounded and verifies bytes before starting the response.
/// The scratch guard lives with the body so a slow consumer cannot race scratch deletion.
pub async fn stream(
    setup: &Setup,
    key: &str,
    backend: &str,
    size: i64,
    expected_sha: &str,
) -> ApiResult<Response> {
    verified_stream(&setup.store, key, backend, size, expected_sha).await
}
async fn verified_stream(
    store: &crate::storage::ArtifactStore,
    key: &str,
    backend: &str,
    size: i64,
    expected_sha: &str,
) -> ApiResult<Response> {
    if !(1..=MAX_APK).contains(&size) {
        return Err(ApiFailure::internal());
    }
    let work = async {
        let scratch = store.materialize_sized(key, backend, size).await?;
        let mut file = tokio::fs::File::open(&scratch.0).await?;
        let mut hash = Sha256::new();
        let mut count = 0i64;
        let mut buffer = vec![0u8; 65536];
        loop {
            let n = file.read(&mut buffer).await?;
            if n == 0 {
                break;
            }
            count += n as i64;
            if count > size {
                return Err(ApiFailure::new(
                    409,
                    "artifact_changed",
                    "Stored APK differs from its manifest",
                ));
            }
            hash.update(&buffer[..n]);
        }
        if count != size || format!("{:x}", hash.finalize()) != expected_sha {
            return Err(ApiFailure::new(
                409,
                "artifact_changed",
                "Stored APK differs from its manifest",
            ));
        }
        file.seek(std::io::SeekFrom::Start(0)).await?;
        let chunks = tokio_util::io::ReaderStream::with_capacity(file, 65536).map(move |chunk| {
            let _keep_scratch = &scratch;
            chunk
        });
        let mut response = Response::new(Body::from_stream(chunks));
        response.headers_mut().insert(
            "content-type",
            "application/vnd.android.package-archive"
                .parse()
                .expect("literal"),
        );
        response.headers_mut().insert(
            "content-length",
            size.to_string()
                .parse()
                .map_err(|_| ApiFailure::internal())?,
        );
        response.headers_mut().insert(
            "cache-control",
            "private, no-store".parse().expect("literal"),
        );
        Ok(response)
    };
    tokio::time::timeout(std::time::Duration::from_secs(1800), work)
        .await
        .map_err(|_| {
            ApiFailure::new(
                503,
                "artifact_timeout",
                "Build retrieval timed out; retry delivery",
            )
        })?
}
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt;
    #[tokio::test]
    async fn streams_verified_bytes_and_retains_scratch_until_body_drop() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::storage::ArtifactStore::new(dir.path().to_path_buf(), false).unwrap();
        let bytes = vec![7u8; 3 * 65536];
        let (scratch, mut file) = store.scratch_with_budget(bytes.len() as i64).await.unwrap();
        file.write_all(&bytes).await.unwrap();
        file.flush().await.unwrap();
        drop(file);
        let key = (0..4)
            .map(|_| Uuid::new_v4().to_string())
            .collect::<Vec<_>>()
            .join("/");
        store.publish(&key, &scratch.0).await.unwrap();
        let expected = format!("{:x}", Sha256::digest(&bytes));
        let response = verified_stream(&store, &key, "local", bytes.len() as i64, &expected)
            .await
            .unwrap();
        assert_eq!(
            std::fs::read_dir(dir.path().join("scratch"))
                .unwrap()
                .count(),
            2
        );
        let received = axum::body::to_bytes(response.into_body(), bytes.len())
            .await
            .unwrap();
        assert_eq!(received.as_ref(), bytes);
        assert_eq!(
            std::fs::read_dir(dir.path().join("scratch"))
                .unwrap()
                .count(),
            1
        );
        assert!(
            verified_stream(&store, &key, "local", bytes.len() as i64, &"0".repeat(64))
                .await
                .is_err()
        );
        assert!(
            verified_stream(&store, &key, "local", bytes.len() as i64 + 1, &expected)
                .await
                .is_err()
        );
    }
}
