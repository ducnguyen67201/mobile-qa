//! Paid monthly grants, immutable invoice identity, and one bounded credit hold per run.
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(r#"
ALTER TABLE execution_attempts ADD COLUMN claimed_at TIMESTAMPTZ;
ALTER TABLE execution_attempts ADD COLUMN released_at TIMESTAMPTZ;
CREATE TABLE billing_checkout_intents (
 id UUID PRIMARY KEY, app_id UUID NOT NULL REFERENCES apps(id), actor_id UUID NOT NULL REFERENCES users(id),
 plan TEXT NOT NULL CHECK (plan IN ('starter','plus','business')),
 stripe_session_id TEXT UNIQUE, stripe_session_url TEXT, stripe_subscription_id TEXT UNIQUE,
 state TEXT NOT NULL DEFAULT 'pending' CHECK (state IN ('pending','checkout','paid','expired')),
 created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX billing_checkout_intents_app ON billing_checkout_intents(app_id,created_at DESC);
CREATE UNIQUE INDEX billing_checkout_one_open ON billing_checkout_intents(app_id) WHERE state IN ('pending','checkout');
CREATE TABLE billing_credit_periods (
 id UUID PRIMARY KEY, app_id UUID NOT NULL REFERENCES apps(id), checkout_intent_id UUID NOT NULL REFERENCES billing_checkout_intents(id),
 stripe_subscription_id TEXT NOT NULL, stripe_invoice_id TEXT NOT NULL UNIQUE,
 plan TEXT NOT NULL CHECK (plan IN ('starter','plus','business')),
 granted_credits BIGINT NOT NULL CHECK (granted_credits>0),
 starts_at TIMESTAMPTZ NOT NULL, ends_at TIMESTAMPTZ NOT NULL,
 rate_revision INTEGER NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 CHECK (ends_at>starts_at)
);
CREATE INDEX billing_credit_periods_app ON billing_credit_periods(app_id,starts_at DESC);
CREATE UNIQUE INDEX billing_credit_one_grant_per_period ON billing_credit_periods(stripe_subscription_id,starts_at);
CREATE TABLE billing_credit_quotes (
 id UUID PRIMARY KEY, period_id UUID NOT NULL REFERENCES billing_credit_periods(id), app_id UUID NOT NULL REFERENCES apps(id),
 actor_id UUID NOT NULL REFERENCES users(id), kind TEXT NOT NULL CHECK (kind IN ('release_plan','saved_suite')),
 request_hash TEXT NOT NULL, manifest_hash TEXT NOT NULL, build_id UUID NOT NULL REFERENCES builds(id),
 source_version_id UUID NOT NULL REFERENCES execution_definitions(id), profile_id UUID NOT NULL REFERENCES execution_profiles(id),
 environment_revision INTEGER NOT NULL, rate_revision INTEGER NOT NULL, max_credits BIGINT NOT NULL CHECK (max_credits>0),
 expires_at TIMESTAMPTZ NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE billing_credit_usage (
 run_id UUID PRIMARY KEY REFERENCES execution_runs(id), quote_id UUID NOT NULL UNIQUE REFERENCES billing_credit_quotes(id),
 period_id UUID NOT NULL REFERENCES billing_credit_periods(id), app_id UUID NOT NULL REFERENCES apps(id),
 held_credits BIGINT NOT NULL CHECK (held_credits>0), measured_credits BIGINT,
 charged_credits BIGINT NOT NULL DEFAULT 0 CHECK (charged_credits>=0),
 input_tokens BIGINT, output_tokens BIGINT, device_seconds BIGINT, stored_bytes BIGINT,
 state TEXT NOT NULL DEFAULT 'held' CHECK (state IN ('held','settled','released')),
 reason TEXT, reviewer_id UUID REFERENCES users(id), reviewed_at TIMESTAMPTZ,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 CHECK (charged_credits<=held_credits),
 CHECK ((state='held' AND reviewed_at IS NULL) OR (state<>'held' AND reviewed_at IS NOT NULL))
);
CREATE INDEX billing_credit_usage_period ON billing_credit_usage(period_id,state);
"#).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
DROP TABLE billing_credit_usage;
DROP TABLE billing_credit_quotes;
DROP TABLE billing_credit_periods;
DROP TABLE billing_checkout_intents;
ALTER TABLE execution_attempts DROP COLUMN released_at;
ALTER TABLE execution_attempts DROP COLUMN claimed_at;
"#,
            )
            .await?;
        Ok(())
    }
}
