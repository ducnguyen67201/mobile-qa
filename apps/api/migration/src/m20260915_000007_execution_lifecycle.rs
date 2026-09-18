//! Preserve original execution evidence separately from later operational recovery.
use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
CREATE TABLE execution_preflight_receipts (
 attempt_id UUID NOT NULL REFERENCES execution_attempts(id), generation INTEGER NOT NULL,
 digest TEXT NOT NULL, payload JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 PRIMARY KEY(attempt_id,generation));
CREATE TABLE execution_recovery_events (
 id UUID PRIMARY KEY, attempt_id UUID NOT NULL REFERENCES execution_attempts(id),
 actor_id UUID NOT NULL REFERENCES users(id), evidence_reference TEXT NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE INDEX execution_recovery_attempt ON execution_recovery_events(attempt_id,created_at);
"#,
            )
            .await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DROP TABLE execution_recovery_events,execution_preflight_receipts;",
            )
            .await?;
        Ok(())
    }
}
