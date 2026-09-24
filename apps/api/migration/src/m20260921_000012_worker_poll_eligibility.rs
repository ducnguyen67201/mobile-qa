//! Keep execution and phone claim advertisements distinct without rewriting earlier migrations.
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("execution_workers"))
                    .add_column(ColumnDef::new(Alias::new("execution_protocol_version")).integer())
                    .add_column(
                        ColumnDef::new(Alias::new("execution_model_capabilities")).json_binary(),
                    )
                    .add_column(
                        ColumnDef::new(Alias::new("execution_last_seen_at"))
                            .timestamp_with_time_zone(),
                    )
                    .add_column(ColumnDef::new(Alias::new("phone_protocol_version")).integer())
                    .add_column(
                        ColumnDef::new(Alias::new("phone_model_capabilities")).json_binary(),
                    )
                    .add_column(
                        ColumnDef::new(Alias::new("phone_last_seen_at")).timestamp_with_time_zone(),
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
                    .drop_column(Alias::new("phone_last_seen_at"))
                    .drop_column(Alias::new("phone_model_capabilities"))
                    .drop_column(Alias::new("phone_protocol_version"))
                    .drop_column(Alias::new("execution_last_seen_at"))
                    .drop_column(Alias::new("execution_model_capabilities"))
                    .drop_column(Alias::new("execution_protocol_version"))
                    .to_owned(),
            )
            .await
    }
}
