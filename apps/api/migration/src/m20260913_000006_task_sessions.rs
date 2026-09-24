//! Interactive sessions share the existing physical device reservation namespace.
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let twenty_minutes = Func::cust(Alias::new("make_interval")).args([
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            20.into(),
            0.into(),
        ]);
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("phone_sessions"))
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Alias::new("app_id")).uuid().not_null())
                    .col(ColumnDef::new(Alias::new("creator_id")).uuid().not_null())
                    .col(ColumnDef::new(Alias::new("build_id")).uuid().not_null())
                    .col(ColumnDef::new(Alias::new("profile_id")).uuid().not_null())
                    .col(
                        ColumnDef::new(Alias::new("payload"))
                            .json_binary()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Alias::new("fingerprint")).text().not_null())
                    .col(ColumnDef::new(Alias::new("build_sha256")).text().not_null())
                    .col(
                        ColumnDef::new(Alias::new("build_bytes"))
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Alias::new("worker_id")).uuid())
                    .col(ColumnDef::new(Alias::new("claim_id")).uuid())
                    .col(ColumnDef::new(Alias::new("lease_hash")).text())
                    .col(ColumnDef::new(Alias::new("expires_at")).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(Alias::new("created_at"))
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Alias::new("deadline"))
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp().add(twenty_minutes)),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("phone_sessions"), Alias::new("app_id"))
                            .to(Alias::new("apps"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("phone_sessions"), Alias::new("creator_id"))
                            .to(Alias::new("users"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("phone_sessions"), Alias::new("build_id"))
                            .to(Alias::new("builds"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("phone_sessions"), Alias::new("profile_id"))
                            .to(Alias::new("execution_profiles"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("phone_sessions"), Alias::new("worker_id"))
                            .to(Alias::new("execution_workers"), Alias::new("id")),
                    )
                    .index(
                        Index::create()
                            .unique()
                            .col(Alias::new("worker_id"))
                            .col(Alias::new("claim_id")),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("phone_tasks"))
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Alias::new("session_id")).uuid().not_null())
                    .col(ColumnDef::new(Alias::new("fingerprint")).text().not_null())
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
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("phone_tasks"), Alias::new("session_id"))
                            .to(Alias::new("phone_sessions"), Alias::new("id")),
                    )
                    .to_owned(),
            )
            .await?;
        let mut session_key = TableForeignKey::new();
        session_key
            .from_tbl(Alias::new("execution_reservations"))
            .from_col(Alias::new("session_id"))
            .to_tbl(Alias::new("phone_sessions"))
            .to_col(Alias::new("id"));
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("execution_reservations"))
                    .modify_column(ColumnDef::new(Alias::new("attempt_id")).uuid().null())
                    .add_column(ColumnDef::new(Alias::new("session_id")).uuid())
                    .add_column(
                        ColumnDef::new(Alias::new("reservation_owner"))
                            .boolean()
                            .generated(
                                Expr::col(Alias::new("attempt_id"))
                                    .is_null()
                                    .and(Expr::col(Alias::new("session_id")).is_not_null())
                                    .or(Expr::col(Alias::new("attempt_id"))
                                        .is_not_null()
                                        .and(Expr::col(Alias::new("session_id")).is_null())),
                                true,
                            )
                            .check((
                                "reservation_owner",
                                Expr::col(Alias::new("reservation_owner")).eq(true),
                            )),
                    )
                    .add_foreign_key(&session_key)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("phone_sessions_app")
                    .table(Alias::new("phone_sessions"))
                    .col(Alias::new("app_id"))
                    .col(Alias::new("created_at"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .exec_stmt(
                Query::delete()
                    .from_table(Alias::new("execution_reservations"))
                    .and_where(Expr::col(Alias::new("session_id")).is_not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("execution_reservations"))
                    .drop_column(Alias::new("reservation_owner"))
                    .drop_column(Alias::new("session_id"))
                    .modify_column(ColumnDef::new(Alias::new("attempt_id")).uuid().not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(Alias::new("phone_tasks")).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Alias::new("phone_sessions")).to_owned())
            .await
    }
}
