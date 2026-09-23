//! SeaORM persistence for restartable artifact work.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "artifact_jobs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub upload_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub phase: String,
    pub state: String,
    pub attempt_id: Option<Uuid>,
    pub lease_until: Option<DateTimeUtc>,
    pub attempts: i32,
    pub reason_code: Option<String>,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
