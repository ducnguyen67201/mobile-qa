//! Replace password identities without dropping apps, memberships, or build history.
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
ALTER TABLE users ADD COLUMN google_subject TEXT UNIQUE;
ALTER TABLE users DROP COLUMN password_hash;
UPDATE sessions SET revoked_at = CURRENT_TIMESTAMP WHERE revoked_at IS NULL;
CREATE TABLE google_login_challenges (
    id UUID PRIMARY KEY,
    nonce TEXT NOT NULL,
    cookie_hash TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX google_login_challenges_expiry ON google_login_challenges(expires_at);
"#,
            )
            .await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Old passwords cannot be recovered. A rollback leaves password login disabled.
        manager
            .get_connection()
            .execute_unprepared(
                r#"
DROP TABLE google_login_challenges;
ALTER TABLE users DROP COLUMN google_subject;
ALTER TABLE users ADD COLUMN password_hash TEXT NOT NULL DEFAULT '!disabled';
UPDATE sessions SET revoked_at = CURRENT_TIMESTAMP WHERE revoked_at IS NULL;
"#,
            )
            .await?;
        Ok(())
    }
}
