//! Schedule one future plan without altering the current paid credit period.
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let pair_is_valid = Expr::col(Alias::new("pending_plan"))
            .is_null()
            .and(Expr::col(Alias::new("pending_effective_at")).is_null())
            .or(Expr::col(Alias::new("pending_plan"))
                .is_not_null()
                .and(Expr::col(Alias::new("pending_effective_at")).is_not_null()));
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("billing_checkout_intents"))
                    .add_column(
                        ColumnDef::new(Alias::new("pending_plan")).text().check(
                            Expr::col(Alias::new("pending_plan"))
                                .is_in(["starter", "plus", "business"]),
                        ),
                    )
                    .add_column(
                        ColumnDef::new(Alias::new("pending_effective_at"))
                            .timestamp_with_time_zone(),
                    )
                    .add_column(
                        ColumnDef::new(Alias::new("stripe_schedule_id"))
                            .text()
                            .unique_key(),
                    )
                    .add_column(
                        ColumnDef::new(Alias::new("billing_pending_plan_pair"))
                            .boolean()
                            .generated(pair_is_valid, true)
                            .check((
                                "billing_pending_plan_pair",
                                Expr::col(Alias::new("billing_pending_plan_pair")).eq(true),
                            )),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("billing_checkout_intents"))
                    .drop_column(Alias::new("billing_pending_plan_pair"))
                    .drop_column(Alias::new("stripe_schedule_id"))
                    .drop_column(Alias::new("pending_effective_at"))
                    .drop_column(Alias::new("pending_plan"))
                    .to_owned(),
            )
            .await
    }
}
