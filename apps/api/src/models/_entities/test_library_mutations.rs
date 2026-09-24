//! SeaORM persistence for idempotent library mutation receipts.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "test_library_mutations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub app_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub actor_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub mutation_id: Uuid,
    pub fingerprint: String,
    pub response: Json,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
