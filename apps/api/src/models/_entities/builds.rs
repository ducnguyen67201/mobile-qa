//! SeaORM persistence for builds; never serialized as a browser response.
use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "builds")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub organization_id: Uuid,
    pub app_id: Uuid,
    pub upload_id: Uuid,
    pub storage_backend: String,
    pub storage_key: String,
    pub sha256: String,
    pub byte_size: i64,
    pub original_filename: String,
    pub validation_state: String,
    pub reason_code: Option<String>,
    pub message: Option<String>,
    pub metadata: Option<Json>,
    pub validator_version: String,
    pub intake_policy_version: String,
    pub attempt_id: Option<Uuid>,
    pub lease_until: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
    pub started_at: DateTimeUtc,
    pub validated_at: Option<DateTimeUtc>,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
