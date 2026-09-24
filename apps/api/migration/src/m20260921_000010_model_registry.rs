//! Immutable nonsecret model definitions and worker capability heartbeats.
use sea_orm_migration::{
    prelude::*,
    sea_query::extension::postgres::{PgBinOper, PgExpr},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("model_definitions"))
                    .col(
                        ColumnDef::new(Alias::new("key")).text().not_null().check(
                            Expr::col(Alias::new("key"))
                                .binary(PgBinOper::Regex, "^[a-z0-9][a-z0-9._-]{0,99}$"),
                        ),
                    )
                    .col(
                        ColumnDef::new(Alias::new("revision"))
                            .integer()
                            .not_null()
                            .check(Expr::col(Alias::new("revision")).gt(0)),
                    )
                    .col(
                        ColumnDef::new(Alias::new("payload"))
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("created_at"))
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Alias::new("retired_at")).timestamp_with_time_zone())
                    .primary_key(
                        Index::create()
                            .col(Alias::new("key"))
                            .col(Alias::new("revision")),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("model_definitions_active_provider_model")
                    .table(Alias::new("model_definitions"))
                    .col(Expr::col(Alias::new("payload")).cast_json_field("provider"))
                    .col(Expr::col(Alias::new("payload")).cast_json_field("provider_model"))
                    .and_where(Expr::col(Alias::new("retired_at")).is_null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("execution_workers"))
                    .add_column(ColumnDef::new(Alias::new("model_capabilities")).json_binary())
                    .add_column(
                        ColumnDef::new(Alias::new("model_last_seen_at")).timestamp_with_time_zone(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("execution_workers"))
                    .drop_column(Alias::new("model_last_seen_at"))
                    .drop_column(Alias::new("model_capabilities"))
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(Alias::new("model_definitions"))
                    .to_owned(),
            )
            .await
    }
}
