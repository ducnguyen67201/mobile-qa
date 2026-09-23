//! SeaORM persistence for leased capacity-wake delivery.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "capacity_wake_outbox")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub pool_id: Uuid,
    pub created_at: DateTimeUtc,
    pub next_attempt_at: DateTimeUtc,
    pub attempts: i32,
    pub delivered_at: Option<DateTimeUtc>,
    pub lease_id: Option<Uuid>,
    pub lease_until: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
