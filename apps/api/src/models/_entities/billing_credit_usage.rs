//! SeaORM persistence for credit holds and settlement.
use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "billing_credit_usage")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub run_id: Uuid,
    pub quote_id: Uuid,
    pub period_id: Uuid,
    pub app_id: Uuid,
    pub held_credits: i64,
    pub measured_credits: Option<i64>,
    pub charged_credits: i64,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub device_seconds: Option<i64>,
    pub stored_bytes: Option<i64>,
    pub state: String,
    pub reason: Option<String>,
    pub reviewer_id: Option<Uuid>,
    pub reviewed_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
