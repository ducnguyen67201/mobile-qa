//! Durable APK storage. S3 objects and local files share immutable attempt keys.
//! Scratch is disposable and never exposed through static routes or public DTOs.
pub mod multipart;
use crate::{
    config::MAX_APK,
    errors::{ApiFailure, ApiResult},
};
use futures_util::StreamExt;
use loco_rs::storage::{drivers::aws, stream::BytesStream, Storage};
use std::{
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::io::AsyncWriteExt;

pub struct ArtifactStore {
    pub root: PathBuf,
    remote: Option<Storage>,
    multipart: Option<multipart::MultipartClient>,
    reserved_scratch: Arc<AtomicU64>,
}
// The second field is retained solely for its Drop guard, through the final reader.
pub struct Scratch(pub PathBuf, #[allow(dead_code)] ScratchReservation);
struct ScratchReservation {
    bytes: u64,
    reserved: Arc<AtomicU64>,
}
impl Drop for ScratchReservation {
    fn drop(&mut self) {
        self.reserved.fetch_sub(self.bytes, Ordering::AcqRel);
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
impl ArtifactStore {
    pub fn new(root: PathBuf, remote: bool) -> loco_rs::Result<Self> {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&root)?;
        if std::fs::symlink_metadata(&root)?.file_type().is_symlink() {
            return Err(loco_rs::Error::string(
                "Artifact root must not be a symlink",
            ));
        }
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))?;
        let root = root.canonicalize()?;
        let storage = if remote {
            let var = |name| {
                std::env::var(name)
                    .map_err(|_| loco_rs::Error::string(&format!("{name} is required")))
            };
            let endpoint = var("ARTIFACT_S3_ENDPOINT")?;
            if !endpoint.starts_with("https://") {
                return Err(loco_rs::Error::string("S3 endpoint must use HTTPS"));
            }
            Some(Storage::single(
                aws::with_credentials_and_endpoint(
                    &var("ARTIFACT_S3_BUCKET")?,
                    &var("ARTIFACT_S3_REGION")?,
                    &endpoint,
                    aws::Credential {
                        key_id: var("ARTIFACT_S3_ACCESS_KEY_ID")?,
                        secret_key: var("ARTIFACT_S3_SECRET_ACCESS_KEY")?,
                        token: None,
                    },
                )
                .map_err(|_| loco_rs::Error::string("Cannot configure private object storage"))?,
            ))
        } else {
            None
        };
        Ok(Self {
            root,
            remote: storage,
            multipart: if remote {
                Some(multipart::MultipartClient::from_env()?)
            } else {
                None
            },
            reserved_scratch: Arc::new(AtomicU64::new(0)),
        })
    }
    pub fn is_remote(&self) -> bool {
        self.remote.is_some()
    }
    fn key_path(&self, key: &str) -> ApiResult<PathBuf> {
        // Keys contain only server UUID segments; never join user filenames or locators.
        if key.split('/').count() != 4 || !key.split('/').all(|s| uuid::Uuid::parse_str(s).is_ok())
        {
            return Err(ApiFailure::internal());
        }
        Ok(self.root.join("objects").join(key))
    }
    fn private_dir(&self, path: &Path) -> ApiResult<()> {
        let relative = path
            .strip_prefix(&self.root)
            .map_err(|_| ApiFailure::internal())?;
        let mut current = self.root.clone();
        for component in relative.components() {
            if !matches!(component, std::path::Component::Normal(_)) {
                return Err(ApiFailure::internal());
            }
            current.push(component);
            match std::fs::symlink_metadata(&current) {
                Ok(meta) if !meta.is_dir() || meta.file_type().is_symlink() => {
                    return Err(ApiFailure::internal())
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    match std::fs::DirBuilder::new().mode(0o700).create(&current) {
                        Ok(()) => {}
                        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                        Err(e) => return Err(e.into()),
                    }
                    let meta = std::fs::symlink_metadata(&current)?;
                    if !meta.is_dir() || meta.file_type().is_symlink() {
                        return Err(ApiFailure::internal());
                    }
                }
                Err(error) => return Err(error.into()),
            }
            std::fs::set_permissions(&current, std::fs::Permissions::from_mode(0o700))?;
        }
        Ok(())
    }
    pub async fn scratch(&self) -> ApiResult<(Scratch, tokio::fs::File)> {
        self.scratch_with_budget(MAX_APK).await
    }
    /// Reserve before IO. Outstanding reservations are conservatively added to
    /// current disk usage; the guard stays with the scratch through validation or
    /// HTTP streaming. Other host processes can still fill disk, so writes remain
    /// fallible and callers must preserve their retryable upload state.
    pub async fn scratch_with_budget(&self, bytes: i64) -> ApiResult<(Scratch, tokio::fs::File)> {
        if !(1..=MAX_APK).contains(&bytes) {
            return Err(ApiFailure::internal());
        }
        let available = fs2::available_space(&self.root)?;
        let reservation = reserve_scratch(self.reserved_scratch.clone(), bytes as u64, available)?;
        let dir = self.root.join("scratch");
        self.private_dir(&dir)?;
        let path = dir.join(uuid::Uuid::new_v4().to_string());
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)?;
        Ok((Scratch(path, reservation), tokio::fs::File::from_std(file)))
    }
    pub async fn publish(&self, key: &str, file: &Path) -> ApiResult<()> {
        let path = self.key_path(key)?;
        if let Some(remote) = &self.remote {
            let file = tokio::fs::File::open(file).await?;
            let stream = BytesStream::from_body_stream(
                tokio_util::io::ReaderStream::with_capacity(file, 64 * 1024),
            );
            remote
                .upload_stream(Path::new(key), stream)
                .await
                .map_err(|_| {
                    ApiFailure::new(
                        503,
                        "storage_unavailable",
                        "Could not publish the APK. Retry the upload",
                    )
                })?;
        } else {
            self.private_dir(path.parent().ok_or_else(ApiFailure::internal)?)?;
            // Hard-link publication cannot overwrite an existing attempt; scratch shares filesystem.
            tokio::fs::hard_link(file, &path).await?;
        }
        Ok(())
    }
    pub async fn materialize(&self, key: &str, backend: &str) -> ApiResult<Scratch> {
        self.materialize_inner(key, backend, None).await
    }
    pub async fn materialize_sized(
        &self,
        key: &str,
        backend: &str,
        expected_size: i64,
    ) -> ApiResult<Scratch> {
        if !(1..=MAX_APK).contains(&expected_size) {
            return Err(ApiFailure::internal());
        }
        self.materialize_inner(key, backend, Some(expected_size))
            .await
    }
    async fn materialize_inner(
        &self,
        key: &str,
        backend: &str,
        expected_size: Option<i64>,
    ) -> ApiResult<Scratch> {
        if (backend == "pilot") != self.is_remote() {
            return Err(ApiFailure::new(
                503,
                "storage_backend_unavailable",
                "This build's storage backend is not configured",
            ));
        }
        let path = self.key_path(key)?;
        let maximum = expected_size.unwrap_or(MAX_APK);
        let budget = if self.remote.is_none() {
            let meta = tokio::fs::symlink_metadata(&path).await?;
            if !meta.is_file() || meta.file_type().is_symlink() {
                return Err(ApiFailure::internal());
            }
            if meta.len() > maximum as u64
                || expected_size.is_some_and(|size| meta.len() != size as u64)
            {
                return Err(ApiFailure::new(
                    503,
                    "artifact_changed",
                    "Stored APK differs from its declared size",
                ));
            }
            (meta.len() as i64).max(1)
        } else {
            maximum
        };
        let (scratch, mut output) = self.scratch_with_budget(budget).await?;
        let mut stream = if let Some(remote) = &self.remote {
            remote.download_stream(Path::new(key)).await.map_err(|_| {
                ApiFailure::new(503, "artifact_unavailable", "Stored APK is unavailable")
            })?
        } else {
            BytesStream::from_body_stream(tokio_util::io::ReaderStream::with_capacity(
                tokio::fs::File::open(path).await?,
                64 * 1024,
            ))
        };
        let mut size = 0i64;
        while let Some(chunk) = tokio::time::timeout(Duration::from_secs(60), stream.next())
            .await
            .map_err(|_| {
                ApiFailure::new(
                    503,
                    "artifact_transfer_stalled",
                    "Stored APK transfer stalled",
                )
            })?
        {
            let chunk = chunk?;
            size += chunk.len() as i64;
            if size > maximum {
                return Err(ApiFailure::new(
                    503,
                    "artifact_changed",
                    "Stored APK exceeds its limit",
                ));
            }
            output.write_all(&chunk).await?;
        }
        if expected_size.is_some_and(|expected| size != expected) {
            return Err(ApiFailure::new(
                503,
                "artifact_changed",
                "Stored APK differs from its declared size",
            ));
        }
        output.flush().await?;
        drop(output);
        Ok(scratch)
    }
    fn multipart_client(&self) -> ApiResult<&multipart::MultipartClient> {
        self.multipart.as_ref().ok_or_else(|| {
            ApiFailure::new(
                409,
                "multipart_unavailable",
                "Use local streaming upload for this storage backend",
            )
        })
    }
    pub async fn multipart_start(&self, key: &str) -> ApiResult<multipart::MultipartStart> {
        self.multipart_client()?.start(key).await
    }
    pub fn multipart_part_url(
        &self,
        key: &str,
        upload_id: &str,
        part_number: u16,
        bytes: i64,
        sha256: &str,
    ) -> ApiResult<multipart::MultipartPartUrl> {
        self.multipart_client()?
            .part_url(key, upload_id, part_number, bytes, sha256)
    }
    pub fn presign_download(&self, key: &str) -> ApiResult<multipart::MultipartPartUrl> {
        self.multipart_client()?.download_url(key)
    }
    pub async fn multipart_complete(
        &self,
        key: &str,
        upload_id: &str,
        parts: &[multipart::MultipartPart],
    ) -> ApiResult<()> {
        self.multipart_client()?
            .complete(key, upload_id, parts)
            .await
    }
    pub async fn multipart_abort(&self, key: &str, upload_id: &str) -> ApiResult<()> {
        self.multipart_client()?.abort(key, upload_id).await
    }
    pub async fn multipart_parts(
        &self,
        key: &str,
        upload_id: &str,
    ) -> ApiResult<Vec<multipart::StoredPart>> {
        self.multipart_client()?.parts(key, upload_id).await
    }
    pub async fn multipart_uploads(
        &self,
        prefix: &str,
    ) -> ApiResult<Vec<multipart::MultipartUpload>> {
        self.multipart_client()?.uploads(prefix).await
    }
    pub async fn delete(&self, key: &str, backend: &str) -> ApiResult<()> {
        if (backend == "pilot") != self.is_remote() {
            return Err(ApiFailure::internal());
        }
        let path = self.key_path(key)?;
        if let Some(remote) = &self.remote {
            remote.delete(Path::new(key)).await.map_err(|_| {
                ApiFailure::new(
                    503,
                    "storage_unavailable",
                    "Cleanup could not reach object storage",
                )
            })?;
        } else if let Err(err) = tokio::fs::remove_file(path).await {
            if err.kind() != std::io::ErrorKind::NotFound {
                return Err(err.into());
            }
        }
        Ok(())
    }
    pub async fn list_upload(
        &self,
        prefix: &str,
        backend: &str,
    ) -> ApiResult<Vec<(String, Option<chrono::DateTime<chrono::Utc>>)>> {
        if prefix.split('/').count() != 3
            || !prefix.split('/').all(|p| uuid::Uuid::parse_str(p).is_ok())
            || (backend == "pilot") != self.is_remote()
        {
            return Err(ApiFailure::internal());
        }
        if let Some(remote) = &self.remote {
            let entries = remote
                .list(Path::new(&format!("{prefix}/")), false)
                .await
                .map_err(|_| {
                    ApiFailure::new(
                        503,
                        "storage_unavailable",
                        "Cleanup could not list object storage",
                    )
                })?;
            return Ok(entries
                .into_iter()
                .filter(|e| {
                    !e.is_dir
                        && e.path.starts_with(&format!("{prefix}/"))
                        && self.key_path(&e.path).is_ok()
                })
                .map(|e| (e.path, e.last_modified))
                .collect());
        }
        let dir = self.root.join("objects").join(prefix);
        let entries = match std::fs::read_dir(dir) {
            Ok(v) => v,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(e.into()),
        };
        let mut result = Vec::new();
        for entry in entries {
            let entry = entry?;
            let meta = entry.path().symlink_metadata()?;
            let key = format!("{prefix}/{}", entry.file_name().to_string_lossy());
            if meta.is_file() && !meta.file_type().is_symlink() && self.key_path(&key).is_ok() {
                result.push((key, meta.modified().ok().map(chrono::DateTime::from)));
            }
        }
        Ok(result)
    }
    pub fn cleanup_scratch(
        &self,
        cutoff: chrono::DateTime<chrono::Utc>,
        apply: bool,
    ) -> ApiResult<usize> {
        let entries = match std::fs::read_dir(self.root.join("scratch")) {
            Ok(v) => v,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(e) => return Err(e.into()),
        };
        let mut count = 0;
        for entry in entries {
            let entry = entry?;
            let meta = entry.path().symlink_metadata()?;
            if meta.is_file()
                && !meta.file_type().is_symlink()
                && uuid::Uuid::parse_str(&entry.file_name().to_string_lossy()).is_ok()
                && meta
                    .modified()
                    .ok()
                    .map(chrono::DateTime::<chrono::Utc>::from)
                    .is_some_and(|t| t < cutoff)
            {
                count += 1;
                if apply {
                    std::fs::remove_file(entry.path())?;
                }
            }
        }
        Ok(count)
    }
}

fn reserve_scratch(
    reserved: Arc<AtomicU64>,
    bytes: u64,
    available: u64,
) -> ApiResult<ScratchReservation> {
    const SYSTEM_RESERVE: u64 = 1024 * 1024 * 1024;
    let mut current = reserved.load(Ordering::Acquire);
    loop {
        let next = current
            .checked_add(bytes)
            .ok_or_else(ApiFailure::internal)?;
        if next.saturating_add(SYSTEM_RESERVE) > available {
            return Err(ApiFailure::new(
                503,
                "artifact_scratch_full",
                "Artifact processing is waiting for disk capacity",
            ));
        }
        match reserved.compare_exchange_weak(current, next, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return Ok(ScratchReservation { bytes, reserved }),
            Err(actual) => current = actual,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scratch_budget_is_atomic_and_released_on_drop() {
        let reserved = Arc::new(AtomicU64::new(0));
        let free = 1024 * 1024 * 1024 + 150;
        let first = reserve_scratch(reserved.clone(), 100, free).unwrap();
        assert!(reserve_scratch(reserved.clone(), 100, free).is_err());
        assert_eq!(reserved.load(Ordering::Acquire), 100);
        drop(first);
        assert_eq!(reserved.load(Ordering::Acquire), 0);
        assert!(reserve_scratch(reserved, 100, free).is_ok());
    }

    #[tokio::test]
    async fn local_storage_does_not_mint_remote_capabilities() {
        let dir = tempfile::tempdir().unwrap();
        let store = ArtifactStore::new(dir.path().join("private"), false).unwrap();
        assert!(store.multipart_start("untrusted/path").await.is_err());
        assert!(store.presign_download("untrusted/path").is_err());
    }

    #[tokio::test]
    async fn private_immutable_publication_and_symlink_rejection() {
        let dir = tempfile::tempdir().unwrap();
        let store = ArtifactStore::new(dir.path().join("private"), false).unwrap();
        let (scratch, mut file) = store.scratch().await.unwrap();
        file.write_all(b"private APK bytes").await.unwrap();
        file.sync_all().await.unwrap();
        drop(file);
        let ids: [uuid::Uuid; 4] = std::array::from_fn(|_| uuid::Uuid::new_v4());
        let key = ids
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("/");
        store.publish(&key, &scratch.0).await.unwrap();
        assert!(store.publish(&key, &scratch.0).await.is_err());
        let fetched = store.materialize(&key, "local").await.unwrap();
        assert_eq!(std::fs::read(&fetched.0).unwrap(), b"private APK bytes");
        let sized = store.materialize_sized(&key, "local", 17).await.unwrap();
        assert_eq!(std::fs::read(&sized.0).unwrap(), b"private APK bytes");
        assert!(store.materialize_sized(&key, "local", 16).await.is_err());
        assert_eq!(
            std::fs::metadata(store.key_path(&key).unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        let poisoned = uuid::Uuid::new_v4();
        std::os::unix::fs::symlink(
            dir.path(),
            store.root.join("objects").join(poisoned.to_string()),
        )
        .unwrap();
        let bad = format!("{}/{}/{}/{}", poisoned, ids[1], ids[2], ids[3]);
        assert!(store.publish(&bad, &scratch.0).await.is_err());
    }
}
