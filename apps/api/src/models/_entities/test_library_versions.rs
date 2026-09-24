//! SeaORM persistence for the entry-to-definition version mapping.
use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "test_library_versions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub definition_id: Uuid,
    pub entry_id: Uuid,
    pub legacy_review_state: Option<String>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
