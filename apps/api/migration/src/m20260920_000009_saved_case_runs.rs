use sea_orm_migration::{prelude::*, sea_orm::ConnectionTrait};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE execution_runs ALTER COLUMN plan_id DROP NOT NULL;
                 ALTER TABLE execution_runs ADD COLUMN source_kind TEXT NOT NULL
                    DEFAULT 'legacy_release_plan'
                    CHECK(source_kind IN ('legacy_release_plan','release_plan','saved_case'));
                 ALTER TABLE execution_runs ADD COLUMN baseline_run_id UUID
                    REFERENCES execution_runs(id);
                 ALTER TABLE execution_runs ADD CONSTRAINT execution_baseline_not_self
                    CHECK(baseline_run_id IS NULL OR baseline_run_id<>id);
                 CREATE INDEX execution_runs_baselines
                    ON execution_runs(app_id,source_kind,created_at DESC,id DESC);",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "UPDATE execution_runs SET baseline_run_id=NULL
                    WHERE baseline_run_id IN (
                        SELECT id FROM execution_runs WHERE source_kind='saved_case'
                    );
                 DELETE FROM execution_recovery_events WHERE attempt_id IN (
                    SELECT a.id FROM execution_attempts a
                    JOIN execution_runs r ON r.id=a.run_id WHERE r.source_kind='saved_case'
                 );
                 DELETE FROM execution_preflight_receipts WHERE attempt_id IN (
                    SELECT a.id FROM execution_attempts a
                    JOIN execution_runs r ON r.id=a.run_id WHERE r.source_kind='saved_case'
                 );
                 DELETE FROM execution_artifacts WHERE attempt_id IN (
                    SELECT a.id FROM execution_attempts a
                    JOIN execution_runs r ON r.id=a.run_id WHERE r.source_kind='saved_case'
                 );
                 DELETE FROM execution_events WHERE attempt_id IN (
                    SELECT a.id FROM execution_attempts a
                    JOIN execution_runs r ON r.id=a.run_id WHERE r.source_kind='saved_case'
                 );
                 DELETE FROM execution_reservations WHERE attempt_id IN (
                    SELECT a.id FROM execution_attempts a
                    JOIN execution_runs r ON r.id=a.run_id WHERE r.source_kind='saved_case'
                 );
                 DELETE FROM execution_attempts WHERE run_id IN (
                    SELECT id FROM execution_runs WHERE source_kind='saved_case'
                 );
                 DELETE FROM execution_runs WHERE source_kind='saved_case';
                 DROP INDEX execution_runs_baselines;
                 ALTER TABLE execution_runs DROP CONSTRAINT execution_baseline_not_self;
                 ALTER TABLE execution_runs DROP COLUMN baseline_run_id;
                 ALTER TABLE execution_runs DROP COLUMN source_kind;
                 ALTER TABLE execution_runs ALTER COLUMN plan_id SET NOT NULL;",
            )
            .await?;
        Ok(())
    }
}
