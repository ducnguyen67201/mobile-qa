//! Additive large-build metadata, resumable part receipts and restartable artifact work.
use sea_orm_migration::{prelude::*, sea_query::extension::postgres::PgBinOper};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("build_uploads"))
                    .drop_constraint(Alias::new("build_uploads_expected_size_check"))
                    .add_column(
                        // SeaQuery cannot add a standalone CHECK to an existing table. A
                        // generated guard keeps the invariant declarative without raw DDL.
                        ColumnDef::new(Alias::new("expected_size_within_limit"))
                            .boolean()
                            .generated(
                                Expr::col(Alias::new("expected_size")).gt(0).and(
                                    Expr::col(Alias::new("expected_size")).lte(2_147_483_648_i64),
                                ),
                                true,
                            )
                            .check((
                                "build_uploads_expected_size_check",
                                Expr::col(Alias::new("expected_size_within_limit")).eq(true),
                            )),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("phone_sessions"))
                    .modify_column(
                        ColumnDef::new(Alias::new("build_bytes"))
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Alias::new("artifact_multipart"))
                    .col(
                        ColumnDef::new(Alias::new("upload_id"))
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("storage_key"))
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Alias::new("provider_upload_id")).text())
                    .col(ColumnDef::new(Alias::new("state")).text().not_null().check(
                        Expr::col(Alias::new("state")).is_in([
                            "initiating",
                            "uploading",
                            "completing",
                            "sealed",
                            "aborted",
                        ]),
                    ))
                    .col(ColumnDef::new(Alias::new("operation_id")).uuid().not_null())
                    .col(ColumnDef::new(Alias::new("lease_until")).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(Alias::new("created_at"))
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("artifact_multipart"), Alias::new("upload_id"))
                            .to(Alias::new("build_uploads"), Alias::new("id")),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Alias::new("artifact_parts"))
                    .col(ColumnDef::new(Alias::new("upload_id")).uuid().not_null())
                    .col(
                        ColumnDef::new(Alias::new("part_number"))
                            .integer()
                            .not_null()
                            .check(Expr::col(Alias::new("part_number")).between(1, 128)),
                    )
                    .col(
                        ColumnDef::new(Alias::new("byte_size"))
                            .integer()
                            .not_null()
                            .check(Expr::col(Alias::new("byte_size")).between(1, 16_777_216)),
                    )
                    .col(
                        ColumnDef::new(Alias::new("sha256"))
                            .text()
                            .not_null()
                            .check(
                                Expr::col(Alias::new("sha256"))
                                    .binary(PgBinOper::Regex, "^[a-f0-9]{64}$"),
                            ),
                    )
                    .col(ColumnDef::new(Alias::new("etag")).text())
                    .primary_key(
                        Index::create()
                            .col(Alias::new("upload_id"))
                            .col(Alias::new("part_number")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("artifact_parts"), Alias::new("upload_id"))
                            .to(Alias::new("artifact_multipart"), Alias::new("upload_id")),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Alias::new("artifact_jobs"))
                    .col(ColumnDef::new(Alias::new("upload_id")).uuid().not_null())
                    .col(
                        ColumnDef::new(Alias::new("phase"))
                            .text()
                            .not_null()
                            .check(Expr::col(Alias::new("phase")).is_in(["seal", "validate"])),
                    )
                    .col(
                        ColumnDef::new(Alias::new("state"))
                            .text()
                            .not_null()
                            .default("pending")
                            .check(Expr::col(Alias::new("state")).is_in([
                                "pending",
                                "processing",
                                "done",
                                "error",
                            ])),
                    )
                    .col(ColumnDef::new(Alias::new("attempt_id")).uuid())
                    .col(ColumnDef::new(Alias::new("lease_until")).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(Alias::new("attempts"))
                            .integer()
                            .not_null()
                            .default(0)
                            .check(Expr::col(Alias::new("attempts")).between(0, 5)),
                    )
                    .col(ColumnDef::new(Alias::new("reason_code")).text())
                    .col(
                        ColumnDef::new(Alias::new("updated_at"))
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .primary_key(
                        Index::create()
                            .col(Alias::new("upload_id"))
                            .col(Alias::new("phase")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("artifact_jobs"), Alias::new("upload_id"))
                            .to(Alias::new("build_uploads"), Alias::new("id")),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("artifact_jobs_pending")
                    .table(Alias::new("artifact_jobs"))
                    .col(Alias::new("state"))
                    .col(Alias::new("lease_until"))
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Metadata may already hold > i32::MAX. Operational rollback disables new intake.
        Err(DbErr::Custom("Large artifact schema is forward-only; disable intake instead of narrowing retained build metadata".into()))
    }
}
