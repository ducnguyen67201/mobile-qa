//! App-scoped commercial agreements and one reservation per accepted run.
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(r#"
CREATE TABLE commercial_agreements (
 id UUID PRIMARY KEY,
 app_id UUID NOT NULL REFERENCES apps(id),
 offer TEXT NOT NULL CHECK (offer IN ('pilot','recurring')),
 status TEXT NOT NULL CHECK (status IN ('active','paused','ended')),
 price_revision INTEGER NOT NULL CHECK (price_revision > 0),
 starts_at TIMESTAMPTZ NOT NULL,
 ends_at TIMESTAMPTZ NOT NULL,
 first_source_version_id UUID NOT NULL REFERENCES execution_definitions(id),
 second_source_version_id UUID REFERENCES execution_definitions(id),
 profile_id UUID NOT NULL REFERENCES execution_profiles(id),
 base_cents INTEGER NOT NULL CHECK (base_cents >= 0),
 second_suite_cents INTEGER NOT NULL CHECK (second_suite_cents >= 0),
 check_cents INTEGER NOT NULL CHECK (check_cents >= 0),
 check_cap INTEGER NOT NULL CHECK (check_cap BETWEEN 1 AND 8),
 currency TEXT NOT NULL DEFAULT 'USD' CHECK (currency = 'USD'),
 agreement_reference TEXT NOT NULL,
 created_by UUID NOT NULL REFERENCES users(id),
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 CHECK (ends_at > starts_at),
 CHECK (second_source_version_id IS NULL OR second_source_version_id <> first_source_version_id)
);
CREATE UNIQUE INDEX commercial_one_active_agreement_per_app ON commercial_agreements(app_id) WHERE status='active';
CREATE INDEX commercial_agreements_app_history ON commercial_agreements(app_id,created_at DESC);
CREATE TABLE commercial_quotes (
 id UUID PRIMARY KEY,
 agreement_id UUID NOT NULL REFERENCES commercial_agreements(id),
 app_id UUID NOT NULL REFERENCES apps(id),
 actor_id UUID NOT NULL REFERENCES users(id),
 kind TEXT NOT NULL CHECK (kind IN ('release_plan','saved_suite')),
 request_hash TEXT NOT NULL,
 manifest_hash TEXT NOT NULL,
 build_id UUID NOT NULL REFERENCES builds(id),
 source_version_id UUID NOT NULL REFERENCES execution_definitions(id),
 profile_id UUID NOT NULL REFERENCES execution_profiles(id),
 environment_revision INTEGER NOT NULL,
 price_revision INTEGER NOT NULL,
 amount_cents INTEGER NOT NULL CHECK (amount_cents >= 0),
 expires_at TIMESTAMPTZ NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX commercial_quotes_app_actor ON commercial_quotes(app_id,actor_id,expires_at);
CREATE TABLE commercial_usage (
 run_id UUID PRIMARY KEY REFERENCES execution_runs(id),
 quote_id UUID NOT NULL UNIQUE REFERENCES commercial_quotes(id),
 agreement_id UUID NOT NULL REFERENCES commercial_agreements(id),
 app_id UUID NOT NULL REFERENCES apps(id),
 period_start TIMESTAMPTZ NOT NULL,
 period_end TIMESTAMPTZ NOT NULL,
 amount_cents INTEGER NOT NULL CHECK (amount_cents >= 0),
 state TEXT NOT NULL DEFAULT 'reserved' CHECK (state IN ('reserved','delivered','credited')),
 reason TEXT,
 reviewer_id UUID REFERENCES users(id),
 reviewed_at TIMESTAMPTZ,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 CHECK (period_end > period_start),
 CHECK ((state='reserved' AND reviewer_id IS NULL AND reviewed_at IS NULL) OR
        (state<>'reserved' AND reviewer_id IS NOT NULL AND reviewed_at IS NOT NULL))
);
CREATE INDEX commercial_usage_agreement ON commercial_usage(agreement_id,state,created_at);
CREATE TABLE commercial_audit (
 id UUID PRIMARY KEY,
 agreement_id UUID NOT NULL REFERENCES commercial_agreements(id),
 run_id UUID REFERENCES execution_runs(id),
 actor_id UUID NOT NULL REFERENCES users(id),
 action TEXT NOT NULL CHECK (action IN ('activated','paused','ended','delivered','credited')),
 reason TEXT NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX commercial_audit_agreement ON commercial_audit(agreement_id,created_at);
CREATE TABLE commercial_pilot_requests (
 id UUID PRIMARY KEY,
 app_id UUID NOT NULL REFERENCES apps(id),
 actor_id UUID NOT NULL REFERENCES users(id),
 coverage_note TEXT NOT NULL CHECK (char_length(coverage_note) BETWEEN 1 AND 500),
 created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX commercial_one_pilot_request_per_app ON commercial_pilot_requests(app_id);
"#).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared("DROP TABLE commercial_pilot_requests, commercial_audit, commercial_usage, commercial_quotes, commercial_agreements;").await?;
        Ok(())
    }
}
