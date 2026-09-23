//! Real PostgreSQL and HTTP authority tests; no AWS, Android or model calls.
mod support;
use chrono::Duration;
use mobile_qa::{
    models::_entities::{
        apps as app_rows, capacity_wake_outbox, device_hosts as host_rows, device_slots,
        execution_attempts, execution_reservations, execution_runs, execution_workers,
    },
    services::{
        capacity_control as capacity, device_hosts as hosts,
        execution_store::{hash, json},
        scheduler, test_definitions, worker_auth,
    },
};
use mobile_qa_contracts::{
    device_hosts::*,
    execution::{ClaimRequest, ExecutionProfile},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QuerySelect, Set,
    TransactionTrait,
};
use support::*;

struct Pool {
    registration: PoolRegistration,
    host_token: String,
    control_token: String,
    boot: Uuid,
    actor: Uuid,
    owner: Login,
}
impl Pool {
    fn host_path(&self, suffix: &str) -> String {
        format!(
            "/api/internal/hosts/{}/{}",
            self.registration.host_id, suffix
        )
    }
    fn control_path(&self, suffix: &str) -> String {
        format!(
            "/api/internal/capacity/pools/{}{}",
            self.registration.pool_id, suffix
        )
    }
    fn host_auth(&self) -> String {
        format!("Bearer {}", self.host_token)
    }
    fn control_auth(&self) -> String {
        format!("Bearer {}", self.control_token)
    }
    fn heartbeat(&self, generation: i32) -> HostHeartbeatRequest {
        HostHeartbeatRequest {
            generation,
            boot_id: self.boot,
            slots: vec![SlotHeartbeat {
                slot_id: self.registration.slots[0].id,
                state: SlotState::Idle,
                emulator_boot_id: None,
            }],
            cache_bytes: 0,
        }
    }
    fn cleanup(&self, generation: i32) -> HostCleanupRequest {
        HostCleanupRequest {
            generation,
            boot_id: self.boot,
            processes_stopped: true,
            ports_released: true,
            journals_resolved: true,
        }
    }
    fn grant(&self, generation: i32) -> SlotGrantRequest {
        let b = &self.registration.bindings[0];
        SlotGrantRequest {
            generation,
            boot_id: self.boot,
            app_id: b.app_id,
            profile_id: b.profile_id,
        }
    }
}
async fn prepare(server: &TestServer, ctx: &AppContext) -> Pool {
    let owner = login(server, ctx).await;
    let mut app_input = create_input(owner.org);
    app_input.android_package = "ai.mobileqa.demo".into();
    let app = apps::create(ctx, owner.user, app_input).await.unwrap();
    let identity = Uuid::new_v4().to_string();
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/execution/comparison.json")).unwrap();
    let mut profile: ExecutionProfile =
        serde_json::from_value(fixture["manifest"]["profile"].clone()).unwrap();
    profile.id = Uuid::new_v4();
    profile.device_identity = identity.clone();
    profile.model = None;
    profile.max_apk_bytes = 2147483648;
    profile
        .execution_context
        .as_mut()
        .unwrap()
        .qualified_profile_id = profile.id;
    // The historical comparison snapshot predates fixed clean-start assertions.
    let start = &mut profile.execution_context.as_mut().unwrap().starting_checks[0];
    start.checkpoint_id = "preflight".into();
    start.text_filter.clear();
    start.required = true;
    test_definitions::register_profile(ctx, owner.user, app.id, profile.clone())
        .await
        .unwrap();
    let slot = SlotDefinition {
        id: Uuid::new_v4(),
        index: 0,
        device_identity: identity,
        system_image: "system-images;android-35;google_apis;x86_64".into(),
        memory_mb: 4096,
        cpu_cores: 2,
        console_port: 5554,
        adb_server_port: 5038,
        qualified: true,
        warm_qualified: false,
        qualification_reference: "synthetic-only".into(),
    };
    let pool = Pool {
        registration: PoolRegistration {
            pool_id: Uuid::new_v4(),
            host_id: Uuid::new_v4(),
            instance_id: format!("i-{}", &Uuid::new_v4().simple().to_string()[..17]),
            toolchain_digest: "a".repeat(64),
            policy: PoolPolicy {
                enabled: true,
                idle_seconds: 60,
                warm_target: 0,
                timezone: "UTC".into(),
                warm_windows: vec![],
            },
            bindings: vec![SlotBinding {
                slot_id: slot.id,
                app_id: app.id,
                profile_id: profile.id,
            }],
            slots: vec![slot],
        },
        host_token: loco_rs::hash::random_string(64),
        control_token: loco_rs::hash::random_string(64),
        boot: Uuid::new_v4(),
        actor: owner.user,
        owner,
    };
    hosts::provision(
        ctx,
        pool.actor,
        pool.registration.clone(),
        &pool.host_token,
        &pool.control_token,
    )
    .await
    .unwrap();
    pool
}
async fn boot(server: &TestServer, pool: &Pool) -> HostStatus {
    let response = server
        .post(&pool.host_path("register"))
        .add_header("authorization", pool.host_auth())
        .json(&HostRegisterRequest {
            version: 1,
            boot_id: pool.boot,
            toolchain_digest: pool.registration.toolchain_digest.clone(),
        })
        .await;
    response.assert_status_ok();
    let host = response.json::<HostStatus>();
    let response = server
        .post(&pool.host_path("heartbeat"))
        .add_header("authorization", pool.host_auth())
        .json(&pool.heartbeat(host.generation))
        .await;
    response.assert_status_ok();
    response.json()
}
async fn action(
    server: &TestServer,
    pool: &Pool,
    snapshot: &CapacitySnapshot,
    action: CapacityAction,
    power: Option<ObservedPower>,
) -> TestResponse {
    server
        .post(&pool.control_path("/actions"))
        .add_header("authorization", pool.control_auth())
        .json(&CapacityActionRequest {
            request_id: Uuid::new_v4(),
            action,
            host_id: pool.registration.host_id,
            expected_control_version: snapshot.control_version,
            observed_power: power,
            operation_id: snapshot.current_operation.as_ref().map(|op| op.id),
            reason: None,
        })
        .await
}
async fn snap(ctx: &AppContext, pool: &Pool) -> CapacitySnapshot {
    capacity::snapshot(&ctx.db, pool.registration.pool_id)
        .await
        .unwrap()
}
async fn make_idle(ctx: &AppContext, pool: &Pool) {
    host_rows::Entity::update_many()
        .col_expr(
            host_rows::Column::LastDemandAt,
            sea_orm::sea_query::Expr::value(Utc::now() - Duration::minutes(2)),
        )
        .filter(host_rows::Column::Id.eq(pool.registration.host_id))
        .exec(&ctx.db)
        .await
        .unwrap();
}

#[tokio::test]
async fn host_credentials_are_scoped_and_boot_grants_are_fenced() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let mut pool = prepare(&server, &ctx).await;
        server
            .get(&pool.control_path(""))
            .add_header("authorization", pool.host_auth())
            .await
            .assert_status_unauthorized();
        server
            .post(&pool.host_path("register"))
            .add_header("authorization", pool.control_auth())
            .json(&HostRegisterRequest {
                version: 1,
                boot_id: pool.boot,
                toolchain_digest: "a".repeat(64),
            })
            .await
            .assert_status_unauthorized();
        let h = boot(&server, &pool).await;
        assert_eq!(h.state, HostState::Ready);
        assert_eq!(h.generation, 1);
        let again = boot(&server, &pool).await;
        assert_eq!(again.generation, h.generation);
        let grant_path = pool.host_path(&format!("slots/{}/grant", pool.registration.slots[0].id));
        let mut foreign = pool.grant(h.generation);
        foreign.profile_id = Uuid::new_v4();
        server
            .post(&grant_path)
            .add_header("authorization", pool.host_auth())
            .json(&foreign)
            .await
            .assert_status_not_found();
        let response = server
            .post(&grant_path)
            .add_header("authorization", pool.host_auth())
            .json(&pool.grant(h.generation))
            .await;
        response.assert_status_ok();
        let grant = response.json::<SlotGrantResponse>();
        let claim = ClaimRequest {
            version: 6,
            claim_id: Uuid::new_v4(),
            profile_id: grant.profile_id,
            model_capabilities: None,
        };
        // The tenant-neutral host token never becomes a worker/build credential.
        server
            .post("/api/worker/claims")
            .add_header("authorization", pool.host_auth())
            .json(&claim)
            .await
            .assert_status_unauthorized();
        let worker = worker_auth::Worker {
            id: grant.worker_id,
            app_id: grant.app_id,
            profile_id: grant.profile_id,
        };
        assert!(scheduler::claim(&ctx, &worker, claim.clone())
            .await
            .unwrap()
            .lease
            .is_none());
        let old_heartbeat = pool.heartbeat(h.generation);
        pool.boot = Uuid::new_v4();
        let next = boot(&server, &pool).await;
        assert_eq!(next.generation, 2);
        server
            .post("/api/worker/claims")
            .add_header("authorization", format!("Bearer {}", grant.worker_token))
            .json(&claim)
            .await
            .assert_status_unauthorized();
        server
            .post(&pool.host_path("heartbeat"))
            .add_header("authorization", pool.host_auth())
            .json(&old_heartbeat)
            .await
            .assert_status_conflict();
        host_rows::Entity::update_many()
            .col_expr(
                host_rows::Column::HeartbeatAt,
                sea_orm::sea_query::Expr::value(Some(Utc::now() - Duration::minutes(2))),
            )
            .filter(host_rows::Column::Id.eq(pool.registration.host_id))
            .exec(&ctx.db)
            .await
            .unwrap();
        server
            .post(&grant_path)
            .add_header("authorization", pool.host_auth())
            .json(&pool.grant(next.generation))
            .await
            .assert_status_conflict();
        let hint = ConsumeHintRequest {
            request_id: Uuid::new_v4(),
            timestamp: Utc::now().timestamp(),
        };
        let accepted = server
            .post(&pool.control_path("/hints"))
            .add_header("authorization", pool.control_auth())
            .json(&hint)
            .await;
        accepted.assert_status_ok();
        assert!(accepted.json::<HintReceipt>().accepted);
        server
            .post(&pool.control_path("/hints"))
            .add_header("authorization", pool.control_auth())
            .json(&hint)
            .await
            .assert_status_conflict();
        for timestamp in [i64::MIN, i64::MAX] {
            server
                .post(&pool.control_path("/hints"))
                .add_header("authorization", pool.control_auth())
                .json(&ConsumeHintRequest {
                    request_id: Uuid::new_v4(),
                    timestamp,
                })
                .await
                .assert_status_conflict();
        }
    })
    .await;
}

#[tokio::test]
async fn drain_cancel_and_committed_stop_keep_authority_until_observed_completion() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let pool = prepare(&server, &ctx).await;
        let h = boot(&server, &pool).await;
        let response = action(
            &server,
            &pool,
            &snap(&ctx, &pool).await,
            CapacityAction::Observe,
            Some(ObservedPower::Running),
        )
        .await;
        response.assert_status_ok();
        // Grace is based on last demand/occupancy, not on an empty queue sample.
        action(
            &server,
            &pool,
            &snap(&ctx, &pool).await,
            CapacityAction::Drain,
            None,
        )
        .await
        .assert_status_conflict();
        make_idle(&ctx, &pool).await;
        action(
            &server,
            &pool,
            &snap(&ctx, &pool).await,
            CapacityAction::Drain,
            None,
        )
        .await
        .assert_status_ok();
        let b = &pool.registration.bindings[0];
        let tx = ctx.db.begin().await.unwrap();
        app_rows::Entity::find_by_id(b.app_id)
            .lock_exclusive()
            .one(&tx)
            .await
            .unwrap()
            .unwrap();
        capacity::enqueue(&tx, b.app_id, b.profile_id)
            .await
            .unwrap();
        assert_eq!(
            capacity::snapshot(&tx, pool.registration.pool_id)
                .await
                .unwrap()
                .state,
            HostState::Ready
        );
        tx.commit().await.unwrap();
        server
            .post(&pool.host_path("cleanup"))
            .add_header("authorization", pool.host_auth())
            .json(&pool.cleanup(h.generation))
            .await
            .assert_status_conflict();
        let count = capacity_wake_outbox::Entity::find()
            .filter(capacity_wake_outbox::Column::PoolId.eq(pool.registration.pool_id))
            .count(&ctx.db)
            .await
            .unwrap();
        assert_eq!(count, 1);
        make_idle(&ctx, &pool).await;
        action(
            &server,
            &pool,
            &snap(&ctx, &pool).await,
            CapacityAction::Drain,
            None,
        )
        .await
        .assert_status_ok();
        action(
            &server,
            &pool,
            &snap(&ctx, &pool).await,
            CapacityAction::CommitStop,
            None,
        )
        .await
        .assert_status_conflict();
        server
            .post(&pool.host_path("cleanup"))
            .add_header("authorization", pool.host_auth())
            .json(&pool.cleanup(h.generation))
            .await
            .assert_status_ok();
        let before = snap(&ctx, &pool).await;
        let commit = CapacityActionRequest {
            request_id: Uuid::new_v4(),
            action: CapacityAction::CommitStop,
            host_id: pool.registration.host_id,
            expected_control_version: before.control_version,
            observed_power: None,
            operation_id: None,
            reason: None,
        };
        let response = server
            .post(&pool.control_path("/actions"))
            .add_header("authorization", pool.control_auth())
            .json(&commit)
            .await;
        response.assert_status_ok();
        let stopped = response.json::<CapacitySnapshot>();
        assert_eq!(stopped.state, HostState::StopCommitted);
        assert!(stopped.current_operation.is_some());
        // Identical replay returns current authority without adding a second power operation.
        server
            .post(&pool.control_path("/actions"))
            .add_header("authorization", pool.control_auth())
            .json(&commit)
            .await
            .assert_status_ok();
        let tx = ctx.db.begin().await.unwrap();
        capacity::enqueue(&tx, b.app_id, b.profile_id)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        assert_eq!(snap(&ctx, &pool).await.state, HostState::StopCommitted);
        server
            .post(&pool.host_path("register"))
            .add_header("authorization", pool.host_auth())
            .json(&HostRegisterRequest {
                version: 1,
                boot_id: Uuid::new_v4(),
                toolchain_digest: "a".repeat(64),
            })
            .await
            .assert_status_conflict();
        action(
            &server,
            &pool,
            &snap(&ctx, &pool).await,
            CapacityAction::RecordPower,
            Some(ObservedPower::Stopping),
        )
        .await
        .assert_status_ok();
        let stopping = snap(&ctx, &pool).await;
        assert_eq!(stopping.state, HostState::Stopping);
        assert!(stopping.current_operation.is_some());
        action(
            &server,
            &pool,
            &stopping,
            CapacityAction::RecordPower,
            Some(ObservedPower::Stopped),
        )
        .await
        .assert_status_ok();
        let done = snap(&ctx, &pool).await;
        assert_eq!(done.state, HostState::Stopped);
        assert!(done.current_operation.is_none());
    })
    .await;
}

#[tokio::test]
async fn uncertain_cleanup_is_quarantined_and_cannot_be_cleared_by_reboot() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let mut pool = prepare(&server, &ctx).await;
        let h = boot(&server, &pool).await;
        action(
            &server,
            &pool,
            &snap(&ctx, &pool).await,
            CapacityAction::Observe,
            Some(ObservedPower::Running),
        )
        .await
        .assert_status_ok();
        make_idle(&ctx, &pool).await;
        action(
            &server,
            &pool,
            &snap(&ctx, &pool).await,
            CapacityAction::Drain,
            None,
        )
        .await
        .assert_status_ok();
        let mut receipt = pool.cleanup(h.generation);
        receipt.journals_resolved = false;
        let r = server
            .post(&pool.host_path("cleanup"))
            .add_header("authorization", pool.host_auth())
            .json(&receipt)
            .await;
        r.assert_status_ok();
        assert_eq!(r.json::<HostStatus>().state, HostState::Quarantined);
        action(
            &server,
            &pool,
            &snap(&ctx, &pool).await,
            CapacityAction::CommitStop,
            None,
        )
        .await
        .assert_status_conflict();
        pool.boot = Uuid::new_v4();
        let h = boot(&server, &pool).await;
        assert_eq!(h.state, HostState::Quarantined);
    })
    .await;
}

#[tokio::test]
async fn grant_rotation_is_rechecked_after_waiting_for_host_lock() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let pool = prepare(&server, &ctx).await;
        let h = boot(&server, &pool).await;
        let grant = hosts::grant(
            &ctx,
            pool.registration.host_id,
            pool.registration.slots[0].id,
            pool.grant(h.generation),
        )
        .await
        .unwrap();
        let worker = worker_auth::Worker {
            id: grant.worker_id,
            app_id: grant.app_id,
            profile_id: grant.profile_id,
        };
        let rotation = ctx.db.begin().await.unwrap();
        hosts::host(&rotation, pool.registration.host_id, true)
            .await
            .unwrap();
        let claimant = ctx.db.begin().await.unwrap();
        {
            let future = hosts::claim_fence(&claimant, &worker, Uuid::new_v4());
            tokio::pin!(future);
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(30), &mut future)
                    .await
                    .is_err()
            );
            execution_workers::Entity::update_many()
                .col_expr(
                    execution_workers::Column::Revoked,
                    sea_orm::sea_query::Expr::value(true),
                )
                .filter(execution_workers::Column::Id.eq(grant.worker_id))
                .exec(&rotation)
                .await
                .unwrap();
            rotation.commit().await.unwrap();
            let denied = future.await.unwrap_err();
            assert_eq!(denied.status, axum::http::StatusCode::UNAUTHORIZED);
        }
        claimant.rollback().await.unwrap();
        // Even a Worker extractor obtained before rotation must fail inside its later lease transaction.
        assert_eq!(
            worker_auth::lease_authority(&ctx.db, &worker)
                .await
                .unwrap_err()
                .status,
            axum::http::StatusCode::UNAUTHORIZED
        );
    })
    .await;
}

#[tokio::test]
async fn operator_resolution_requires_paused_quarantine_and_exact_pending_operation() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let pool = prepare(&server, &ctx).await;
        let h = boot(&server, &pool).await;
        action(
            &server,
            &pool,
            &snap(&ctx, &pool).await,
            CapacityAction::Observe,
            Some(ObservedPower::Running),
        )
        .await
        .assert_status_ok();
        make_idle(&ctx, &pool).await;
        action(
            &server,
            &pool,
            &snap(&ctx, &pool).await,
            CapacityAction::Drain,
            None,
        )
        .await
        .assert_status_ok();
        hosts::cleanup(&ctx, pool.registration.host_id, pool.cleanup(h.generation))
            .await
            .unwrap();
        action(
            &server,
            &pool,
            &snap(&ctx, &pool).await,
            CapacityAction::CommitStop,
            None,
        )
        .await
        .assert_status_ok();
        action(
            &server,
            &pool,
            &snap(&ctx, &pool).await,
            CapacityAction::Quarantine,
            None,
        )
        .await
        .assert_status_ok();
        let s = snap(&ctx, &pool).await;
        let op = s.current_operation.unwrap().id;
        assert!(hosts::resolve_operation(
            &ctx,
            pool.actor,
            pool.registration.pool_id,
            op,
            s.control_version,
            ObservedPower::Stopped,
            true,
            "incident:fixture-verified-power"
        )
        .await
        .is_err());
        hosts::maintain(
            &ctx,
            pool.actor,
            pool.registration.pool_id,
            "pause-pool",
            None,
        )
        .await
        .unwrap();
        assert!(hosts::resolve_operation(
            &ctx,
            pool.actor,
            pool.registration.pool_id,
            Uuid::new_v4(),
            s.control_version,
            ObservedPower::Stopped,
            true,
            "incident:fixture-verified-power"
        )
        .await
        .is_err());
        hosts::resolve_operation(
            &ctx,
            pool.actor,
            pool.registration.pool_id,
            op,
            s.control_version,
            ObservedPower::Stopped,
            true,
            "incident:fixture-verified-power",
        )
        .await
        .unwrap();
        assert_eq!(snap(&ctx, &pool).await.state, HostState::Quarantined);
        hosts::maintain(
            &ctx,
            pool.actor,
            pool.registration.pool_id,
            "recover-host",
            None,
        )
        .await
        .unwrap();
        let final_state = snap(&ctx, &pool).await;
        assert_eq!(final_state.state, HostState::Stopped);
        assert!(!final_state.policy.enabled);
        assert!(final_state.current_operation.is_none());
    })
    .await;
}

/// Populate a real build and accepted lease; only device effects are synthetic in these fence tests.
async fn seed_lease(
    server: &TestServer,
    ctx: &AppContext,
    pool: &Pool,
    worker: &worker_auth::Worker,
    claim: Uuid,
) -> Uuid {
    let bytes = fixture("execution");
    let upload = new_upload(server, &pool.owner, worker.app_id, &bytes).await;
    transfer(server, &pool.owner, worker.app_id, upload.id, bytes)
        .await
        .assert_status_ok();
    let response = pool
        .owner
        .write(server.post(&format!(
            "/api/apps/{}/build-uploads/{}/complete",
            worker.app_id, upload.id
        )))
        .await;
    response.assert_status_ok();
    let build = response.json::<BuildResponse>();
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/execution/comparison.json")).unwrap();
    let mut manifest: mobile_qa_contracts::execution::RunManifest =
        serde_json::from_value(fixture["manifest"].clone()).unwrap();
    manifest.app_id = worker.app_id;
    manifest.build_id = build.id;
    manifest.build_sha256 = build.sha256.clone();
    manifest.build_bytes = build.byte_size as u32;
    manifest.profile = test_definitions::profile(&ctx.db, worker.app_id, worker.profile_id)
        .await
        .unwrap();
    manifest.resolved_model = None;
    let run = Uuid::new_v4();
    let attempt = Uuid::new_v4();
    execution_runs::ActiveModel {
        id: Set(run),
        app_id: Set(worker.app_id),
        creator_id: Set(pool.actor),
        build_id: Set(build.id),
        plan_id: Set(None),
        idempotency_key: Set(Uuid::new_v4().to_string()),
        fingerprint: Set("synthetic-fence-fixture".into()),
        manifest: Set(json(&manifest).unwrap()),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .unwrap();
    execution_attempts::ActiveModel {
        id: Set(attempt),
        run_id: Set(run),
        case_index: Set(0),
        state: Set("leased".into()),
        worker_id: Set(Some(worker.id)),
        claim_id: Set(Some(claim)),
        lease_hash: Set(Some(hash(
            "test-lease-token-test-lease-token-test-lease-token-test-lease-token",
        ))),
        expires_at: Set(Some(Utc::now() + Duration::seconds(60))),
        claimed_at: Set(Some(Utc::now())),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .unwrap();
    for resource in [
        format!("app:{}", worker.app_id),
        format!("device:{}", manifest.profile.device_identity),
    ] {
        execution_reservations::ActiveModel {
            resource: Set(resource),
            attempt_id: Set(Some(attempt)),
            session_id: Set(None),
        }
        .insert(&ctx.db)
        .await
        .unwrap();
    }
    attempt
}

#[tokio::test]
async fn owned_claim_replay_renews_same_lease_while_host_is_draining() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let pool = prepare(&server, &ctx).await;
        let h = boot(&server, &pool).await;
        let slot = pool.registration.slots[0].id;
        let grant = hosts::grant(
            &ctx,
            pool.registration.host_id,
            slot,
            pool.grant(h.generation),
        )
        .await
        .unwrap();
        let worker = worker_auth::Worker {
            id: grant.worker_id,
            app_id: grant.app_id,
            profile_id: grant.profile_id,
        };
        let claim = Uuid::new_v4();
        let attempt = seed_lease(&server, &ctx, &pool, &worker, claim).await;
        device_slots::Entity::update_many()
            .col_expr(
                device_slots::Column::State,
                sea_orm::sea_query::Expr::value("leased"),
            )
            .filter(device_slots::Column::Id.eq(slot))
            .exec(&ctx.db)
            .await
            .unwrap();
        host_rows::Entity::update_many()
            .col_expr(
                host_rows::Column::State,
                sea_orm::sea_query::Expr::value("draining"),
            )
            .col_expr(
                host_rows::Column::ObservedPower,
                sea_orm::sea_query::Expr::value("running"),
            )
            .filter(host_rows::Column::Id.eq(pool.registration.host_id))
            .exec(&ctx.db)
            .await
            .unwrap();
        let request = ClaimRequest {
            version: 6,
            claim_id: claim,
            profile_id: worker.profile_id,
            model_capabilities: None,
        };
        let replay = scheduler::claim(&ctx, &worker, request.clone())
            .await
            .unwrap()
            .lease
            .unwrap();
        assert_eq!(replay.attempt_id, attempt);
        let again = scheduler::claim(&ctx, &worker, request)
            .await
            .unwrap()
            .lease
            .unwrap();
        assert_eq!(again.attempt_id, attempt);
        assert_ne!(again.lease_token, replay.lease_token);
        let new = scheduler::claim(
            &ctx,
            &worker,
            ClaimRequest {
                version: 6,
                claim_id: Uuid::new_v4(),
                profile_id: worker.profile_id,
                model_capabilities: None,
            },
        )
        .await
        .unwrap();
        assert!(new.lease.is_none());
        assert_eq!(
            hosts::active_work(&ctx.db, pool.registration.host_id)
                .await
                .unwrap(),
            1
        );
        action(
            &server,
            &pool,
            &snap(&ctx, &pool).await,
            CapacityAction::CommitStop,
            None,
        )
        .await
        .assert_status_conflict();
    })
    .await;
}

#[tokio::test]
async fn legacy_physical_reservation_remains_visible_after_pool_binding() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let pool = prepare(&server, &ctx).await;
        let b = &pool.registration.bindings[0];
        let worker = worker_auth::Worker {
            id: Uuid::new_v4(),
            app_id: b.app_id,
            profile_id: b.profile_id,
        };
        worker_auth::register(
            &ctx,
            pool.actor,
            b.app_id,
            worker.id,
            b.profile_id,
            &loco_rs::hash::random_string(64),
        )
        .await
        .unwrap();
        let attempt = seed_lease(&server, &ctx, &pool, &worker, Uuid::new_v4()).await;
        assert_eq!(hosts::slot_active(&ctx.db, b.slot_id).await.unwrap(), 1);
        assert_eq!(
            hosts::active_work(&ctx.db, pool.registration.host_id)
                .await
                .unwrap(),
            1
        );
        let h = boot(&server, &pool).await;
        assert_eq!(h.state, HostState::Quarantined);
        assert_eq!(
            execution_attempts::Entity::find_by_id(attempt)
                .one(&ctx.db)
                .await
                .unwrap()
                .unwrap()
                .state,
            "leased"
        );
        assert_eq!(
            hosts::active_work(&ctx.db, pool.registration.host_id)
                .await
                .unwrap(),
            1
        );
        let copy = PoolRegistration {
            pool_id: Uuid::new_v4(),
            host_id: Uuid::new_v4(),
            instance_id: format!("i-{}", &Uuid::new_v4().simple().to_string()[..17]),
            slots: vec![SlotDefinition {
                id: Uuid::new_v4(),
                ..pool.registration.slots[0].clone()
            }],
            ..pool.registration.clone()
        };
        let mut copy = copy;
        copy.bindings[0].slot_id = copy.slots[0].id;
        assert!(hosts::provision(
            &ctx,
            pool.actor,
            copy,
            &loco_rs::hash::random_string(64),
            &loco_rs::hash::random_string(64)
        )
        .await
        .is_err());
    })
    .await;
}
