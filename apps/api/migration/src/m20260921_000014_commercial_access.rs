//! App-scoped commercial agreements and one reservation per accepted run.
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("commercial_agreements"))
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Alias::new("app_id")).uuid().not_null())
                    .col(
                        ColumnDef::new(Alias::new("offer"))
                            .text()
                            .not_null()
                            .check(Expr::col(Alias::new("offer")).is_in(["pilot", "recurring"])),
                    )
                    .col(
                        ColumnDef::new(Alias::new("status"))
                            .text()
                            .not_null()
                            .check(
                                Expr::col(Alias::new("status"))
                                    .is_in(["active", "paused", "ended"]),
                            ),
                    )
                    .col(
                        ColumnDef::new(Alias::new("price_revision"))
                            .integer()
                            .not_null()
                            .check(Expr::col(Alias::new("price_revision")).gt(0)),
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
                        ColumnDef::new(Alias::new("first_source_version_id"))
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("second_source_version_id"))
                            .uuid()
                            .check(
                                Expr::col(Alias::new("second_source_version_id"))
                                    .is_null()
                                    .or(Expr::col(Alias::new("second_source_version_id"))
                                        .ne(Expr::col(Alias::new("first_source_version_id")))),
                            ),
                    )
                    .col(ColumnDef::new(Alias::new("profile_id")).uuid().not_null())
                    .col(
                        ColumnDef::new(Alias::new("base_cents"))
                            .integer()
                            .not_null()
                            .check(Expr::col(Alias::new("base_cents")).gte(0)),
                    )
                    .col(
                        ColumnDef::new(Alias::new("second_suite_cents"))
                            .integer()
                            .not_null()
                            .check(Expr::col(Alias::new("second_suite_cents")).gte(0)),
                    )
                    .col(
                        ColumnDef::new(Alias::new("check_cents"))
                            .integer()
                            .not_null()
                            .check(Expr::col(Alias::new("check_cents")).gte(0)),
                    )
                    .col(
                        ColumnDef::new(Alias::new("check_cap"))
                            .integer()
                            .not_null()
                            .check(Expr::col(Alias::new("check_cap")).between(1, 8)),
                    )
                    .col(
                        ColumnDef::new(Alias::new("currency"))
                            .text()
                            .not_null()
                            .default("USD")
                            .check(Expr::col(Alias::new("currency")).eq("USD")),
                    )
                    .col(
                        ColumnDef::new(Alias::new("agreement_reference"))
                            .text()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Alias::new("created_by")).uuid().not_null())
                    .col(
                        ColumnDef::new(Alias::new("created_at"))
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("commercial_agreements"), Alias::new("app_id"))
                            .to(Alias::new("apps"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                Alias::new("commercial_agreements"),
                                Alias::new("first_source_version_id"),
                            )
                            .to(Alias::new("execution_definitions"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                Alias::new("commercial_agreements"),
                                Alias::new("second_source_version_id"),
                            )
                            .to(Alias::new("execution_definitions"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                Alias::new("commercial_agreements"),
                                Alias::new("profile_id"),
                            )
                            .to(Alias::new("execution_profiles"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                Alias::new("commercial_agreements"),
                                Alias::new("created_by"),
                            )
                            .to(Alias::new("users"), Alias::new("id")),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("commercial_one_active_agreement_per_app")
                    .unique()
                    .table(Alias::new("commercial_agreements"))
                    .col(Alias::new("app_id"))
                    .and_where(Expr::col(Alias::new("status")).eq("active"))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("commercial_agreements_app_history")
                    .table(Alias::new("commercial_agreements"))
                    .col(Alias::new("app_id"))
                    .col((Alias::new("created_at"), IndexOrder::Desc))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Alias::new("commercial_quotes"))
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Alias::new("agreement_id")).uuid().not_null())
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
                        ColumnDef::new(Alias::new("price_revision"))
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("amount_cents"))
                            .integer()
                            .not_null()
                            .check(Expr::col(Alias::new("amount_cents")).gte(0)),
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
                            .from(Alias::new("commercial_quotes"), Alias::new("agreement_id"))
                            .to(Alias::new("commercial_agreements"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("commercial_quotes"), Alias::new("app_id"))
                            .to(Alias::new("apps"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("commercial_quotes"), Alias::new("actor_id"))
                            .to(Alias::new("users"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("commercial_quotes"), Alias::new("build_id"))
                            .to(Alias::new("builds"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                Alias::new("commercial_quotes"),
                                Alias::new("source_version_id"),
                            )
                            .to(Alias::new("execution_definitions"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("commercial_quotes"), Alias::new("profile_id"))
                            .to(Alias::new("execution_profiles"), Alias::new("id")),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("commercial_quotes_app_actor")
                    .table(Alias::new("commercial_quotes"))
                    .col(Alias::new("app_id"))
                    .col(Alias::new("actor_id"))
                    .col(Alias::new("expires_at"))
                    .to_owned(),
            )
            .await?;

        let review_is_valid = Expr::col(Alias::new("state"))
            .eq("reserved")
            .and(Expr::col(Alias::new("reviewer_id")).is_null())
            .and(Expr::col(Alias::new("reviewed_at")).is_null())
            .or(Expr::col(Alias::new("state"))
                .ne("reserved")
                .and(Expr::col(Alias::new("reviewer_id")).is_not_null())
                .and(Expr::col(Alias::new("reviewed_at")).is_not_null()));
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("commercial_usage"))
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
                    .col(ColumnDef::new(Alias::new("agreement_id")).uuid().not_null())
                    .col(ColumnDef::new(Alias::new("app_id")).uuid().not_null())
                    .col(
                        ColumnDef::new(Alias::new("period_start"))
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("period_end"))
                            .timestamp_with_time_zone()
                            .not_null()
                            .check(
                                Expr::col(Alias::new("period_end"))
                                    .gt(Expr::col(Alias::new("period_start"))),
                            ),
                    )
                    .col(
                        ColumnDef::new(Alias::new("amount_cents"))
                            .integer()
                            .not_null()
                            .check(Expr::col(Alias::new("amount_cents")).gte(0)),
                    )
                    .col(
                        ColumnDef::new(Alias::new("state"))
                            .text()
                            .not_null()
                            .default("reserved")
                            .check(Expr::col(Alias::new("state")).is_in([
                                "reserved",
                                "delivered",
                                "credited",
                            ])),
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
                            .from(Alias::new("commercial_usage"), Alias::new("run_id"))
                            .to(Alias::new("execution_runs"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("commercial_usage"), Alias::new("quote_id"))
                            .to(Alias::new("commercial_quotes"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("commercial_usage"), Alias::new("agreement_id"))
                            .to(Alias::new("commercial_agreements"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("commercial_usage"), Alias::new("app_id"))
                            .to(Alias::new("apps"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("commercial_usage"), Alias::new("reviewer_id"))
                            .to(Alias::new("users"), Alias::new("id")),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("commercial_usage_agreement")
                    .table(Alias::new("commercial_usage"))
                    .col(Alias::new("agreement_id"))
                    .col(Alias::new("state"))
                    .col(Alias::new("created_at"))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Alias::new("commercial_audit"))
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Alias::new("agreement_id")).uuid().not_null())
                    .col(ColumnDef::new(Alias::new("run_id")).uuid())
                    .col(ColumnDef::new(Alias::new("actor_id")).uuid().not_null())
                    .col(
                        ColumnDef::new(Alias::new("action"))
                            .text()
                            .not_null()
                            .check(Expr::col(Alias::new("action")).is_in([
                                "activated",
                                "paused",
                                "ended",
                                "delivered",
                                "credited",
                            ])),
                    )
                    .col(ColumnDef::new(Alias::new("reason")).text().not_null())
                    .col(
                        ColumnDef::new(Alias::new("created_at"))
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("commercial_audit"), Alias::new("agreement_id"))
                            .to(Alias::new("commercial_agreements"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("commercial_audit"), Alias::new("run_id"))
                            .to(Alias::new("execution_runs"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("commercial_audit"), Alias::new("actor_id"))
                            .to(Alias::new("users"), Alias::new("id")),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("commercial_audit_agreement")
                    .table(Alias::new("commercial_audit"))
                    .col(Alias::new("agreement_id"))
                    .col(Alias::new("created_at"))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Alias::new("commercial_pilot_requests"))
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Alias::new("app_id")).uuid().not_null())
                    .col(ColumnDef::new(Alias::new("actor_id")).uuid().not_null())
                    .col(
                        ColumnDef::new(Alias::new("coverage_note"))
                            .text()
                            .not_null()
                            .check(
                                Func::char_length(Expr::col(Alias::new("coverage_note")))
                                    .between(1, 500),
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
                            .from(
                                Alias::new("commercial_pilot_requests"),
                                Alias::new("app_id"),
                            )
                            .to(Alias::new("apps"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                Alias::new("commercial_pilot_requests"),
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
                    .name("commercial_one_pilot_request_per_app")
                    .unique()
                    .table(Alias::new("commercial_pilot_requests"))
                    .col(Alias::new("app_id"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            "commercial_pilot_requests",
            "commercial_audit",
            "commercial_usage",
            "commercial_quotes",
            "commercial_agreements",
        ] {
            manager
                .drop_table(Table::drop().table(Alias::new(table)).to_owned())
                .await?;
        }
        Ok(())
    }
}
