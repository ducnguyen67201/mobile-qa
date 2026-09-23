//! SeaORM persistence for durable execution attempts.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "execution_attempts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub run_id: Uuid,
    pub case_index: i32,
    pub number: i32,
    pub generation: i32,
    pub state: String,
    pub worker_id: Option<Uuid>,
    pub claim_id: Option<Uuid>,
    pub lease_hash: Option<String>,
    pub expires_at: Option<DateTimeUtc>,
    pub outcome: Option<String>,
    pub cleanup: String,
    pub reason: Option<String>,
    pub checks: Json,
    pub usage: Json,
    pub completion_hash: Option<String>,
    pub cleanup_hash: Option<String>,
    pub cleanup_receipt: Option<Json>,
    pub claimed_at: Option<DateTimeUtc>,
    pub released_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
