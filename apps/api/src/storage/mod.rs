//! Durable APK storage. S3 objects and local files share immutable attempt keys.
//! Scratch is disposable and never exposed through static routes or public DTOs.
use crate::{
    config::MAX_APK,
    errors::{ApiFailure, ApiResult},
};
use futures_util::StreamExt;
use loco_rs::storage::{drivers::aws, stream::BytesStream, Storage};
use std::{
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
use tokio::io::AsyncWriteExt;

pub struct ArtifactStore {
    pub root: PathBuf,
    remote: Option<Storage>,
}
pub struct Scratch(pub PathBuf);
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
        let dir = self.root.join("scratch");
        self.private_dir(&dir)?;
        let path = dir.join(uuid::Uuid::new_v4().to_string());
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)?;
        Ok((Scratch(path), tokio::fs::File::from_std(file)))
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
        if (backend == "pilot") != self.is_remote() {
            return Err(ApiFailure::new(
                503,
                "storage_backend_unavailable",
                "This build's storage backend is not configured",
            ));
        }
        let path = self.key_path(key)?;
        let (scratch, mut output) = self.scratch().await?;
        let mut stream = if let Some(remote) = &self.remote {
            remote.download_stream(Path::new(key)).await.map_err(|_| {
                ApiFailure::new(503, "artifact_unavailable", "Stored APK is unavailable")
            })?
        } else {
            let meta = tokio::fs::symlink_metadata(&path).await?;
            if !meta.is_file() || meta.file_type().is_symlink() {
                return Err(ApiFailure::internal());
            }
            BytesStream::from_body_stream(tokio_util::io::ReaderStream::with_capacity(
                tokio::fs::File::open(path).await?,
                64 * 1024,
            ))
        };
        let mut size = 0i64;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            size += chunk.len() as i64;
            if size > MAX_APK {
                return Err(ApiFailure::new(
                    503,
                    "artifact_changed",
                    "Stored APK exceeds its limit",
                ));
            }
            output.write_all(&chunk).await?;
        }
        output.flush().await?;
        drop(output);
        Ok(scratch)
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

#[cfg(test)]
mod tests {
    use super::*;
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
