//! SeaORM persistence for environments; never serialized as a browser response.
use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "environments")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub organization_id: Uuid,
    pub app_id: Uuid,
    pub name: String,
    pub backend_origins: Json,
    pub login_origins: Json,
    pub revision: i32,
    pub account_secret_reference_id: Option<Uuid>,
    pub reset_secret_reference_id: Option<Uuid>,
    pub updated_at: DateTimeUtc,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
