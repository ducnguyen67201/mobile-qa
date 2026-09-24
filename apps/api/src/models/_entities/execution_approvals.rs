//! SeaORM persistence for immutable execution-definition approvals.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "execution_approvals")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub definition_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub purpose: String,
    pub actor_id: Uuid,
    pub content_hash: String,
    pub approved_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
