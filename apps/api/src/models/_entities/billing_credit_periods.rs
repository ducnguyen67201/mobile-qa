//! SeaORM persistence for immutable monthly credit grants.
use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "billing_credit_periods")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub app_id: Uuid,
    pub checkout_intent_id: Uuid,
    pub stripe_subscription_id: String,
    pub stripe_invoice_id: String,
    pub plan: String,
    pub granted_credits: i64,
    pub starts_at: DateTimeUtc,
    pub ends_at: DateTimeUtc,
    pub rate_revision: i32,
    pub created_at: DateTimeUtc,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
