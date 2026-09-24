//! SeaORM persistence for interactive phone sessions.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "phone_sessions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub app_id: Uuid,
    pub creator_id: Uuid,
    pub build_id: Uuid,
    pub profile_id: Uuid,
    pub payload: Json,
    pub fingerprint: String,
    pub build_sha256: String,
    pub build_bytes: i64,
    pub worker_id: Option<Uuid>,
    pub claim_id: Option<Uuid>,
    pub lease_hash: Option<String>,
    pub expires_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
    pub deadline: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
