//! New runs pin comparison inputs; old session activity remains unclassified.
use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(r#"
 ALTER TABLE execution_runs ALTER COLUMN plan_id DROP NOT NULL;
 ALTER TABLE execution_runs ADD COLUMN baseline_run_id UUID REFERENCES execution_runs(id);
 ALTER TABLE execution_runs ADD COLUMN comparison JSONB;
 ALTER TABLE execution_runs ADD CONSTRAINT comparison_not_self CHECK(baseline_run_id IS DISTINCT FROM id);
 ALTER TABLE phone_tasks ADD COLUMN purpose TEXT CHECK(purpose IN ('trial','manual'));
 CREATE INDEX execution_runs_app_created ON execution_runs(app_id,created_at DESC,id DESC);
 "#).await?;
        Ok(())
    }
    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "Forward-only: saved-case runs cannot be converted into legacy plans".into(),
        ))
    }
}
