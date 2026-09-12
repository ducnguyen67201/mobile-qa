//! SeaORM persistence for build_uploads; never serialized as a browser response.
use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "build_uploads")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub organization_id: Uuid,
    pub app_id: Uuid,
    pub created_by: Uuid,
    pub original_filename: String,
    pub expected_size: i64,
    pub state: String,
    pub expires_at: DateTimeUtc,
    pub attempt_id: Option<Uuid>,
    pub lease_until: Option<DateTimeUtc>,
    pub storage_backend: String,
    pub sealed_storage_key: Option<String>,
    pub actual_size: Option<i64>,
    pub sha256: Option<String>,
    pub created_at: DateTimeUtc,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
