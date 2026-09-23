//! SeaORM persistence for exclusive physical-resource ownership.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "execution_reservations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub resource: String,
    pub attempt_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
