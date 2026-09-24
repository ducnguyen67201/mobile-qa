//! Commercial authorization is exercised through real HTTP routes and PostgreSQL.
mod support;
use mobile_qa::{
    models::_entities::{
        billing_checkout_intents, billing_credit_periods, commercial_usage, execution_artifacts,
        execution_attempts,
    },
    services::{commercial, credit_billing, test_definitions as defs},
};
use mobile_qa_contracts::{
    commercial::*, execution::*, regression::CaseRunRequest, task_sessions::OpenPhoneRequest,
};
use serde_json::json;
use support::*;

async fn prepared(
    server: &TestServer,
    ctx: &AppContext,
    owner: &Login,
) -> (Uuid, Uuid, Uuid, Uuid) {
    let mut input = create_input(owner.org);
    input.android_package = "ai.mobileqa.demo".into();
    let app = apps::create(ctx, owner.user, input).await.unwrap();
    let bytes = fixture("execution");
    let upload = new_upload(server, owner, app.id, &bytes).await;
    transfer(server, owner, app.id, upload.id, bytes)
        .await
        .assert_status_ok();
    let build = owner
        .write(server.post(&format!(
            "/api/apps/{}/build-uploads/{}/complete",
            app.id, upload.id
        )))
        .await
        .json::<BuildResponse>();
    assert_eq!(build.validation.state, ValidationState::Validated);
    let profile = ExecutionProfile {
        execution_context: None,
        id: Uuid::new_v4(),
        name: "Synthetic profile".into(),
        driver: Driver::Fake,
        package: "ai.mobileqa.demo".into(),
        adapter: "demo_persistence_v1".into(),
        device_identity: Uuid::new_v4().to_string(),
        image: "synthetic".into(),
        model: None,
        qualified: true,
        qualification_reference: "synthetic-fixture-only".into(),
        max_apk_bytes: 262144000,
    };
    defs::register_profile(ctx, owner.user, app.id, profile.clone())
        .await
        .unwrap();
    let case: TestDefinition = serde_json::from_str(include_str!(
        "../../../contracts/fixtures/execution/persistence-case.json"
    ))
    .unwrap();
    let case = defs::import(
        ctx,
        owner.user,
        DefinitionImport {
            app_id: app.id,
            definition: case,
        },
    )
    .await
    .unwrap();
    let plan = TestDefinition::Plan(PlanDefinition {
        key: "commercial".into(),
        version: 1,
        title: "Commercial check".into(),
        suite_version_ids: vec![],
        cases: vec![CaseSelection {
            case_version_id: case.id,
            data_variant: "default".into(),
            required: true,
        }],
        profile_id: profile.id,
        budget: ExecutionBudget {
            duration_seconds: 1200,
            max_steps: 30,
            artifact_bytes: 16777216,
        },
        diagnostic_retries: 0,
        exclusions: vec![],
    });
    let plan = defs::import(
        ctx,
        owner.user,
        DefinitionImport {
            app_id: app.id,
            definition: plan,
        },
    )
    .await
    .unwrap();
    (app.id, build.id, plan.id, profile.id)
}

#[tokio::test]
async fn pilot_quote_authorization_replay_cancel_credit_and_tenant_isolation() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let owner = login(&server, &ctx).await;
        let foreign = login(&server, &ctx).await;
        let (app, build, plan, profile) = prepared(&server, &ctx, &owner).await;
        let access_url = format!("/api/apps/{app}/commercial-access");
        let change_url = format!("/api/apps/{app}/credit-plan-change");
        error(
            &foreign
                .write(server.post(&change_url))
                .json(&CreditPlanChangeRequest {
                    plan: CreditPlan::Plus,
                })
                .await,
            404,
        );
        error(
            &owner
                .write(server.post(&change_url))
                .json(&CreditPlanChangeRequest {
                    plan: CreditPlan::Plus,
                })
                .await,
            409,
        );
        assert_eq!(
            owner
                .read(server.get(&access_url))
                .await
                .json::<CommercialAccessResponse>()
                .state,
            CommercialState::Uncontracted
        );
        error(&foreign.read(server.get(&access_url)).await, 404);
        let pilot_url = format!("/api/apps/{app}/commercial-pilot-requests");
        error(
            &foreign
                .write(server.post(&pilot_url))
                .json(&CommercialPilotRequest {
                    coverage_note: "Demo".into(),
                })
                .await,
            404,
        );
        let inquiry = owner
            .write(server.post(&pilot_url))
            .json(&CommercialPilotRequest {
                coverage_note: "Release journeys".into(),
            })
            .await
            .json::<CommercialPilotResponse>();
        assert_eq!(
            owner
                .read(server.get(&access_url))
                .await
                .json::<CommercialAccessResponse>()
                .pilot_request_id,
            Some(inquiry.id)
        );
        error(
            &owner
                .write(server.post(&pilot_url))
                .json(&CommercialPilotRequest {
                    coverage_note: "Again".into(),
                })
                .await,
            409,
        );
        let quote_url = format!("/api/apps/{app}/commercial-check-quotes");
        let input = CreateRunRequest {
            build_id: build,
            plan_version_id: plan,
            environment_revision: 1,
        };
        let intent = CommercialQuoteRequest {
            intent: CommercialRunIntent::ReleasePlan(input.clone()),
        };
        error(
            &owner.write(server.post(&quote_url)).json(&intent).await,
            409,
        );
        let start = Utc::now() - Duration::minutes(1);
        commercial::activate(
            &ctx,
            owner.user,
            app,
            CommercialOffer::Pilot,
            start,
            start + Duration::days(14),
            plan,
            None,
            profile,
            "synthetic-pilot",
            "Fixture agreement",
        )
        .await
        .unwrap();
        error(
            &owner
                .write(server.post(&format!("/api/apps/{app}/case-runs")))
                .add_header("idempotency-key", "unpriced-case")
                .json(&CaseRunRequest {
                    case_version_id: Uuid::new_v4(),
                    build_id: build,
                    profile_id: profile,
                    environment_revision: 1,
                    baseline_run_id: None,
                })
                .await,
            409,
        );
        error(
            &owner
                .write(server.post(&format!("/api/apps/{app}/phones")))
                .json(&OpenPhoneRequest {
                    id: Uuid::new_v4(),
                    build_id: Some(build),
                    profile_id: Some(profile),
                })
                .await,
            409,
        );
        error(
            &foreign.write(server.post(&quote_url)).json(&intent).await,
            404,
        );
        let quote = owner
            .write(server.post(&quote_url))
            .json(&intent)
            .await
            .json::<CommercialQuoteResponse>();
        assert_eq!(quote.amount_cents, 0);
        assert_eq!(quote.check_cap, 4);
        let run_url = format!("/api/apps/{app}/runs");
        error(
            &owner
                .write(server.post(&run_url))
                .add_header("idempotency-key", "unquoted")
                .json(&input)
                .await,
            409,
        );
        let submit = || {
            owner
                .write(server.post(&run_url))
                .add_header("idempotency-key", "commercial-same-click")
                .add_header("x-commercial-quote-id", quote.id.to_string())
                .json(&input)
        };
        let first = submit().await;
        first.assert_status(axum::http::StatusCode::CREATED);
        let run = first.json::<RunResponse>();
        submit().await.assert_status_ok();
        error(
            &owner
                .write(server.post(&run_url))
                .add_header("idempotency-key", "different-click")
                .add_header("x-commercial-quote-id", quote.id.to_string())
                .json(&input)
                .await,
            409,
        );
        assert!(commercial_usage::Entity::find_by_id(run.id)
            .one(&ctx.db)
            .await
            .unwrap()
            .is_some());
        owner
            .write(server.post(&format!("/api/runs/{}/cancel", run.id)))
            .await
            .assert_status_ok();
        let status = owner
            .read(server.get(&access_url))
            .await
            .json::<CommercialAccessResponse>();
        assert_eq!(status.credited_checks, 1);
        assert_eq!(status.reserved_checks, 0);
        assert_eq!(status.usage[0].run_id, run.id);
        assert_eq!(status.usage[0].state, CommercialUsageState::Credited);
        for index in 0..3 {
            let next = owner
                .write(server.post(&quote_url))
                .json(&intent)
                .await
                .json::<CommercialQuoteResponse>();
            let response = owner
                .write(server.post(&run_url))
                .add_header("idempotency-key", format!("pilot-check-{index}"))
                .add_header("x-commercial-quote-id", next.id.to_string())
                .json(&input)
                .await;
            response.assert_status(axum::http::StatusCode::CREATED);
        }
        let last_a = owner
            .write(server.post(&quote_url))
            .json(&intent)
            .await
            .json::<CommercialQuoteResponse>();
        let last_b = owner
            .write(server.post(&quote_url))
            .json(&intent)
            .await
            .json::<CommercialQuoteResponse>();
        let (a, b) = tokio::join!(
            owner
                .write(server.post(&run_url))
                .add_header("idempotency-key", "concurrent-a")
                .add_header("x-commercial-quote-id", last_a.id.to_string())
                .json(&input),
            owner
                .write(server.post(&run_url))
                .add_header("idempotency-key", "concurrent-b")
                .add_header("x-commercial-quote-id", last_b.id.to_string())
                .json(&input),
        );
        let mut outcomes = [a.status_code().as_u16(), b.status_code().as_u16()];
        outcomes.sort();
        assert_eq!(outcomes, [201, 409]);
        error(
            &owner.write(server.post(&quote_url)).json(&intent).await,
            409,
        );
        let full = owner
            .read(server.get(&access_url))
            .await
            .json::<CommercialAccessResponse>();
        assert_eq!(full.reserved_checks, 4);
        assert_eq!(full.delivered_check_cents, 0);
        commercial::set_status(&ctx, owner.user, app, "paused", "Synthetic pause")
            .await
            .unwrap();
        error(
            &owner.write(server.post(&quote_url)).json(&intent).await,
            409,
        );
    })
    .await;
}

#[tokio::test]
async fn recurring_price_is_server_owned_and_period_begins_with_zero_checks() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let owner = login(&server, &ctx).await;
        let (app, build, plan, profile) = prepared(&server, &ctx, &owner).await;
        let start = Utc::now() - Duration::minutes(1);
        commercial::activate(
            &ctx,
            owner.user,
            app,
            CommercialOffer::Recurring,
            start,
            start + Duration::days(30),
            plan,
            None,
            profile,
            "synthetic-recurring",
            "Fixture agreement",
        )
        .await
        .unwrap();
        let status = owner
            .read(server.get(&format!("/api/apps/{app}/commercial-access")))
            .await
            .json::<CommercialAccessResponse>();
        let agreement = status.agreement.unwrap();
        assert_eq!(agreement.base_cents, 25_000);
        assert_eq!(agreement.second_suite_cents, 0);
        assert_eq!(agreement.check_cents, 12_500);
        assert_eq!(agreement.check_cap, 8);
        assert_eq!(status.delivered_check_cents, 0);
        let quote = owner
            .write(server.post(&format!("/api/apps/{app}/commercial-check-quotes")))
            .json(&CommercialQuoteRequest {
                intent: CommercialRunIntent::ReleasePlan(CreateRunRequest {
                    build_id: build,
                    plan_version_id: plan,
                    environment_revision: 1,
                }),
            })
            .await
            .json::<CommercialQuoteResponse>();
        assert_eq!(quote.amount_cents, 12_500);
        assert_eq!(quote.checks_after_authorization, 1);
    })
    .await;
}

#[tokio::test]
async fn expired_agreement_can_be_closed_before_a_new_period() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let owner = login(&server, &ctx).await;
        let (app, _, plan, profile) = prepared(&server, &ctx, &owner).await;
        let past = Utc::now() - Duration::days(15);
        commercial::activate(
            &ctx,
            owner.user,
            app,
            CommercialOffer::Pilot,
            past,
            past + Duration::days(14),
            plan,
            None,
            profile,
            "past-pilot",
            "Fixture past agreement",
        )
        .await
        .unwrap();
        assert_eq!(
            commercial::status(&ctx, owner.user, app)
                .await
                .unwrap()
                .state,
            CommercialState::Expired
        );
        commercial::set_status(&ctx, owner.user, app, "ended", "Pilot period closed")
            .await
            .unwrap();
        let now = Utc::now();
        commercial::activate(
            &ctx,
            owner.user,
            app,
            CommercialOffer::Recurring,
            now,
            now + Duration::days(30),
            plan,
            None,
            profile,
            "new-recurring",
            "Fixture renewal",
        )
        .await
        .unwrap();
        assert_eq!(
            commercial::status(&ctx, owner.user, app)
                .await
                .unwrap()
                .state,
            CommercialState::Active
        );
    })
    .await;
}

#[tokio::test]
async fn paid_period_holds_credits_atomically_and_queued_cancel_releases_them() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let owner = login(&server, &ctx).await;
        let foreign = login(&server, &ctx).await;
        let (app, build, plan, _) = prepared(&server, &ctx, &owner).await;
        let checkout_id = Uuid::new_v4();
        let period_id = Uuid::new_v4();
        let now = Utc::now();
        billing_checkout_intents::ActiveModel {
            id: Set(checkout_id),
            app_id: Set(app),
            actor_id: Set(owner.user),
            plan: Set("starter".into()),
            state: Set("paid".into()),
            stripe_subscription_id: Set(Some(format!("sub_{checkout_id}"))),
            ..Default::default()
        }
        .insert(&ctx.db)
        .await
        .unwrap();
        billing_credit_periods::ActiveModel {
            id: Set(period_id),
            app_id: Set(app),
            checkout_intent_id: Set(checkout_id),
            stripe_subscription_id: Set(format!("sub_{checkout_id}")),
            stripe_invoice_id: Set(format!("in_{checkout_id}")),
            plan: Set("starter".into()),
            granted_credits: Set(50_000),
            starts_at: Set(now - Duration::minutes(1)),
            ends_at: Set(now + Duration::days(30)),
            rate_revision: Set(1),
            ..Default::default()
        }
        .insert(&ctx.db)
        .await
        .unwrap();
        let access_url = format!("/api/apps/{app}/commercial-access");
        error(&foreign.read(server.get(&access_url)).await, 404);
        let status = owner
            .read(server.get(&access_url))
            .await
            .json::<CommercialAccessResponse>();
        assert_eq!(status.credit.unwrap().available_credits, 50_000);
        let change_url = format!("/api/apps/{app}/credit-plan-change");
        error(
            &foreign
                .write(server.post(&change_url))
                .json(&CreditPlanChangeRequest {
                    plan: CreditPlan::Plus,
                })
                .await,
            404,
        );
        error(
            &owner
                .write(server.post(&change_url))
                .json(&CreditPlanChangeRequest {
                    plan: CreditPlan::Starter,
                })
                .await,
            409,
        );
        assert_eq!(status.plans.len(), 3);
        let quote_url = format!("/api/apps/{app}/commercial-check-quotes");
        let input = CreateRunRequest {
            build_id: build,
            plan_version_id: plan,
            environment_revision: 1,
        };
        let intent = CommercialQuoteRequest {
            intent: CommercialRunIntent::ReleasePlan(input.clone()),
        };
        let quote = owner
            .write(server.post(&quote_url))
            .json(&intent)
            .await
            .json::<CommercialQuoteResponse>();
        let max = quote.maximum_credits.unwrap();
        assert!(max > 3_000 && max < 50_000);
        // A second quote is allowed, but only one hold can fit after the grant shrinks.
        let second = owner
            .write(server.post(&quote_url))
            .json(&intent)
            .await
            .json::<CommercialQuoteResponse>();
        let row = billing_credit_periods::Entity::find_by_id(period_id)
            .one(&ctx.db)
            .await
            .unwrap()
            .unwrap();
        let mut active: billing_credit_periods::ActiveModel = row.into();
        active.granted_credits = Set(i64::from(max));
        active.update(&ctx.db).await.unwrap();
        let run_url = format!("/api/apps/{app}/runs");
        let run = owner
            .write(server.post(&run_url))
            .add_header("idempotency-key", "credit-first")
            .add_header("x-commercial-quote-id", quote.id.to_string())
            .json(&input)
            .await
            .json::<RunResponse>();
        error(
            &owner
                .write(server.post(&run_url))
                .add_header("idempotency-key", "credit-second")
                .add_header("x-commercial-quote-id", second.id.to_string())
                .json(&input)
                .await,
            409,
        );
        let held = owner
            .read(server.get(&access_url))
            .await
            .json::<CommercialAccessResponse>()
            .credit
            .unwrap();
        assert_eq!(held.held_credits, max);
        assert_eq!(held.available_credits, 0);
        owner
            .write(server.post(&format!("/api/runs/{}/cancel", run.id)))
            .await
            .assert_status_ok();
        let released = owner
            .read(server.get(&access_url))
            .await
            .json::<CommercialAccessResponse>()
            .credit
            .unwrap();
        assert_eq!(released.held_credits, 0);
        assert_eq!(released.available_credits, max);
        assert_eq!(released.usage[0].state, "released");
        let delivered_run = owner
            .write(server.post(&run_url))
            .add_header("idempotency-key", "credit-measured")
            .add_header("x-commercial-quote-id", second.id.to_string())
            .json(&input)
            .await
            .json::<RunResponse>();
        let attempt = execution_attempts::Entity::find()
            .filter(execution_attempts::Column::RunId.eq(delivered_run.id))
            .one(&ctx.db)
            .await
            .unwrap()
            .unwrap();
        let ended = Utc::now();
        let usage = json!([{"model":"synthetic","calls":1,"unknown_calls":0,
            "input_tokens":1000000,"output_tokens":1000000}]);
        let attempt_id = attempt.id;
        let mut active: execution_attempts::ActiveModel = attempt.into();
        active.state = Set("finished".into());
        active.outcome = Set(Some("passed".into()));
        active.cleanup = Set("verified_clean".into());
        active.claim_id = Set(Some(Uuid::new_v4()));
        active.claimed_at = Set(Some(ended - Duration::seconds(60)));
        active.released_at = Set(Some(ended));
        active.usage = Set(usage);
        active.update(&ctx.db).await.unwrap();
        execution_artifacts::ActiveModel {
            id: Set(Uuid::new_v4()),
            attempt_id: Set(attempt_id),
            checkpoint_id: Set("evidence".into()),
            name: Set("screen.png".into()),
            mime: Set("image/png".into()),
            byte_size: Set(1_048_576),
            sha256: Set("0".repeat(64)),
            state: Set("sealed".into()),
            storage_key: Set(format!("credit-test-{attempt_id}")),
            storage_backend: Set("local".into()),
            ..Default::default()
        }
        .insert(&ctx.db)
        .await
        .unwrap();
        commercial::review(
            &ctx,
            owner.user,
            delivered_run.id,
            CommercialUsageState::Delivered,
            "Reviewed usable report",
        )
        .await
        .unwrap();
        let settled = owner
            .read(server.get(&access_url))
            .await
            .json::<CommercialAccessResponse>()
            .credit
            .unwrap();
        assert_eq!(settled.held_credits, 0);
        assert_eq!(settled.charged_credits, 1061);
        assert_eq!(settled.available_credits, max - 1061);
        assert_eq!(settled.usage[0].measured_credits.as_deref(), Some("1061"));
        assert_eq!(settled.usage[0].device_seconds, Some(60));
        assert_eq!(settled.usage[0].stored_bytes.as_deref(), Some("1048576"));
        assert!(commercial::review(
            &ctx,
            owner.user,
            delivered_run.id,
            CommercialUsageState::Delivered,
            "Again"
        )
        .await
        .is_err());
    })
    .await;
}

#[tokio::test]
async fn verified_paid_invoice_grants_once_and_underpayment_grants_nothing() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|_server, ctx| async move {
        let owner = login(&_server, &ctx).await;
        let app = apps::create(&ctx,owner.user,create_input(owner.org)).await.unwrap();
        let intent_id=Uuid::new_v4();
        let subscription_id=format!("sub_{intent_id}");
        billing_checkout_intents::ActiveModel{id:Set(intent_id),app_id:Set(app.id),actor_id:Set(owner.user),plan:Set("plus".into()),state:Set("checkout".into()),..Default::default()}.insert(&ctx.db).await.unwrap();
        let start=Utc::now()-Duration::minutes(1);
        let end=start+Duration::days(30);
        let subscription=json!({"id":subscription_id,"metadata":{"checkout_intent_id":intent_id.to_string()},
            "items":{"data":[{"quantity":1,"price":{"id":"price_plus","unit_amount":75000,
                "currency":"usd","recurring":{"interval":"month"}}}]}});
        // Match the current Stripe Invoice payload: status replaces paid,
        // and the subscription and line price live under parent/pricing.
        let invoice=json!({"id":"in_paid_plus","status":"paid","collection_method":"charge_automatically",
            "currency":"usd","amount_paid":75000,
            "parent":{"subscription_details":{"subscription":subscription_id}},
            "lines":{"data":[{"pricing":{"price_details":{"price":"price_plus"}},
                "period":{"start":start.timestamp(),"end":end.timestamp()}}]}});
        credit_billing::apply_verified_invoice(&ctx,&invoice,&subscription).await.unwrap();
        credit_billing::apply_verified_invoice(&ctx,&invoice,&subscription).await.unwrap();
        let status=commercial::status(&ctx,owner.user,app.id).await.unwrap();
        let balance=status.credit.unwrap();
        assert_eq!(balance.granted_credits,75_000);
        assert_eq!(balance.available_credits,75_000);
        assert_eq!(billing_credit_periods::Entity::find().filter(billing_credit_periods::Column::AppId.eq(app.id)).all(&ctx.db).await.unwrap().len(),1);
        let underpaid=json!({"id":"in_underpaid","status":"paid","collection_method":"charge_automatically",
            "currency":"usd","amount_paid":50000,
            "parent":{"subscription_details":{"subscription":subscription_id}},
            "lines":{"data":[{"pricing":{"price_details":{"price":"price_plus"}},
                "period":{"start":end.timestamp(),"end":(end+Duration::days(30)).timestamp()}}]}});
        assert!(credit_billing::apply_verified_invoice(&ctx,&underpaid,&subscription).await.is_err());
        let out_of_band=json!({"id":"in_out_of_band","status":"paid","collection_method":"charge_automatically",
            "paid_out_of_band":true,"currency":"usd","amount_paid":75000,
            "parent":{"subscription_details":{"subscription":subscription_id}},
            "lines":{"data":[{"pricing":{"price_details":{"price":"price_plus"}},
                "period":{"start":end.timestamp(),"end":(end+Duration::days(30)).timestamp()}}]}});
        assert!(credit_billing::apply_verified_invoice(&ctx,&out_of_band,&subscription).await.is_err());
        assert_eq!(billing_credit_periods::Entity::find().filter(billing_credit_periods::Column::AppId.eq(app.id)).all(&ctx.db).await.unwrap().len(),1);
    }).await;
}

#[tokio::test]
async fn next_plan_grants_only_on_paid_renewal_and_old_invoice_replay_is_stable() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let owner = login(&server, &ctx).await;
        let app = apps::create(&ctx,owner.user,create_input(owner.org)).await.unwrap();
        let intent = Uuid::new_v4();
        let subscription_id = format!("sub_{intent}");
        let start = chrono::DateTime::<Utc>::from_timestamp(Utc::now().timestamp(), 0).unwrap()
            - Duration::minutes(1);
        let renewal = start+Duration::days(30);
        let next_end = renewal+Duration::days(30);
        billing_checkout_intents::ActiveModel{id:Set(intent),app_id:Set(app.id),actor_id:Set(owner.user),plan:Set("starter".into()),state:Set("paid".into()),stripe_subscription_id:Set(Some(subscription_id.clone())),pending_plan:Set(Some("plus".into())),pending_effective_at:Set(Some(renewal)),stripe_schedule_id:Set(Some("sub_sched_test".into())),..Default::default()}.insert(&ctx.db).await.unwrap();
        let original = json!({"id":"in_old","status":"paid","collection_method":"charge_automatically",
            "currency":"usd","amount_paid":50000,"parent":{"subscription_details":{"subscription":subscription_id}},
            "lines":{"data":[{"pricing":{"price_details":{"price":"price_starter"}},
                "period":{"start":start.timestamp(),"end":renewal.timestamp()}}]}});
        let old_sub = json!({"id":subscription_id,"metadata":{"checkout_intent_id":intent.to_string()},
            "items":{"data":[{"quantity":1,"price":{"id":"price_starter","unit_amount":50000,
                "currency":"usd","recurring":{"interval":"month"}}}]}});
        credit_billing::apply_verified_invoice(&ctx,&original,&old_sub).await.unwrap();
        let access_url = format!("/api/apps/{}/commercial-access",app.id);
        let first = owner.read(server.get(&access_url)).await.json::<CommercialAccessResponse>().credit.unwrap();
        assert_eq!(first.plan,CreditPlan::Starter);
        assert_eq!(first.pending_plan,Some(CreditPlan::Plus));
        assert_eq!(first.available_credits,50_000);
        let next_sub = json!({"id":subscription_id,"metadata":{"checkout_intent_id":intent.to_string()},
            "items":{"data":[{"quantity":1,"price":{"id":"price_plus","unit_amount":75000,
                "currency":"usd","recurring":{"interval":"month"}}}]}});
        let renewal_invoice = json!({"id":"in_renewal","status":"paid","collection_method":"charge_automatically",
            "currency":"usd","amount_paid":75000,"parent":{"subscription_details":{"subscription":subscription_id}},
            "lines":{"data":[{"pricing":{"price_details":{"price":"price_plus"}},
                "period":{"start":renewal.timestamp(),"end":next_end.timestamp()}}]}});
        let mut underpaid = renewal_invoice.clone();
        underpaid["amount_paid"] = json!(50000);
        assert!(credit_billing::apply_verified_invoice(&ctx,&underpaid,&next_sub).await.is_err());
        let still = owner.read(server.get(&access_url)).await.json::<CommercialAccessResponse>().credit.unwrap();
        assert_eq!(still.plan,CreditPlan::Starter);
        assert_eq!(still.pending_plan,Some(CreditPlan::Plus));
        credit_billing::apply_verified_invoice(&ctx,&renewal_invoice,&next_sub).await.unwrap();
        credit_billing::apply_verified_invoice(&ctx,&original,&next_sub).await.unwrap();
        credit_billing::apply_verified_invoice(&ctx,&renewal_invoice,&next_sub).await.unwrap();
        let upgraded=billing_credit_periods::Entity::find().filter(billing_credit_periods::Column::StripeInvoiceId.eq("in_renewal")).one(&ctx.db).await.unwrap().unwrap().granted_credits;
        assert_eq!(upgraded,75_000);
        let pending=billing_checkout_intents::Entity::find_by_id(intent).one(&ctx.db).await.unwrap().unwrap().pending_plan;
        assert_eq!(pending,None);
        assert_eq!(billing_credit_periods::Entity::find().filter(billing_credit_periods::Column::AppId.eq(app.id)).all(&ctx.db).await.unwrap().len(),2);
    }).await;
}
