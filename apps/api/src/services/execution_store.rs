//! Bound PostgreSQL operations through SeaORM. Never hold a transaction across device I/O.
use crate::errors::{ApiFailure, ApiResult};
use sea_orm::{ConnectionTrait, DatabaseBackend, QueryResult, Statement, Value};
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

pub fn field<T: sea_orm::TryGetable>(row: &QueryResult, name: &str) -> ApiResult<T> {
    row.try_get("", name).map_err(Into::into)
}

pub async fn rows(
    db: &impl ConnectionTrait,
    sql: &str,
    args: Vec<Value>,
) -> ApiResult<Vec<QueryResult>> {
    Ok(db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            args,
        ))
        .await?)
}

pub async fn one(db: &impl ConnectionTrait, sql: &str, args: Vec<Value>) -> ApiResult<QueryResult> {
    db.query_one_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        sql,
        args,
    ))
    .await?
    .ok_or_else(ApiFailure::missing)
}

pub async fn exec(db: &impl ConnectionTrait, sql: &str, args: Vec<Value>) -> ApiResult<u64> {
    Ok(db
        .execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            args,
        ))
        .await?
        .rows_affected())
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
