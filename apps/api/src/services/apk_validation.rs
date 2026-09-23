//! Static intake validation only. A validated APK is not evidence of device installation.
use crate::config::Setup;
use mobile_qa_contracts::browser::{ApkMetadata, ValidationState};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::{io::AsyncReadExt, process::Command};

pub const POLICY: &str = "candidate-api35-x86_64-large-v2";
pub const TOOL_VERSION: &str = "android-build-tools-36.0.0/intake-v2";
#[derive(Debug)]
pub struct Inspection {
    pub state: ValidationState,
    pub code: Option<String>,
    pub message: Option<String>,
    pub metadata: Option<Box<ApkMetadata>>,
}
impl Inspection {
    pub fn fail(state: ValidationState, code: &str, message: &str) -> Self {
        Self {
            state,
            code: Some(code.into()),
            message: Some(message.into()),
            metadata: None,
        }
    }
    pub fn infrastructure(code: &str, message: &str) -> Self {
        Self::fail(ValidationState::Error, code, message)
    }
}
fn invalid(message: &str) -> Inspection {
    Inspection::fail(ValidationState::Invalid, "invalid_apk", message)
}
/// Bound and inspect the central directory before ZipArchive allocates its entry index.
/// APKs are capped below 4 GiB, so ZIP64/multidisk archives are deliberately rejected.
fn directory(file: &mut std::fs::File, size: u64) -> Result<(), Inspection> {
    let tail_len = size.min(65557) as usize;
    let mut tail = vec![0; tail_len];
    file.seek(SeekFrom::End(-(tail_len as i64)))
        .map_err(|_| invalid("APK archive cannot be read"))?;
    file.read_exact(&mut tail)
        .map_err(|_| invalid("APK archive is truncated"))?;
    let Some(pos) = tail.windows(4).rposition(|v| v == b"PK\x05\x06") else {
        return Err(invalid("APK archive directory is missing"));
    };
    if pos + 22 > tail.len() {
        return Err(invalid("APK archive directory is truncated"));
    }
    let u16at = |i| u16::from_le_bytes([tail[pos + i], tail[pos + i + 1]]);
    let u32at = |i| u32::from_le_bytes(tail[pos + i..pos + i + 4].try_into().expect("bounded"));
    let entries = u16at(10);
    let bytes = u32at(12) as u64;
    let offset = u32at(16) as u64;
    if u16at(4) != 0
        || u16at(6) != 0
        || u16at(8) != entries
        || entries == u16::MAX
        || bytes > 32 * 1024 * 1024
        || offset + bytes > size - tail_len as u64 + pos as u64
        || pos + 22 + u16at(20) as usize != tail.len()
    {
        return Err(invalid("APK directory exceeds supported archive limits"));
    }
    file.seek(SeekFrom::Start(offset))
        .map_err(|_| invalid("APK directory is invalid"))?;
    let mut consumed = 0u64;
    let mut names = HashSet::new();
    for _ in 0..entries {
        let mut header = [0; 46];
        file.read_exact(&mut header)
            .map_err(|_| invalid("APK directory is truncated"))?;
        if &header[..4] != b"PK\x01\x02" {
            return Err(invalid("APK directory entry is invalid"));
        }
        let read16 = |i| u16::from_le_bytes([header[i], header[i + 1]]);
        let read32 = |i| {
            u32::from_le_bytes(
                header[i..i + 4]
                    .try_into()
                    .expect("bounded directory field"),
            )
        };
        if [20, 24, 42]
            .into_iter()
            .any(|offset| read32(offset) == u32::MAX)
        {
            return Err(invalid("ZIP64 APK entries are not supported"));
        }
        let name_len = read16(28) as usize;
        let extra = read16(30) as u64 + read16(32) as u64;
        if name_len == 0 || name_len > 4096 || read16(8) & 1 != 0 || read16(34) != 0 {
            return Err(invalid("APK contains oversized or encrypted entries"));
        }
        let mut name = vec![0; name_len];
        file.read_exact(&mut name)
            .map_err(|_| invalid("APK entry is truncated"))?;
        if !names.insert(name) {
            return Err(invalid("APK contains duplicate entries"));
        }
        consumed += 46 + name_len as u64 + extra;
        if consumed > bytes {
            return Err(invalid("APK directory bounds are invalid"));
        }
        file.seek(SeekFrom::Current(extra as i64))
            .map_err(|_| invalid("APK directory is truncated"))?;
    }
    if consumed != bytes {
        return Err(invalid("APK directory count is inconsistent"));
    }
    Ok(())
}
/// CRC is verified by consuming each bounded entry. No archive content is extracted.
fn archive(
    path: &Path,
    size: i64,
    hash: &str,
    deadline: Instant,
) -> Result<Vec<String>, Inspection> {
    let mut file = std::fs::File::open(path).map_err(|_| {
        Inspection::infrastructure("artifact_unavailable", "Stored APK cannot be read")
    })?;
    let mut digest = Sha256::new();
    let mut count = 0i64;
    let mut buf = [0u8; 65536];
    loop {
        if Instant::now() > deadline {
            return Err(Inspection::infrastructure(
                "validation_timeout",
                "Validation timed out. Retry validation",
            ));
        }
        let n = file
            .read(&mut buf)
            .map_err(|_| invalid("APK is truncated"))?;
        if n == 0 {
            break;
        }
        count += n as i64;
        if count > crate::config::MAX_APK {
            return Err(invalid("APK exceeds the size limit"));
        }
        digest.update(&buf[..n]);
    }
    if count != size || format!("{:x}", digest.finalize()) != hash {
        return Err(Inspection::infrastructure(
            "artifact_changed",
            "Stored APK checksum differs from the uploaded build",
        ));
    }
    directory(&mut file, count as u64)?;
    let file = std::fs::File::open(path).map_err(|_| invalid("APK cannot be read"))?;
    let mut zip =
        zip::ZipArchive::new(file).map_err(|_| invalid("File is not a readable APK archive"))?;
    if zip.len() > 100_000 || zip.is_empty() {
        return Err(invalid("APK archive has an unsupported number of entries"));
    }
    if zip
        .has_overlapping_files()
        .map_err(|_| invalid("APK archive is malformed"))?
    {
        return Err(invalid("APK archive has overlapping file entries"));
    }
    let mut names = HashSet::new();
    let mut abis = Vec::new();
    let mut total = 0u64;
    let mut manifest = false;
    for i in 0..zip.len() {
        if Instant::now() > deadline {
            return Err(Inspection::infrastructure(
                "validation_timeout",
                "Validation timed out. Retry validation",
            ));
        }
        let mut entry = zip
            .by_index(i)
            .map_err(|_| invalid("APK contains unreadable or encrypted entries"))?;
        let name = entry.name().to_string();
        if name.len() > 4096
            || entry.enclosed_name().is_none()
            || name.contains('\\')
            || !names.insert(name.clone())
            || entry.encrypted()
        {
            return Err(invalid(
                "APK contains unsafe, duplicate or encrypted entries",
            ));
        }
        if entry.size() > u32::MAX as u64 || entry.size() > entry.compressed_size().max(1) * 1000 {
            return Err(invalid("APK archive exceeds expansion limits"));
        }
        if !matches!(
            entry.compression(),
            zip::CompressionMethod::Stored | zip::CompressionMethod::Deflated
        ) {
            return Err(invalid("APK uses unsupported compression"));
        }
        if name == "AndroidManifest.xml" {
            manifest = true;
        }
        let segments: Vec<_> = name.split('/').collect();
        if segments.len() == 3 && segments[0] == "lib" && segments[2].ends_with(".so") {
            abis.push(segments[1].to_owned());
        }
        loop {
            if Instant::now() > deadline {
                return Err(Inspection::infrastructure(
                    "validation_timeout",
                    "Validation timed out. Retry validation",
                ));
            }
            let n = entry
                .read(&mut buf)
                .map_err(|_| invalid("APK entry is corrupt or truncated"))?;
            if n == 0 {
                break;
            }
            total += n as u64;
            if total > 8 * 1024 * 1024 * 1024 {
                return Err(invalid("APK expanded content exceeds the limit"));
            }
        }
    }
    if !manifest {
        return Err(invalid("APK is missing AndroidManifest.xml"));
    }
    abis.sort();
    abis.dedup();
    Ok(abis)
}
async fn output(reader: impl tokio::io::AsyncRead + Unpin) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(1024 * 1024 + 1).read_to_end(&mut bytes).await?;
    if bytes.len() > 1024 * 1024 {
        return Err(std::io::Error::other("tool output limit"));
    }
    Ok(bytes)
}
async fn tool(
    executable: PathBuf,
    args: Vec<String>,
    deadline: tokio::time::Instant,
) -> Result<(bool, String), Inspection> {
    let mut command = Command::new(executable);
    command
        .args(args)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    // Only the Java runtime location is forwarded, never the API's credential environment.
    if let Some(java) = std::env::var_os("JAVA_HOME") {
        command.env("JAVA_HOME", java);
    }
    let mut child = command.spawn().map_err(|_| {
        Inspection::infrastructure(
            "validator_unavailable",
            "Android validation tools are unavailable. Contact your operator",
        )
    })?;
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let work = async {
        let (out, err) = tokio::try_join!(output(stdout), output(stderr))?;
        let status = child.wait().await?;
        Ok::<_, std::io::Error>((status.success(), out, err))
    };
    match tokio::time::timeout_at(deadline, work).await {
        Ok(Ok((success, out, _))) => Ok((
            success,
            String::from_utf8(out).map_err(|_| invalid("APK metadata contains invalid text"))?,
        )),
        _ => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            Err(Inspection::infrastructure(
                "validation_timeout",
                "Validator timed out or exceeded its output limit. Retry validation",
            ))
        }
    }
}
fn attr(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}='");
    let start = line.find(&prefix)? + prefix.len();
    let tail = &line[start..];
    let end = tail.find('\'')?;
    Some(tail[..end].into())
}
pub fn metadata(badging: &str, tree: &str, abis: Vec<String>) -> Result<ApkMetadata, Inspection> {
    if tree.contains(" split(")
        || tree.contains("A: split=")
        || tree.lines().any(|line| {
            line.contains("isSplitRequired")
                && (line.contains("=true") || line.contains("=0xffffffff"))
        })
        || tree.contains("E: uses-split")
    {
        return Err(Inspection::fail(
            ValidationState::Unsupported,
            "split_apk",
            "Upload a standalone APK; split APK sets are not supported",
        ));
    }
    let package = badging
        .lines()
        .find(|l| l.starts_with("package:"))
        .ok_or_else(|| invalid("APK package metadata is missing"))?;
    if attr(package, "split").is_some() {
        return Err(Inspection::fail(
            ValidationState::Unsupported,
            "split_apk",
            "Upload a standalone APK, not a split APK",
        ));
    }
    let package_name = attr(package, "name").ok_or_else(|| invalid("APK package is missing"))?;
    let low = attr(package, "versionCode")
        .and_then(|v| v.parse::<u32>().ok())
        .ok_or_else(|| invalid("APK version code is invalid"))?;
    let major = attr(package, "versionCodeMajor")
        .map(|v| v.parse::<u32>())
        .transpose()
        .map_err(|_| invalid("APK version code is invalid"))?
        .unwrap_or(0);
    let sdk = |prefix: &str| {
        badging
            .lines()
            .find_map(|l| l.strip_prefix(prefix))
            .map(|s| {
                s.trim_matches('\'').parse::<u32>().map_err(|_| {
                    Inspection::fail(
                        ValidationState::Unsupported,
                        "preview_sdk",
                        "Use a released numeric Android SDK version",
                    )
                })
            })
            .transpose()
    };
    Ok(ApkMetadata {
        package_name,
        version_name: attr(package, "versionName").filter(|v| !v.is_empty()),
        version_code: (((major as u64) << 32) | low as u64).to_string(),
        min_sdk: sdk("minSdkVersion:")?.or(sdk("sdkVersion:")?).unwrap_or(1),
        target_sdk: sdk("targetSdkVersion:")?,
        native_abis: abis,
        signature_verified: false,
    })
}
pub async fn inspect(
    setup: &Setup,
    path: &Path,
    size: i64,
    hash: &str,
    expected_package: &str,
) -> Inspection {
    let deadline = Instant::now() + Duration::from_secs(1200);
    let async_deadline = tokio::time::Instant::now() + Duration::from_secs(1200);
    let owned = path.to_owned();
    let expected_hash = hash.to_owned();
    // Keep a separate CPU permit inside the blocking work even if HTTP is cancelled.
    let permit = match setup.archives.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            return Inspection::infrastructure(
                "validator_busy",
                "Archive validation is busy. Retry shortly",
            )
        }
    };
    let abis = match tokio::task::spawn_blocking(move || {
        let _permit = permit;
        archive(&owned, size, &expected_hash, deadline)
    })
    .await
    {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => return e,
        Err(_) => {
            return Inspection::infrastructure(
                "validator_failed",
                "Validation could not finish. Retry validation",
            )
        }
    };
    // A broken Java launcher is infrastructure failure, not an invalid APK signature.
    let java_ready = std::env::var_os("JAVA_HOME")
        .map(PathBuf::from)
        .is_some_and(|home| {
            home.join("bin/java").is_file()
                && std::fs::read_to_string(home.join("release")).is_ok_and(|text| {
                    text.lines()
                        .any(|line| line.starts_with("JAVA_VERSION=\"17."))
                })
        });
    if !java_ready {
        return Inspection::infrastructure(
            "java_unavailable",
            "Configure JDK 17 for Android validation. Contact your operator",
        );
    }
    let tools = setup.sdk.join("build-tools/36.0.0");
    let file = path.to_string_lossy().into_owned();
    let badging = match tool(
        tools.join("aapt2"),
        vec!["dump".into(), "badging".into(), file.clone()],
        async_deadline,
    )
    .await
    {
        Ok((true, v)) => v,
        Ok(_) => return invalid("Android metadata is corrupt or unsupported"),
        Err(e) => return e,
    };
    let tree = match tool(
        tools.join("aapt2"),
        vec![
            "dump".into(),
            "xmltree".into(),
            file.clone(),
            "--file".into(),
            "AndroidManifest.xml".into(),
        ],
        async_deadline,
    )
    .await
    {
        Ok((true, v)) => v,
        Ok(_) => return invalid("Android manifest cannot be decoded"),
        Err(e) => return e,
    };
    let mut data = match metadata(&badging, &tree, abis) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if data.package_name != expected_package {
        return Inspection {
            metadata: Some(Box::new(data)),
            ..Inspection::fail(
                ValidationState::Invalid,
                "package_mismatch",
                "APK package does not match this app. Upload the correct APK",
            )
        };
    }
    match tool(
        tools.join("apksigner"),
        vec!["verify".into(), "--verbose".into(), file],
        async_deadline,
    )
    .await
    {
        Ok((true, _)) => data.signature_verified = true,
        Ok(_) => {
            return Inspection {
                metadata: Some(Box::new(data)),
                ..Inspection::fail(
                    ValidationState::Invalid,
                    "invalid_signature",
                    "APK signature is missing or invalid. Upload a signed build",
                )
            }
        }
        Err(e) => return e,
    };
    if data.min_sdk > 35
        || (!data.native_abis.is_empty() && !data.native_abis.iter().any(|a| a == "x86_64"))
    {
        return Inspection{metadata:Some(Box::new(data)),..Inspection::fail(ValidationState::Unsupported,"unsupported_device_policy","Build for Android API 35 or lower with x86_64 support, or provide an ABI-neutral APK")};
    }
    Inspection {
        state: ValidationState::Validated,
        code: None,
        message: None,
        metadata: Some(Box::new(data)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    fn zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, data) in entries {
            writer
                .start_file(
                    *name,
                    zip::write::SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Stored),
                )
                .unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }
    fn inspect_archive(bytes: &[u8]) -> Result<Vec<String>, Inspection> {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(bytes).unwrap();
        archive(
            file.path(),
            bytes.len() as i64,
            &format!("{:x}", Sha256::digest(bytes)),
            Instant::now() + Duration::from_secs(2),
        )
    }
    #[test]
    fn rejects_hostile_archive_names_and_missing_manifest() {
        assert!(inspect_archive(&zip_bytes(&[
            ("../escape", b"test"),
            ("AndroidManifest.xml", b"synthetic")
        ]))
        .is_err());
        assert!(inspect_archive(&zip_bytes(&[("readme", b"test")])).is_err());
    }
    #[test]
    fn archive_crc_integrity_and_abi_inventory() {
        let mut bytes = zip_bytes(&[
            ("AndroidManifest.xml", b"synthetic-manifest"),
            ("lib/arm64-v8a/test.so", b"synthetic-not-executable"),
        ]);
        assert_eq!(inspect_archive(&bytes).unwrap(), vec!["arm64-v8a"]);
        let pos = bytes
            .windows(b"synthetic-manifest".len())
            .position(|v| v == b"synthetic-manifest")
            .unwrap();
        bytes[pos] ^= 1;
        assert!(inspect_archive(&bytes).is_err());
    }
    #[test]
    fn checksum_mismatch_is_infrastructure_not_invalid_apk() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"bytes").unwrap();
        let failure = archive(
            file.path(),
            5,
            "wrong",
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap_err();
        assert_eq!(failure.state, ValidationState::Error);
        assert_eq!(failure.code.as_deref(), Some("artifact_changed"));
    }
    #[tokio::test]
    async fn process_timeout_is_reaped_and_never_validated() {
        let failure = tool(
            PathBuf::from("/bin/sleep"),
            vec!["10".into()],
            tokio::time::Instant::now() + Duration::from_millis(20),
        )
        .await
        .unwrap_err();
        assert_eq!(failure.state, ValidationState::Error);
    }
}
