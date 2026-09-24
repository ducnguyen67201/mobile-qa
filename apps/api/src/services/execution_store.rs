//! Bound PostgreSQL operations through SeaORM. Never hold a transaction across device I/O.
use crate::errors::{ApiFailure, ApiResult};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
pub fn hash(bytes: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(bytes.as_ref()))
}

pub fn json<T: Serialize>(value: &T) -> ApiResult<serde_json::Value> {
    serde_json::to_value(value).map_err(|_| ApiFailure::internal())
}

pub fn decode<T: DeserializeOwned>(value: serde_json::Value) -> ApiResult<T> {
    serde_json::from_value(value).map_err(|_| ApiFailure::internal())
}

pub fn conflict(message: &str) -> ApiFailure {
    ApiFailure::new(409, "execution_conflict", message)
}

pub fn word<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .expect("enum serialization")
        .as_str()
        .expect("enum string")
        .to_owned()
}
