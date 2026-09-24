//! SeaORM persistence for active commercial agreements.
use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "commercial_agreements")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub app_id: Uuid,
    pub offer: String,
    pub status: String,
    pub price_revision: i32,
    pub starts_at: DateTimeUtc,
    pub ends_at: DateTimeUtc,
    pub first_source_version_id: Uuid,
    pub second_source_version_id: Option<Uuid>,
    pub profile_id: Uuid,
    pub base_cents: i32,
    pub second_suite_cents: i32,
    pub check_cents: i32,
    pub check_cap: i32,
    pub currency: String,
    pub agreement_reference: String,
    pub created_by: Uuid,
    pub created_at: DateTimeUtc,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
