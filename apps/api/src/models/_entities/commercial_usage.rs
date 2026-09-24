//! SeaORM persistence for reserved and reviewed commercial run usage.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "commercial_usage")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub run_id: Uuid,
    pub quote_id: Uuid,
    pub agreement_id: Uuid,
    pub app_id: Uuid,
    pub period_start: DateTimeUtc,
    pub period_end: DateTimeUtc,
    pub amount_cents: i32,
    pub state: String,
    pub reason: Option<String>,
    pub reviewer_id: Option<Uuid>,
    pub reviewed_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
