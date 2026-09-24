//! SeaORM persistence for immutable model-catalog revisions.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "model_definitions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub key: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub revision: i32,
    pub payload: Json,
    pub created_at: DateTimeUtc,
    pub retired_at: Option<DateTimeUtc>,
    pub registered_by: Option<Uuid>,
    pub payload_sha256: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
