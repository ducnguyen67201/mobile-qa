//! SeaORM persistence for environment_checks; never serialized as a browser response.
use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "environment_checks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub organization_id: Uuid,
    pub app_id: Uuid,
    pub environment_id: Uuid,
    pub environment_revision: i32,
    pub kind: String,
    pub state: String,
    pub note: Option<String>,
    pub checked_by: Uuid,
    pub checked_at: DateTimeUtc,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
