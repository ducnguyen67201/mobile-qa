//! Commercial authorization is exercised through real HTTP routes and PostgreSQL.
mod support;
use mobile_qa::services::{
    commercial, credit_billing, execution_store::*, test_definitions as defs,
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
        let count: i32 = field(
            &one(
                &ctx.db,
                "SELECT COUNT(*)::integer AS n FROM commercial_usage WHERE run_id=$1",
                vec![run.id.into()],
            )
            .await
            .unwrap(),
            "n",
        )
        .unwrap();
        assert_eq!(count, 1);
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
        exec(&ctx.db, "INSERT INTO billing_checkout_intents(id,app_id,actor_id,plan,state,stripe_subscription_id) VALUES($1,$2,$3,'starter','paid',$4)",
            vec![checkout_id.into(),app.into(),owner.user.into(),format!("sub_{checkout_id}").into()]).await.unwrap();
        exec(&ctx.db, "INSERT INTO billing_credit_periods(id,app_id,checkout_intent_id,stripe_subscription_id,stripe_invoice_id,plan,granted_credits,starts_at,ends_at,rate_revision) VALUES($1,$2,$3,$4,$5,'starter',50000,$6,$7,1)",
            vec![period_id.into(),app.into(),checkout_id.into(),format!("sub_{checkout_id}").into(),format!("in_{checkout_id}").into(),(now-Duration::minutes(1)).into(),(now+Duration::days(30)).into()]).await.unwrap();
        let access_url = format!("/api/apps/{app}/commercial-access");
        error(&foreign.read(server.get(&access_url)).await,404);
        let status = owner.read(server.get(&access_url)).await.json::<CommercialAccessResponse>();
        assert_eq!(status.credit.unwrap().available_credits,50_000);
        let change_url = format!("/api/apps/{app}/credit-plan-change");
        error(&foreign.write(server.post(&change_url)).json(&CreditPlanChangeRequest { plan: CreditPlan::Plus }).await,404);
        error(&owner.write(server.post(&change_url)).json(&CreditPlanChangeRequest { plan: CreditPlan::Starter }).await,409);
        assert_eq!(status.plans.len(),3);
        let quote_url=format!("/api/apps/{app}/commercial-check-quotes");
        let input=CreateRunRequest { build_id:build,plan_version_id:plan,environment_revision:1 };
        let intent=CommercialQuoteRequest { intent:CommercialRunIntent::ReleasePlan(input.clone()) };
        let quote=owner.write(server.post(&quote_url)).json(&intent).await.json::<CommercialQuoteResponse>();
        let max=quote.maximum_credits.unwrap();
        assert!(max>3_000 && max<50_000);
        // A second quote is allowed, but only one hold can fit after the grant shrinks.
        let second=owner.write(server.post(&quote_url)).json(&intent).await.json::<CommercialQuoteResponse>();
        exec(&ctx.db,"UPDATE billing_credit_periods SET granted_credits=$2 WHERE id=$1",vec![period_id.into(),max.into()]).await.unwrap();
        let run_url=format!("/api/apps/{app}/runs");
        let run=owner.write(server.post(&run_url)).add_header("idempotency-key","credit-first")
            .add_header("x-commercial-quote-id",quote.id.to_string()).json(&input).await.json::<RunResponse>();
        error(&owner.write(server.post(&run_url)).add_header("idempotency-key","credit-second")
            .add_header("x-commercial-quote-id",second.id.to_string()).json(&input).await,409);
        let held=owner.read(server.get(&access_url)).await.json::<CommercialAccessResponse>().credit.unwrap();
        assert_eq!(held.held_credits,max);
        assert_eq!(held.available_credits,0);
        owner.write(server.post(&format!("/api/runs/{}/cancel",run.id))).await.assert_status_ok();
        let released=owner.read(server.get(&access_url)).await.json::<CommercialAccessResponse>().credit.unwrap();
        assert_eq!(released.held_credits,0);
        assert_eq!(released.available_credits,max);
        assert_eq!(released.usage[0].state,"released");
        let delivered_run=owner.write(server.post(&run_url)).add_header("idempotency-key","credit-measured")
            .add_header("x-commercial-quote-id",second.id.to_string()).json(&input).await.json::<RunResponse>();
        let attempt:Uuid=field(&one(&ctx.db,"SELECT id FROM execution_attempts WHERE run_id=$1",
            vec![delivered_run.id.into()]).await.unwrap(),"id").unwrap();
        let ended=Utc::now();
        let usage=json!([{"model":"synthetic","calls":1,"unknown_calls":0,
            "input_tokens":1000000,"output_tokens":1000000}]);
        exec(&ctx.db,"UPDATE execution_attempts SET state='finished',outcome='passed',cleanup='verified_clean',claim_id=$2,claimed_at=$3,released_at=$4,usage=$5 WHERE id=$1",
            vec![attempt.into(),Uuid::new_v4().into(),(ended-Duration::seconds(60)).into(),ended.into(),usage.into()]).await.unwrap();
        exec(&ctx.db,"INSERT INTO execution_artifacts(id,attempt_id,checkpoint_id,name,mime,byte_size,sha256,state,storage_key,storage_backend) VALUES($1,$2,'evidence','screen.png','image/png',1048576,$3,'sealed',$4,'local')",
            vec![Uuid::new_v4().into(),attempt.into(),"0".repeat(64).into(),format!("credit-test-{attempt}").into()]).await.unwrap();
        commercial::review(&ctx,owner.user,delivered_run.id,CommercialUsageState::Delivered,"Reviewed usable report").await.unwrap();
        let settled=owner.read(server.get(&access_url)).await.json::<CommercialAccessResponse>().credit.unwrap();
        assert_eq!(settled.held_credits,0);
        assert_eq!(settled.charged_credits,1061);
        assert_eq!(settled.available_credits,max-1061);
        assert_eq!(settled.usage[0].measured_credits.as_deref(),Some("1061"));
        assert_eq!(settled.usage[0].device_seconds,Some(60));
        assert_eq!(settled.usage[0].stored_bytes.as_deref(),Some("1048576"));
        assert!(commercial::review(&ctx,owner.user,delivered_run.id,CommercialUsageState::Delivered,"Again").await.is_err());
    }).await;
}

#[tokio::test]
async fn verified_paid_invoice_grants_once_and_underpayment_grants_nothing() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|_server, ctx| async move {
        let owner = login(&_server, &ctx).await;
        let app = apps::create(&ctx,owner.user,create_input(owner.org)).await.unwrap();
        let intent_id=Uuid::new_v4();
        let subscription_id=format!("sub_{intent_id}");
        exec(&ctx.db,"INSERT INTO billing_checkout_intents(id,app_id,actor_id,plan,state) VALUES($1,$2,$3,'plus','checkout')",
            vec![intent_id.into(),app.id.into(),owner.user.into()]).await.unwrap();
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
        let duplicate:i32=field(&one(&ctx.db,"SELECT COUNT(*)::integer AS n FROM billing_credit_periods WHERE app_id=$1",
            vec![app.id.into()]).await.unwrap(),"n").unwrap();
        assert_eq!(duplicate,1);
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
        let count:i32=field(&one(&ctx.db,"SELECT COUNT(*)::integer AS n FROM billing_credit_periods WHERE app_id=$1",
            vec![app.id.into()]).await.unwrap(),"n").unwrap();
        assert_eq!(count,1);
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
        exec(&ctx.db,"INSERT INTO billing_checkout_intents(id,app_id,actor_id,plan,state,stripe_subscription_id,pending_plan,pending_effective_at,stripe_schedule_id) VALUES($1,$2,$3,'starter','paid',$4,'plus',$5,'sub_sched_test')",
            vec![intent.into(),app.id.into(),owner.user.into(),subscription_id.clone().into(),renewal.into()]).await.unwrap();
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
        let upgraded:i64=field(&one(&ctx.db,"SELECT granted_credits FROM billing_credit_periods WHERE stripe_invoice_id='in_renewal'",vec![]).await.unwrap(),"granted_credits").unwrap();
        assert_eq!(upgraded,75_000);
        let pending:Option<String>=field(&one(&ctx.db,"SELECT pending_plan FROM billing_checkout_intents WHERE id=$1",vec![intent.into()]).await.unwrap(),"pending_plan").unwrap();
        assert_eq!(pending,None);
        let count:i32=field(&one(&ctx.db,"SELECT COUNT(*)::integer AS n FROM billing_credit_periods WHERE app_id=$1",vec![app.id.into()]).await.unwrap(),"n").unwrap();
        assert_eq!(count,2);
    }).await;
}
