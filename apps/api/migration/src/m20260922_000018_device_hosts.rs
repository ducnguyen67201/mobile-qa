//! Fixed-host capacity, explicit slot grants and a durable power-operation fence.
use sea_orm_migration::{prelude::*, sea_query::extension::postgres::PgExpr};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("device_pools"))
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("policy"))
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("control_token_hash"))
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("created_at"))
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("device_hosts"))
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("pool_id"))
                            .uuid()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("instance_id"))
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Alias::new("token_hash")).text().not_null())
                    .col(
                        ColumnDef::new(Alias::new("revoked"))
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Alias::new("toolchain_digest"))
                            .text()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Alias::new("boot_id")).uuid())
                    .col(
                        ColumnDef::new(Alias::new("generation"))
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Alias::new("state"))
                            .text()
                            .not_null()
                            .default("stopped")
                            .check(Expr::col(Alias::new("state")).is_in([
                                "stopped",
                                "starting",
                                "ready",
                                "draining",
                                "stop_committed",
                                "stopping",
                                "quarantined",
                            ])),
                    )
                    .col(
                        ColumnDef::new(Alias::new("observed_power"))
                            .text()
                            .not_null()
                            .default("unknown")
                            .check(
                                Expr::col(Alias::new("observed_power")).is_in([
                                    "unknown", "stopped", "pending", "running", "stopping",
                                ]),
                            ),
                    )
                    .col(
                        ColumnDef::new(Alias::new("control_version"))
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Alias::new("heartbeat_at")).timestamp_with_time_zone())
                    .col(ColumnDef::new(Alias::new("clean_at")).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(Alias::new("last_demand_at"))
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Alias::new("startup_started_at")).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("cache_bytes"))
                            .big_integer()
                            .not_null()
                            .default(0)
                            .check(Expr::col(Alias::new("cache_bytes")).gte(0)),
                    )
                    .col(ColumnDef::new(Alias::new("reason")).text())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("device_hosts"), Alias::new("pool_id"))
                            .to(Alias::new("device_pools"), Alias::new("id")),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("device_slots"))
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Alias::new("host_id")).uuid().not_null())
                    .col(
                        ColumnDef::new(Alias::new("slot_index"))
                            .integer()
                            .not_null()
                            .check(Expr::col(Alias::new("slot_index")).between(0, 15)),
                    )
                    .col(
                        ColumnDef::new(Alias::new("definition"))
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("state"))
                            .text()
                            .not_null()
                            .default("offline")
                            .check(Expr::col(Alias::new("state")).is_in([
                                "offline",
                                "preparing",
                                "idle",
                                "leased",
                                "cleaning",
                                "quarantined",
                            ])),
                    )
                    .col(ColumnDef::new(Alias::new("emulator_boot_id")).uuid())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("device_slots"), Alias::new("host_id"))
                            .to(Alias::new("device_hosts"), Alias::new("id")),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("device_slots_host_slot_index")
                    .unique()
                    .table(Alias::new("device_slots"))
                    .col(Alias::new("host_id"))
                    .col(Alias::new("slot_index"))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("device_slot_physical_identity")
                    .unique()
                    .table(Alias::new("device_slots"))
                    .col(Expr::col(Alias::new("definition")).cast_json_field("device_identity"))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Alias::new("device_slot_bindings"))
                    .col(ColumnDef::new(Alias::new("slot_id")).uuid().not_null())
                    .col(ColumnDef::new(Alias::new("app_id")).uuid().not_null())
                    .col(ColumnDef::new(Alias::new("profile_id")).uuid().not_null())
                    .primary_key(
                        Index::create()
                            .col(Alias::new("slot_id"))
                            .col(Alias::new("app_id"))
                            .col(Alias::new("profile_id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("device_slot_bindings"), Alias::new("slot_id"))
                            .to(Alias::new("device_slots"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("device_slot_bindings"), Alias::new("app_id"))
                            .to(Alias::new("apps"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("device_slot_bindings"), Alias::new("profile_id"))
                            .to(Alias::new("execution_profiles"), Alias::new("id")),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("execution_workers"))
                    .add_column(ColumnDef::new(Alias::new("host_slot_id")).uuid())
                    .add_column(
                        ColumnDef::new(Alias::new("host_generation"))
                            .integer()
                            .check((
                                "execution_host_grant_pair",
                                Expr::col(Alias::new("host_slot_id"))
                                    .is_null()
                                    .eq(Expr::col(Alias::new("host_generation")).is_null()),
                            )),
                    )
                    .add_foreign_key(
                        TableForeignKey::new()
                            .from_tbl(Alias::new("execution_workers"))
                            .from_col(Alias::new("host_slot_id"))
                            .to_tbl(Alias::new("device_slots"))
                            .to_col(Alias::new("id")),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("execution_host_grants")
                    .table(Alias::new("execution_workers"))
                    .col(Alias::new("host_slot_id"))
                    .col(Alias::new("host_generation"))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Alias::new("device_power_operations"))
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Alias::new("host_id")).uuid().not_null())
                    .col(
                        ColumnDef::new(Alias::new("action"))
                            .text()
                            .not_null()
                            .check(Expr::col(Alias::new("action")).is_in(["start", "commit_stop"])),
                    )
                    .col(
                        ColumnDef::new(Alias::new("generation"))
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("control_version"))
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("created_at"))
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Alias::new("completed_at")).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("device_power_operations"), Alias::new("host_id"))
                            .to(Alias::new("device_hosts"), Alias::new("id")),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("device_one_pending_operation")
                    .unique()
                    .table(Alias::new("device_power_operations"))
                    .col(Alias::new("host_id"))
                    .and_where(Expr::col(Alias::new("completed_at")).is_null())
                    .to_owned(),
            )
            .await?;

        create_receipt_table(manager, "device_control_receipts", true).await?;
        create_receipt_table(manager, "capacity_hint_receipts", false).await?;

        manager
            .create_table(
                Table::create()
                    .table(Alias::new("capacity_wake_outbox"))
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Alias::new("pool_id")).uuid().not_null())
                    .col(
                        ColumnDef::new(Alias::new("created_at"))
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Alias::new("next_attempt_at"))
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Alias::new("attempts"))
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Alias::new("delivered_at")).timestamp_with_time_zone())
                    .col(ColumnDef::new(Alias::new("lease_id")).uuid())
                    .col(ColumnDef::new(Alias::new("lease_until")).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("capacity_wake_outbox"), Alias::new("pool_id"))
                            .to(Alias::new("device_pools"), Alias::new("id")),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("capacity_pending_wake")
                    .table(Alias::new("capacity_wake_outbox"))
                    .col(Alias::new("next_attempt_at"))
                    .and_where(Expr::col(Alias::new("delivered_at")).is_null())
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom("Retain device ownership and operation records; disable capacity instead of rolling back this migration".into()))
    }
}

async fn create_receipt_table(
    manager: &SchemaManager<'_>,
    table: &'static str,
    fingerprint: bool,
) -> Result<(), DbErr> {
    let mut statement = Table::create();
    statement
        .table(Alias::new(table))
        .col(ColumnDef::new(Alias::new("pool_id")).uuid().not_null())
        .col(ColumnDef::new(Alias::new("request_id")).uuid().not_null());
    if fingerprint {
        statement.col(ColumnDef::new(Alias::new("fingerprint")).text().not_null());
    }
    statement
        .col(
            ColumnDef::new(Alias::new("created_at"))
                .timestamp_with_time_zone()
                .not_null()
                .default(Expr::current_timestamp()),
        )
        .primary_key(
            Index::create()
                .col(Alias::new("pool_id"))
                .col(Alias::new("request_id")),
        )
        .foreign_key(
            ForeignKey::create()
                .from(Alias::new(table), Alias::new("pool_id"))
                .to(Alias::new("device_pools"), Alias::new("id")),
        );
    manager.create_table(statement.to_owned()).await
}
