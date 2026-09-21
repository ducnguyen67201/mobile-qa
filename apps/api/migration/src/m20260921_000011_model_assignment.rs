//! Operator assignment revisions and definition audit are additive to frozen reports.
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(r#"
ALTER TABLE model_definitions ADD COLUMN registered_by UUID;
ALTER TABLE model_definitions ADD COLUMN payload_sha256 TEXT;
CREATE TABLE model_assignments(
  app_id UUID NOT NULL REFERENCES apps(id),
  profile_id UUID NOT NULL REFERENCES execution_profiles(id),
  purpose TEXT NOT NULL CHECK(purpose IN ('navigation','structured_authoring')),
  revision INTEGER NOT NULL CHECK(revision > 0),
  payload JSONB NOT NULL,
  state TEXT NOT NULL CHECK(state IN ('staged','active','draining','retired')),
  created_by UUID NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  changed_by UUID NOT NULL,
  changed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY(app_id,profile_id,purpose,revision)
);
CREATE UNIQUE INDEX model_assignments_active ON model_assignments(app_id,profile_id,purpose) WHERE state='active';
CREATE TABLE model_assignment_events(
  id UUID PRIMARY KEY,
  app_id UUID NOT NULL,
  profile_id UUID NOT NULL,
  purpose TEXT NOT NULL,
  revision INTEGER NOT NULL,
  old_state TEXT NOT NULL,
  new_state TEXT NOT NULL,
  actor_id UUID NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  FOREIGN KEY(app_id,profile_id,purpose,revision) REFERENCES model_assignments(app_id,profile_id,purpose,revision)
);
"#).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared("DROP TABLE model_assignment_events; DROP TABLE model_assignments; ALTER TABLE model_definitions DROP COLUMN payload_sha256; ALTER TABLE model_definitions DROP COLUMN registered_by;").await?;
        Ok(())
    }
}
