//! Durable definitions and physical-resource reservations; expiry never releases a phone.
use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(r#"
 CREATE TABLE execution_definitions (
 id UUID PRIMARY KEY, app_id UUID NOT NULL REFERENCES apps(id), kind TEXT NOT NULL CHECK(kind IN ('case','suite','plan')),
 logical_key TEXT NOT NULL, version INTEGER NOT NULL CHECK(version>0), content_hash TEXT NOT NULL,
 payload JSONB NOT NULL, author_id UUID NOT NULL REFERENCES users(id), created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 UNIQUE(app_id,kind,logical_key,version));
 CREATE TABLE execution_reviewer_grants (app_id UUID NOT NULL REFERENCES apps(id), user_id UUID NOT NULL REFERENCES users(id), purpose TEXT NOT NULL CHECK(purpose IN ('business','executability')), PRIMARY KEY(app_id,user_id,purpose));
 CREATE TABLE execution_approvals (definition_id UUID NOT NULL REFERENCES execution_definitions(id), purpose TEXT NOT NULL CHECK(purpose IN ('business','executability')), actor_id UUID NOT NULL REFERENCES users(id), content_hash TEXT NOT NULL, approved_at TIMESTAMPTZ NOT NULL DEFAULT now(), PRIMARY KEY(definition_id,purpose));
 CREATE TABLE execution_profiles (id UUID PRIMARY KEY, app_id UUID NOT NULL REFERENCES apps(id), payload JSONB NOT NULL);
 CREATE TABLE execution_workers (id UUID PRIMARY KEY, app_id UUID NOT NULL REFERENCES apps(id), profile_id UUID NOT NULL REFERENCES execution_profiles(id), token_hash TEXT NOT NULL UNIQUE, revoked BOOLEAN NOT NULL DEFAULT false);
 CREATE TABLE execution_runs (id UUID PRIMARY KEY, app_id UUID NOT NULL REFERENCES apps(id), creator_id UUID NOT NULL REFERENCES users(id), build_id UUID NOT NULL REFERENCES builds(id), plan_id UUID NOT NULL REFERENCES execution_definitions(id), idempotency_key TEXT NOT NULL, fingerprint TEXT NOT NULL, manifest JSONB NOT NULL, cancel_requested BOOLEAN NOT NULL DEFAULT false, created_at TIMESTAMPTZ NOT NULL DEFAULT now(), UNIQUE(app_id,idempotency_key));
 CREATE TABLE execution_attempts (id UUID PRIMARY KEY, run_id UUID NOT NULL REFERENCES execution_runs(id), case_index INTEGER NOT NULL CHECK(case_index>=0), number INTEGER NOT NULL DEFAULT 1 CHECK(number IN (1,2)), generation INTEGER NOT NULL DEFAULT 1, state TEXT NOT NULL DEFAULT 'queued' CHECK(state IN ('queued','leased','running','finalizing','finished','cancel_requested','recovery_required')), worker_id UUID REFERENCES execution_workers(id), claim_id UUID, lease_hash TEXT, expires_at TIMESTAMPTZ, outcome TEXT, cleanup TEXT NOT NULL DEFAULT 'pending' CHECK(cleanup IN ('pending','verified_clean','quarantined')), reason TEXT, checks JSONB NOT NULL DEFAULT '[]', usage JSONB NOT NULL DEFAULT '[]', completion_hash TEXT, cleanup_hash TEXT, cleanup_receipt JSONB, UNIQUE(run_id,case_index,number), UNIQUE(worker_id,claim_id));
 CREATE TABLE execution_reservations (resource TEXT PRIMARY KEY, attempt_id UUID NOT NULL REFERENCES execution_attempts(id));
 CREATE TABLE execution_events (id UUID PRIMARY KEY, attempt_id UUID NOT NULL REFERENCES execution_attempts(id), sequence INTEGER NOT NULL CHECK(sequence>0), payload JSONB NOT NULL, UNIQUE(attempt_id,sequence));
 CREATE TABLE execution_artifacts (id UUID PRIMARY KEY, attempt_id UUID NOT NULL REFERENCES execution_attempts(id), checkpoint_id TEXT NOT NULL, name TEXT NOT NULL, mime TEXT NOT NULL, byte_size BIGINT NOT NULL CHECK(byte_size>0), sha256 TEXT NOT NULL, state TEXT NOT NULL DEFAULT 'pending' CHECK(state IN ('pending','sealed','unavailable')), reason TEXT, storage_key TEXT NOT NULL UNIQUE, storage_backend TEXT NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now(), UNIQUE(attempt_id,name));
 CREATE INDEX execution_runs_app ON execution_runs(app_id,created_at DESC,id);
 CREATE INDEX execution_attempt_queue ON execution_attempts(state,expires_at);
 CREATE INDEX execution_artifacts_attempt ON execution_artifacts(attempt_id);
 "#).await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared("DROP TABLE execution_artifacts, execution_events, execution_reservations, execution_attempts, execution_runs, execution_workers, execution_profiles, execution_approvals, execution_reviewer_grants, execution_definitions;").await?;
        Ok(())
    }
}
