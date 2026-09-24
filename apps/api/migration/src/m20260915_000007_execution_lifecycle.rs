//! Preserve original execution evidence separately from later operational recovery.
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("execution_preflight_receipts"))
                    .col(ColumnDef::new(Alias::new("attempt_id")).uuid().not_null())
                    .col(
                        ColumnDef::new(Alias::new("generation"))
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Alias::new("digest")).text().not_null())
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
                    .primary_key(
                        Index::create()
                            .col(Alias::new("attempt_id"))
                            .col(Alias::new("generation")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                Alias::new("execution_preflight_receipts"),
                                Alias::new("attempt_id"),
                            )
                            .to(Alias::new("execution_attempts"), Alias::new("id")),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("execution_recovery_events"))
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Alias::new("attempt_id")).uuid().not_null())
                    .col(ColumnDef::new(Alias::new("actor_id")).uuid().not_null())
                    .col(
                        ColumnDef::new(Alias::new("evidence_reference"))
                            .text()
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
                            .from(
                                Alias::new("execution_recovery_events"),
                                Alias::new("attempt_id"),
                            )
                            .to(Alias::new("execution_attempts"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                Alias::new("execution_recovery_events"),
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
                    .name("execution_recovery_attempt")
                    .table(Alias::new("execution_recovery_events"))
                    .col(Alias::new("attempt_id"))
                    .col(Alias::new("created_at"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in ["execution_recovery_events", "execution_preflight_receipts"] {
            manager
                .drop_table(Table::drop().table(Alias::new(table)).to_owned())
                .await?;
        }
        Ok(())
    }
}
