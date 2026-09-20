//! Review decisions remain historical audit facts; saved versions no longer need them.
use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared("ALTER TABLE test_library_versions RENAME COLUMN review_state TO legacy_review_state; ALTER TABLE test_library_versions ALTER COLUMN legacy_review_state DROP NOT NULL;").await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // A downgrade must not invent approvals for versions created by the save lifecycle.
        manager.get_connection().execute_unprepared("DO $$ BEGIN IF EXISTS(SELECT 1 FROM test_library_versions WHERE legacy_review_state IS NULL) THEN RAISE EXCEPTION 'Saved versions exist; restore a pre-upgrade backup or forward-fix instead'; END IF; END $$; ALTER TABLE test_library_versions ALTER COLUMN legacy_review_state SET NOT NULL; ALTER TABLE test_library_versions RENAME COLUMN legacy_review_state TO review_state;").await?;
        Ok(())
    }
}
