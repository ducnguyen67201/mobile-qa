//! Keep execution and phone claim advertisements distinct without rewriting earlier migrations.
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Claim protocols have different gates. A phone heartbeat must not make an
        // execution worker appear eligible for a queued run.
        manager
            .get_connection()
            .execute_unprepared(
                r#"
ALTER TABLE execution_workers ADD COLUMN execution_protocol_version INTEGER;
ALTER TABLE execution_workers ADD COLUMN execution_model_capabilities JSONB;
ALTER TABLE execution_workers ADD COLUMN execution_last_seen_at TIMESTAMPTZ;
ALTER TABLE execution_workers ADD COLUMN phone_protocol_version INTEGER;
ALTER TABLE execution_workers ADD COLUMN phone_model_capabilities JSONB;
ALTER TABLE execution_workers ADD COLUMN phone_last_seen_at TIMESTAMPTZ;
"#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared("ALTER TABLE execution_workers DROP COLUMN phone_last_seen_at, DROP COLUMN phone_model_capabilities, DROP COLUMN phone_protocol_version, DROP COLUMN execution_last_seen_at, DROP COLUMN execution_model_capabilities, DROP COLUMN execution_protocol_version;").await?;
        Ok(())
    }
}
