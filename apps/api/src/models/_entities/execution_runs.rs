//! SeaORM persistence for immutable run manifests.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "execution_runs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub app_id: Uuid,
    pub creator_id: Uuid,
    pub build_id: Uuid,
    pub plan_id: Option<Uuid>,
    pub idempotency_key: String,
    pub fingerprint: String,
    pub manifest: Json,
    pub cancel_requested: bool,
    pub created_at: DateTimeUtc,
    pub baseline_run_id: Option<Uuid>,
    pub comparison: Option<Json>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
