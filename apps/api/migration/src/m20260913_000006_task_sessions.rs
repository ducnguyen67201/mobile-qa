//! Interactive sessions share the existing physical device reservation namespace.
use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(r#"
 CREATE TABLE phone_sessions (
 id UUID PRIMARY KEY, app_id UUID NOT NULL REFERENCES apps(id), creator_id UUID NOT NULL REFERENCES users(id),
 build_id UUID NOT NULL REFERENCES builds(id), profile_id UUID NOT NULL REFERENCES execution_profiles(id),
 payload JSONB NOT NULL, fingerprint TEXT NOT NULL, build_sha256 TEXT NOT NULL, build_bytes INTEGER NOT NULL,
 worker_id UUID REFERENCES execution_workers(id), claim_id UUID, lease_hash TEXT, expires_at TIMESTAMPTZ,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(), deadline TIMESTAMPTZ NOT NULL DEFAULT now()+interval '20 minutes',
 UNIQUE(worker_id,claim_id));
 CREATE TABLE phone_tasks (id UUID PRIMARY KEY, session_id UUID NOT NULL REFERENCES phone_sessions(id),
 fingerprint TEXT NOT NULL, payload JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now());
 ALTER TABLE execution_reservations ALTER COLUMN attempt_id DROP NOT NULL;
 ALTER TABLE execution_reservations ADD COLUMN session_id UUID REFERENCES phone_sessions(id);
 ALTER TABLE execution_reservations ADD CONSTRAINT reservation_owner CHECK ((attempt_id IS NULL) <> (session_id IS NULL));
 CREATE INDEX phone_sessions_app ON phone_sessions(app_id,created_at);
 "#).await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared("DELETE FROM execution_reservations WHERE session_id IS NOT NULL; ALTER TABLE execution_reservations DROP CONSTRAINT reservation_owner; ALTER TABLE execution_reservations DROP COLUMN session_id; ALTER TABLE execution_reservations ALTER COLUMN attempt_id SET NOT NULL; DROP TABLE phone_tasks,phone_sessions;").await?;
        Ok(())
    }
}
