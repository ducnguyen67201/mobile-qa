//! Replace password identities without dropping apps, memberships, or build history.
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
                        ColumnDef::new(Alias::new("google_subject"))
                            .text()
                            .unique_key(),
                    )
                    .drop_column(Alias::new("password_hash"))
                    .to_owned(),
            )
            .await?;
        manager
            .exec_stmt(
                Query::update()
                    .table(Alias::new("sessions"))
                    .value(Alias::new("revoked_at"), Expr::current_timestamp())
                    .and_where(Expr::col(Alias::new("revoked_at")).is_null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("google_login_challenges"))
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Alias::new("nonce")).text().not_null())
                    .col(ColumnDef::new(Alias::new("cookie_hash")).text().not_null())
                    .col(
                        ColumnDef::new(Alias::new("expires_at"))
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("google_login_challenges_expiry")
                    .table(Alias::new("google_login_challenges"))
                    .col(Alias::new("expires_at"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(Alias::new("google_login_challenges"))
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("users"))
                    .drop_column(Alias::new("google_subject"))
                    .add_column(
                        ColumnDef::new(Alias::new("password_hash"))
                            .text()
                            .not_null()
                            .default("!disabled"),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .exec_stmt(
                Query::update()
                    .table(Alias::new("sessions"))
                    .value(Alias::new("revoked_at"), Expr::current_timestamp())
                    .and_where(Expr::col(Alias::new("revoked_at")).is_null())
                    .to_owned(),
            )
            .await
    }
}
