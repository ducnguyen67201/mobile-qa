//! SeaORM persistence for users; never serialized as a browser response.
use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub display_name: String,
    pub disabled_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
