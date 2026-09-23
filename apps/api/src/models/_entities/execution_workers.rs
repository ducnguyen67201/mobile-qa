//! SeaORM persistence for worker identity and host-slot grants.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "execution_workers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub app_id: Uuid,
    pub profile_id: Uuid,
    pub token_hash: String,
    pub revoked: bool,
    pub model_capabilities: Option<Json>,
    pub model_last_seen_at: Option<DateTimeUtc>,
    pub execution_protocol_version: Option<i32>,
    pub execution_model_capabilities: Option<Json>,
    pub execution_last_seen_at: Option<DateTimeUtc>,
    pub phone_protocol_version: Option<i32>,
    pub phone_model_capabilities: Option<Json>,
    pub phone_last_seen_at: Option<DateTimeUtc>,
    pub host_slot_id: Option<Uuid>,
    pub host_generation: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
