//! SeaORM persistence for immutable multipart fingerprints and receipts.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "artifact_parts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub upload_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub part_number: i32,
    pub byte_size: i32,
    pub sha256: String,
    pub etag: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
