//! Paid monthly grants, immutable invoice identity, and one bounded credit hold per run.
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("execution_attempts"))
                    .add_column(ColumnDef::new(Alias::new("claimed_at")).timestamp_with_time_zone())
                    .add_column(
                        ColumnDef::new(Alias::new("released_at")).timestamp_with_time_zone(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("billing_checkout_intents"))
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Alias::new("app_id")).uuid().not_null())
                    .col(ColumnDef::new(Alias::new("actor_id")).uuid().not_null())
                    .col(ColumnDef::new(Alias::new("plan")).text().not_null().check(
                        Expr::col(Alias::new("plan")).is_in(["starter", "plus", "business"]),
                    ))
                    .col(
                        ColumnDef::new(Alias::new("stripe_session_id"))
                            .text()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Alias::new("stripe_session_url")).text())
                    .col(
                        ColumnDef::new(Alias::new("stripe_subscription_id"))
                            .text()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("state"))
                            .text()
                            .not_null()
                            .default("pending")
                            .check(
                                Expr::col(Alias::new("state"))
                                    .is_in(["pending", "checkout", "paid", "expired"]),
                            ),
                    )
                    .col(
                        ColumnDef::new(Alias::new("created_at"))
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("billing_checkout_intents"), Alias::new("app_id"))
                            .to(Alias::new("apps"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                Alias::new("billing_checkout_intents"),
                                Alias::new("actor_id"),
                            )
                            .to(Alias::new("users"), Alias::new("id")),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("billing_checkout_intents_app")
                    .table(Alias::new("billing_checkout_intents"))
                    .col(Alias::new("app_id"))
                    .col((Alias::new("created_at"), IndexOrder::Desc))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("billing_checkout_one_open")
                    .unique()
                    .table(Alias::new("billing_checkout_intents"))
                    .col(Alias::new("app_id"))
                    .and_where(Expr::col(Alias::new("state")).is_in(["pending", "checkout"]))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Alias::new("billing_credit_periods"))
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Alias::new("app_id")).uuid().not_null())
                    .col(
                        ColumnDef::new(Alias::new("checkout_intent_id"))
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("stripe_subscription_id"))
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("stripe_invoice_id"))
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Alias::new("plan")).text().not_null().check(
                        Expr::col(Alias::new("plan")).is_in(["starter", "plus", "business"]),
                    ))
                    .col(
                        ColumnDef::new(Alias::new("granted_credits"))
                            .big_integer()
                            .not_null()
                            .check(Expr::col(Alias::new("granted_credits")).gt(0)),
                    )
                    .col(
                        ColumnDef::new(Alias::new("starts_at"))
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("ends_at"))
                            .timestamp_with_time_zone()
                            .not_null()
                            .check(
                                Expr::col(Alias::new("ends_at"))
                                    .gt(Expr::col(Alias::new("starts_at"))),
                            ),
                    )
                    .col(
                        ColumnDef::new(Alias::new("rate_revision"))
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("created_at"))
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("billing_credit_periods"), Alias::new("app_id"))
                            .to(Alias::new("apps"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                Alias::new("billing_credit_periods"),
                                Alias::new("checkout_intent_id"),
                            )
                            .to(Alias::new("billing_checkout_intents"), Alias::new("id")),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("billing_credit_periods_app")
                    .table(Alias::new("billing_credit_periods"))
                    .col(Alias::new("app_id"))
                    .col((Alias::new("starts_at"), IndexOrder::Desc))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("billing_credit_one_grant_per_period")
                    .unique()
                    .table(Alias::new("billing_credit_periods"))
                    .col(Alias::new("stripe_subscription_id"))
                    .col(Alias::new("starts_at"))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Alias::new("billing_credit_quotes"))
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Alias::new("period_id")).uuid().not_null())
                    .col(ColumnDef::new(Alias::new("app_id")).uuid().not_null())
                    .col(ColumnDef::new(Alias::new("actor_id")).uuid().not_null())
                    .col(ColumnDef::new(Alias::new("kind")).text().not_null().check(
                        Expr::col(Alias::new("kind")).is_in(["release_plan", "saved_suite"]),
                    ))
                    .col(ColumnDef::new(Alias::new("request_hash")).text().not_null())
                    .col(
                        ColumnDef::new(Alias::new("manifest_hash"))
                            .text()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Alias::new("build_id")).uuid().not_null())
                    .col(
                        ColumnDef::new(Alias::new("source_version_id"))
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Alias::new("profile_id")).uuid().not_null())
                    .col(
                        ColumnDef::new(Alias::new("environment_revision"))
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("rate_revision"))
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("max_credits"))
                            .big_integer()
                            .not_null()
                            .check(Expr::col(Alias::new("max_credits")).gt(0)),
                    )
                    .col(
                        ColumnDef::new(Alias::new("expires_at"))
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("created_at"))
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("billing_credit_quotes"), Alias::new("period_id"))
                            .to(Alias::new("billing_credit_periods"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("billing_credit_quotes"), Alias::new("app_id"))
                            .to(Alias::new("apps"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("billing_credit_quotes"), Alias::new("actor_id"))
                            .to(Alias::new("users"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("billing_credit_quotes"), Alias::new("build_id"))
                            .to(Alias::new("builds"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                Alias::new("billing_credit_quotes"),
                                Alias::new("source_version_id"),
                            )
                            .to(Alias::new("execution_definitions"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                Alias::new("billing_credit_quotes"),
                                Alias::new("profile_id"),
                            )
                            .to(Alias::new("execution_profiles"), Alias::new("id")),
                    )
                    .to_owned(),
            )
            .await?;

        let review_is_valid = Expr::col(Alias::new("state"))
            .eq("held")
            .and(Expr::col(Alias::new("reviewed_at")).is_null())
            .or(Expr::col(Alias::new("state"))
                .ne("held")
                .and(Expr::col(Alias::new("reviewed_at")).is_not_null()));
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("billing_credit_usage"))
                    .col(
                        ColumnDef::new(Alias::new("run_id"))
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("quote_id"))
                            .uuid()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Alias::new("period_id")).uuid().not_null())
                    .col(ColumnDef::new(Alias::new("app_id")).uuid().not_null())
                    .col(
                        ColumnDef::new(Alias::new("held_credits"))
                            .big_integer()
                            .not_null()
                            .check(Expr::col(Alias::new("held_credits")).gt(0)),
                    )
                    .col(ColumnDef::new(Alias::new("measured_credits")).big_integer())
                    .col(
                        ColumnDef::new(Alias::new("charged_credits"))
                            .big_integer()
                            .not_null()
                            .default(0)
                            .check(Expr::col(Alias::new("charged_credits")).gte(0))
                            .check(
                                Expr::col(Alias::new("charged_credits"))
                                    .lte(Expr::col(Alias::new("held_credits"))),
                            ),
                    )
                    .col(ColumnDef::new(Alias::new("input_tokens")).big_integer())
                    .col(ColumnDef::new(Alias::new("output_tokens")).big_integer())
                    .col(ColumnDef::new(Alias::new("device_seconds")).big_integer())
                    .col(ColumnDef::new(Alias::new("stored_bytes")).big_integer())
                    .col(
                        ColumnDef::new(Alias::new("state"))
                            .text()
                            .not_null()
                            .default("held")
                            .check(
                                Expr::col(Alias::new("state"))
                                    .is_in(["held", "settled", "released"]),
                            ),
                    )
                    .col(ColumnDef::new(Alias::new("reason")).text())
                    .col(ColumnDef::new(Alias::new("reviewer_id")).uuid())
                    .col(
                        ColumnDef::new(Alias::new("reviewed_at"))
                            .timestamp_with_time_zone()
                            .check(review_is_valid),
                    )
                    .col(
                        ColumnDef::new(Alias::new("created_at"))
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("billing_credit_usage"), Alias::new("run_id"))
                            .to(Alias::new("execution_runs"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("billing_credit_usage"), Alias::new("quote_id"))
                            .to(Alias::new("billing_credit_quotes"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("billing_credit_usage"), Alias::new("period_id"))
                            .to(Alias::new("billing_credit_periods"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("billing_credit_usage"), Alias::new("app_id"))
                            .to(Alias::new("apps"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                Alias::new("billing_credit_usage"),
                                Alias::new("reviewer_id"),
                            )
                            .to(Alias::new("users"), Alias::new("id")),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("billing_credit_usage_period")
                    .table(Alias::new("billing_credit_usage"))
                    .col(Alias::new("period_id"))
                    .col(Alias::new("state"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            "billing_credit_usage",
            "billing_credit_quotes",
            "billing_credit_periods",
            "billing_checkout_intents",
        ] {
            manager
                .drop_table(Table::drop().table(Alias::new(table)).to_owned())
                .await?;
        }
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("execution_attempts"))
                    .drop_column(Alias::new("released_at"))
                    .drop_column(Alias::new("claimed_at"))
                    .to_owned(),
            )
            .await
    }
}
