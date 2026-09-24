//! SeaORM persistence for app/profile model-assignment revisions.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "model_assignments")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub app_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub profile_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub purpose: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub revision: i32,
    pub payload: Json,
    pub state: String,
    pub created_by: Uuid,
    pub created_at: DateTimeUtc,
    pub changed_by: Uuid,
    pub changed_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
