//! Separate Google registration from approval to create workspaces; preserve existing access.
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("users"))
                    .add_column(
                        ColumnDef::new(Alias::new("approval_status"))
                            .text()
                            .not_null()
                            .default("approved")
                            .check(
                                Expr::col(Alias::new("approval_status"))
                                    .is_in(["pending", "approved"]),
                            ),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("users"))
                    .modify_column(
                        ColumnDef::new(Alias::new("approval_status"))
                            .text()
                            .not_null()
                            .default("pending"),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("users"))
                    .drop_column(Alias::new("approval_status"))
                    .to_owned(),
            )
            .await
    }
}
