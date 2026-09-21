//! Immutable nonsecret model definitions and worker capability heartbeats.
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
CREATE TABLE model_definitions(
  key TEXT NOT NULL,
  revision INTEGER NOT NULL CHECK(revision > 0),
  payload JSONB NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  retired_at TIMESTAMPTZ,
  PRIMARY KEY(key,revision),
  CONSTRAINT model_definition_key CHECK(key ~ '^[a-z0-9][a-z0-9._-]{0,99}$')
);
CREATE INDEX model_definitions_active_provider_model
  ON model_definitions((payload->>'provider'),(payload->>'provider_model'))
  WHERE retired_at IS NULL;
ALTER TABLE execution_workers ADD COLUMN model_capabilities JSONB;
ALTER TABLE execution_workers ADD COLUMN model_last_seen_at TIMESTAMPTZ;
"#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
ALTER TABLE execution_workers DROP COLUMN model_last_seen_at;
ALTER TABLE execution_workers DROP COLUMN model_capabilities;
DROP TABLE model_definitions;
"#,
            )
            .await?;
        Ok(())
    }
}
