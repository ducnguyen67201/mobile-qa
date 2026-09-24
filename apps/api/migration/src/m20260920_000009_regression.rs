//! New runs pin comparison inputs; old session activity remains unclassified.
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut baseline_key = TableForeignKey::new();
        baseline_key
            .from_tbl(Alias::new("execution_runs"))
            .from_col(Alias::new("baseline_run_id"))
            .to_tbl(Alias::new("execution_runs"))
            .to_col(Alias::new("id"));
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("execution_runs"))
                    .modify_column(ColumnDef::new(Alias::new("plan_id")).uuid().null())
                    .add_column(ColumnDef::new(Alias::new("baseline_run_id")).uuid())
                    .add_column(ColumnDef::new(Alias::new("comparison")).json_binary())
                    .add_column(
                        ColumnDef::new(Alias::new("comparison_not_self"))
                            .boolean()
                            .generated(
                                Expr::col(Alias::new("baseline_run_id"))
                                    .is_null()
                                    .or(Expr::col(Alias::new("baseline_run_id"))
                                        .ne(Expr::col(Alias::new("id")))),
                                true,
                            )
                            .check((
                                "comparison_not_self",
                                Expr::col(Alias::new("comparison_not_self")).eq(true),
                            )),
                    )
                    .add_foreign_key(&baseline_key)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("phone_tasks"))
                    .add_column(
                        ColumnDef::new(Alias::new("purpose"))
                            .text()
                            .check(Expr::col(Alias::new("purpose")).is_in(["trial", "manual"])),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("execution_runs_app_created")
                    .table(Alias::new("execution_runs"))
                    .col(Alias::new("app_id"))
                    .col((Alias::new("created_at"), IndexOrder::Desc))
                    .col((Alias::new("id"), IndexOrder::Desc))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "Forward-only: saved-case runs cannot be converted into legacy plans".into(),
        ))
    }
}
