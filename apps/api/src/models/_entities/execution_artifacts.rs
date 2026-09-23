//! SeaORM persistence for attempt artifacts.
use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "execution_artifacts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub attempt_id: Uuid,
    pub checkpoint_id: String,
    pub name: String,
    pub mime: String,
    pub byte_size: i64,
    pub sha256: String,
    pub state: String,
    pub reason: Option<String>,
    pub storage_key: String,
    pub storage_backend: String,
    pub created_at: DateTimeUtc,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
