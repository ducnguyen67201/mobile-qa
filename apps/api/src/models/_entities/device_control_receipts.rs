//! SeaORM persistence for idempotent controller actions.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "device_control_receipts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub pool_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub request_id: Uuid,
    pub fingerprint: String,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
