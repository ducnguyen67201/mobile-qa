//! SeaORM persistence for fixed Android hosts.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "device_hosts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub pool_id: Uuid,
    pub instance_id: String,
    pub token_hash: String,
    pub revoked: bool,
    pub toolchain_digest: String,
    pub boot_id: Option<Uuid>,
    pub generation: i32,
    pub state: String,
    pub observed_power: String,
    pub control_version: i32,
    pub heartbeat_at: Option<DateTimeUtc>,
    pub clean_at: Option<DateTimeUtc>,
    pub last_demand_at: DateTimeUtc,
    pub startup_started_at: Option<DateTimeUtc>,
    pub cache_bytes: i64,
    pub reason: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
