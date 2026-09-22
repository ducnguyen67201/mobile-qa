//! Schedule one future plan without altering the current paid credit period.
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(r#"
ALTER TABLE billing_checkout_intents
 ADD COLUMN pending_plan TEXT CHECK (pending_plan IN ('starter','plus','business')),
 ADD COLUMN pending_effective_at TIMESTAMPTZ,
 ADD COLUMN stripe_schedule_id TEXT UNIQUE,
 ADD CONSTRAINT billing_pending_plan_pair CHECK ((pending_plan IS NULL) = (pending_effective_at IS NULL));
"#).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
ALTER TABLE billing_checkout_intents
 DROP CONSTRAINT billing_pending_plan_pair,
 DROP COLUMN stripe_schedule_id,
 DROP COLUMN pending_effective_at,
 DROP COLUMN pending_plan;
"#,
            )
            .await?;
        Ok(())
    }
}
