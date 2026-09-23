//! SeaORM persistence for bounded credit quotes.
use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "billing_credit_quotes")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub period_id: Uuid,
    pub app_id: Uuid,
    pub actor_id: Uuid,
    pub kind: String,
    pub request_hash: String,
    pub manifest_hash: String,
    pub build_id: Uuid,
    pub source_version_id: Uuid,
    pub profile_id: Uuid,
    pub environment_revision: i32,
    pub rate_revision: i32,
    pub max_credits: i64,
    pub expires_at: DateTimeUtc,
    pub created_at: DateTimeUtc,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
