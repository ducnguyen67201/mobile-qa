//! First product schema. Explicit FK/check constraints protect tenant and lifecycle invariants.
use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(r#"CREATE TABLE users (
id UUID PRIMARY KEY,
email TEXT NOT NULL,
password_hash TEXT NOT NULL,
display_name TEXT NOT NULL,
disabled_at TIMESTAMPTZ,
created_at TIMESTAMPTZ NOT NULL,
UNIQUE(email),
CHECK (char_length(email) BETWEEN 3 AND 254)
);
CREATE TABLE organizations (
id UUID PRIMARY KEY,
name TEXT NOT NULL,
created_at TIMESTAMPTZ NOT NULL
);
CREATE TABLE memberships (
id UUID PRIMARY KEY,
user_id UUID NOT NULL,
organization_id UUID NOT NULL,
role TEXT NOT NULL,
active BOOLEAN NOT NULL,
UNIQUE(user_id,organization_id),
FOREIGN KEY(user_id) REFERENCES users(id),
FOREIGN KEY(organization_id) REFERENCES organizations(id),
CHECK(role IN ('operator','member'))
);
CREATE TABLE sessions (
id UUID PRIMARY KEY,
user_id UUID NOT NULL,
csrf_token TEXT NOT NULL,
expires_at TIMESTAMPTZ NOT NULL,
revoked_at TIMESTAMPTZ,
created_at TIMESTAMPTZ NOT NULL,
FOREIGN KEY(user_id) REFERENCES users(id)
);
CREATE TABLE login_attempts (
id UUID PRIMARY KEY,
bucket TEXT NOT NULL,
window_start TIMESTAMPTZ NOT NULL,
count INTEGER NOT NULL,
expires_at TIMESTAMPTZ NOT NULL,
UNIQUE(bucket),
CHECK(count >= 0)
);
CREATE TABLE apps (
id UUID PRIMARY KEY,
organization_id UUID NOT NULL,
name TEXT NOT NULL,
android_package TEXT NOT NULL,
created_by UUID NOT NULL,
created_at TIMESTAMPTZ NOT NULL,
UNIQUE(organization_id,android_package),
UNIQUE(id,organization_id),
FOREIGN KEY(organization_id) REFERENCES organizations(id),
FOREIGN KEY(created_by) REFERENCES users(id),
CHECK(char_length(name) BETWEEN 1 AND 100),
CHECK(char_length(android_package) BETWEEN 1 AND 255)
);
CREATE TABLE app_memberships (
id UUID PRIMARY KEY,
user_id UUID NOT NULL,
organization_id UUID NOT NULL,
app_id UUID NOT NULL,
UNIQUE(user_id,app_id),
FOREIGN KEY(user_id,organization_id) REFERENCES memberships(user_id,organization_id),
FOREIGN KEY(app_id,organization_id) REFERENCES apps(id,organization_id)
);
CREATE TABLE environments (
id UUID PRIMARY KEY,
organization_id UUID NOT NULL,
app_id UUID NOT NULL,
name TEXT NOT NULL,
backend_origins JSONB NOT NULL,
login_origins JSONB NOT NULL,
revision INTEGER NOT NULL,
account_secret_reference_id UUID,
reset_secret_reference_id UUID,
updated_at TIMESTAMPTZ NOT NULL,
UNIQUE(app_id),
UNIQUE(id,app_id,organization_id),
FOREIGN KEY(app_id,organization_id) REFERENCES apps(id,organization_id),
CHECK(revision > 0),
CHECK(char_length(name) BETWEEN 1 AND 80)
);
CREATE TABLE secret_references (
id UUID PRIMARY KEY,
organization_id UUID NOT NULL,
app_id UUID NOT NULL,
label TEXT NOT NULL,
locator TEXT NOT NULL,
kind TEXT NOT NULL,
created_at TIMESTAMPTZ NOT NULL,
UNIQUE(id,app_id,organization_id),
FOREIGN KEY(app_id,organization_id) REFERENCES apps(id,organization_id),
CHECK(kind IN ('account','reset'))
);
CREATE TABLE environment_checks (
id UUID PRIMARY KEY,
organization_id UUID NOT NULL,
app_id UUID NOT NULL,
environment_id UUID NOT NULL,
environment_revision INTEGER NOT NULL,
kind TEXT NOT NULL,
state TEXT NOT NULL,
note TEXT,
checked_by UUID NOT NULL,
checked_at TIMESTAMPTZ NOT NULL,
FOREIGN KEY(environment_id,app_id,organization_id) REFERENCES environments(id,app_id,organization_id),
FOREIGN KEY(checked_by) REFERENCES users(id),
CHECK(kind IN ('backend','account','reset')),
CHECK(state IN ('operator_reported_ok','operator_reported_blocked'))
);
CREATE TABLE build_uploads (
id UUID PRIMARY KEY,
organization_id UUID NOT NULL,
app_id UUID NOT NULL,
created_by UUID NOT NULL,
original_filename TEXT NOT NULL,
expected_size BIGINT NOT NULL,
state TEXT NOT NULL,
expires_at TIMESTAMPTZ NOT NULL,
attempt_id UUID,
lease_until TIMESTAMPTZ,
storage_backend TEXT NOT NULL,
sealed_storage_key TEXT,
actual_size BIGINT,
sha256 TEXT,
created_at TIMESTAMPTZ NOT NULL,
UNIQUE(id,app_id,organization_id),
FOREIGN KEY(app_id,organization_id) REFERENCES apps(id,organization_id),
FOREIGN KEY(created_by) REFERENCES users(id),
CHECK(expected_size > 0 AND expected_size <= 262144000),
CHECK(actual_size IS NULL OR (actual_size > 0 AND actual_size = expected_size)),
CHECK(state IN ('pending','receiving','uploaded','finalized','expired'))
);
CREATE TABLE builds (
id UUID PRIMARY KEY,
organization_id UUID NOT NULL,
app_id UUID NOT NULL,
upload_id UUID NOT NULL,
storage_backend TEXT NOT NULL,
storage_key TEXT NOT NULL,
sha256 TEXT NOT NULL,
byte_size BIGINT NOT NULL,
original_filename TEXT NOT NULL,
validation_state TEXT NOT NULL,
reason_code TEXT,
message TEXT,
metadata JSONB,
validator_version TEXT NOT NULL,
intake_policy_version TEXT NOT NULL,
attempt_id UUID,
lease_until TIMESTAMPTZ,
created_at TIMESTAMPTZ NOT NULL,
started_at TIMESTAMPTZ NOT NULL,
validated_at TIMESTAMPTZ,
UNIQUE(upload_id),
FOREIGN KEY(upload_id,app_id,organization_id) REFERENCES build_uploads(id,app_id,organization_id),
CHECK(byte_size > 0 AND char_length(sha256) = 64),
CHECK(validation_state IN ('validating','validated','invalid','unsupported','error')),
CHECK(validation_state <> 'validated' OR (metadata IS NOT NULL AND validated_at IS NOT NULL))
);
ALTER TABLE environments ADD FOREIGN KEY(account_secret_reference_id,app_id,organization_id) REFERENCES secret_references(id,app_id,organization_id);
ALTER TABLE environments ADD FOREIGN KEY(reset_secret_reference_id,app_id,organization_id) REFERENCES secret_references(id,app_id,organization_id);
CREATE INDEX sessions_expiry ON sessions(expires_at);
CREATE INDEX sessions_user ON sessions(user_id);
CREATE INDEX uploads_app_created ON build_uploads(app_id,created_at,id);
CREATE INDEX uploads_state_expiry ON build_uploads(state,expires_at);
CREATE INDEX builds_app_created ON builds(app_id,created_at,id);
CREATE INDEX checks_environment_revision ON environment_checks(environment_id,environment_revision,checked_at);
"#).await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Explicit reverse migration only; normal startup never invokes this path.
        manager.get_connection().execute_unprepared("ALTER TABLE environments DROP COLUMN account_secret_reference_id; ALTER TABLE environments DROP COLUMN reset_secret_reference_id;").await?;
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE builds;")
            .await?;
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE build_uploads;")
            .await?;
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE environment_checks;")
            .await?;
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE secret_references;")
            .await?;
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE environments;")
            .await?;
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE app_memberships;")
            .await?;
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE apps;")
            .await?;
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE login_attempts;")
            .await?;
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE sessions;")
            .await?;
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE memberships;")
            .await?;
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE organizations;")
            .await?;
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE users;")
            .await?;
        Ok(())
    }
}
