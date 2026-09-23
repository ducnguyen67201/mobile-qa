//! SeaORM persistence for hosted checkout and scheduled plan changes.
use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "billing_checkout_intents")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub app_id: Uuid,
    pub actor_id: Uuid,
    pub plan: String,
    pub stripe_session_id: Option<String>,
    pub stripe_session_url: Option<String>,
    pub stripe_subscription_id: Option<String>,
    pub state: String,
    pub created_at: DateTimeUtc,
    pub pending_plan: Option<String>,
    pub pending_effective_at: Option<DateTimeUtc>,
    pub stripe_schedule_id: Option<String>,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
