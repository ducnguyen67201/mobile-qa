//! Actual HTTP and PostgreSQL session lifecycle; worker responses are synthetic.
mod support;
use mobile_qa::services::{execution_store::*, test_definitions, worker_auth};
use mobile_qa_contracts::{automation::*, execution::*, task_sessions::*, test_library::*};
use support::*;

#[tokio::test]
async fn task_session_requires_no_plan_and_fences_worker_and_task_identity() {
    assert_session_protocol(3).await;
}

#[tokio::test]
async fn protocol_four_runs_direct_commands_and_ai_discovery() {
    assert_session_protocol(4).await;
}

async fn assert_session_protocol(protocol_version: u32) {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let owner = login(&server, &ctx).await;
        let other = login(&server, &ctx).await;
        let mut input = create_input(owner.org);
        input.android_package = "ai.mobileqa.demo".into();
        let app = apps::create(&ctx, owner.user, input).await.unwrap().id;
        let bytes = fixture("execution");
        let upload = new_upload(&server, &owner, app, &bytes).await;
        transfer(&server, &owner, app, upload.id, bytes)
            .await
            .assert_status_ok();
        owner
            .write(server.post(&format!(
                "/api/apps/{app}/build-uploads/{}/complete",
                upload.id
            )))
            .await
            .assert_status_ok();
        let profile = ExecutionProfile {
        execution_context: None,
            id: Uuid::new_v4(),
            name: "Synthetic protocol test".into(),
            driver: Driver::Minitap,
            package: "ai.mobileqa.demo".into(),
            adapter: "demo_persistence_v1".into(),
            device_identity: Uuid::new_v4().to_string(),
            image: "test".into(),
            model: "test-model".into(),
            qualified: true,
            qualification_reference: "synthetic-not-device-evidence".into(),
            max_apk_bytes: 104857600,
        };
        test_definitions::register_profile(&ctx, owner.user, app, profile.clone())
            .await
            .unwrap();
        let token = loco_rs::hash::random_string(64);
        worker_auth::register(&ctx, owner.user, app, Uuid::new_v4(), profile.id, &token)
            .await
            .unwrap();
        let opts = owner
            .read(server.get(&format!("/api/apps/{app}/phone-options")))
            .await;
        opts.assert_status_ok();
        assert!(opts.json::<PhoneOptions>().blockers.is_empty());
        let open = OpenPhoneRequest {
            id: Uuid::new_v4(),
            build_id: None,
            profile_id: None,
        };
        let url = format!("/api/apps/{app}/phones");
        let first = owner.write(server.post(&url)).json(&open).await;
        first.assert_status_ok();
        let s = first.json::<PhoneSession>();
        assert_eq!(s.state, PhoneState::Queued);
        assert_eq!(
            owner
                .write(server.post(&url))
                .json(&open)
                .await
                .json::<PhoneSession>()
                .id,
            s.id
        );
        let path = format!("/api/phones/{}", s.id);
        assert!(!other
            .read(server.get(&path))
            .await
            .status_code()
            .is_success());
        let claim = server
            .post("/api/worker/phone-claims")
            .add_header("authorization", format!("Bearer {token}"))
            .json(&PhoneClaimRequest {
                protocol_version: 0,
                claim_id: Uuid::new_v4(),
            })
            .await;
        claim.assert_status_ok();
        let lease = claim.json::<PhoneClaimResponse>().lease.unwrap();
        let update = format!("/api/worker/phones/{}/update", s.id);
        let frame = PhoneFrame {
            id: Uuid::new_v4(),
            png_base64: {
                use base64::Engine;
                let mut p = b"\x89PNG\r\n\x1a\n00000000".to_vec();
                p.extend(1080u32.to_be_bytes());
                p.extend(1920u32.to_be_bytes());
                base64::engine::general_purpose::STANDARD.encode(p)
            },
            width: 1080,
            height: 1920,
            controls: vec![],
        };
        let mut status = PhoneUpdate {
            state: PhoneState::Ready,
            frame: Some(frame.clone()),
            task: None,
            message: "Synthetic screen".into(),
            clean: false,
        };
        let publish = || {
            server
                .post(&update)
                .add_header("authorization", format!("Bearer {token}"))
                .add_header("x-lease-token", lease.lease_token.clone())
        };
        publish().json(&status).await.assert_status_ok();
        let unsupported = serde_json::json!({"id":Uuid::new_v4(),"expected_revision":0,"frame_id":null,"title":"Back","sequence":{"actions":[{"id":"b","checkpoint_id":"b","kind":"direct","instruction":"","command":{"operation":"back"}}],"checks":[]}});
        assert_eq!(
            owner
                .write(server.post(&format!("{path}/commands")))
                .json(&unsupported)
                .await
                .status_code()
                .as_u16(),
            409
        );
        let legacy_generation = serde_json::json!({"id":Uuid::new_v4(),"session_id":s.id,"expected_revision":0,"category":"smoke","journey":"","allow_writes":false,"reuse_job_id":null,"engine":"minitap_v1"});
        assert_eq!(
            owner
                .write(server.post(&format!("/api/apps/{app}/test-generations")))
                .json(&legacy_generation)
                .await
                .status_code()
                .as_u16(),
            409
        );
        let stale = PhoneTaskRequest {
            id: Uuid::new_v4(),
            goal: "Tap Save".into(),
            selection: Some(PhoneSelection {
                frame_id: Uuid::new_v4(),
                control_id: "missing".into(),
            }),
        };
        assert_eq!(
            owner
                .write(server.post(&format!("{path}/tasks")))
                .json(&stale)
                .await
                .status_code()
                .as_u16(),
            409
        );
        let task = PhoneTaskRequest {
            id: Uuid::new_v4(),
            goal: "Type Buy milk".into(),
            selection: None,
        };
        let tasks = format!("{path}/tasks");
        let r = owner.write(server.post(&tasks)).json(&task).await;
        r.assert_status_ok();
        assert_eq!(r.json::<PhoneSession>().tasks.len(), 1);
        assert_eq!(
            owner
                .write(server.post(&tasks))
                .json(&task)
                .await
                .json::<PhoneSession>()
                .tasks
                .len(),
            1
        );
        let changed = PhoneTaskRequest {
            goal: "Different task".into(),
            ..task.clone()
        };
        assert_eq!(
            owner
                .write(server.post(&tasks))
                .json(&changed)
                .await
                .status_code()
                .as_u16(),
            409
        );
        owner
            .write(server.post(&format!("{path}/stop")))
            .await
            .assert_status_ok();
        assert_eq!(
            rows(
                &ctx.db,
                "SELECT resource FROM execution_reservations WHERE session_id=$1",
                vec![s.id.into()]
            )
            .await
            .unwrap()
            .len(),
            2
        );
        status.state = PhoneState::Closed;
        status.frame = None;
        status.clean = true;
        let closed = publish().json(&status).await;
        closed.assert_status_ok();
        assert_eq!(closed.json::<PhoneSession>().state, PhoneState::Closed);
        assert!(rows(
            &ctx.db,
            "SELECT resource FROM execution_reservations WHERE session_id=$1",
            vec![s.id.into()]
        )
        .await
        .unwrap()
        .is_empty());
        assert_eq!(publish().json(&status).await.status_code().as_u16(), 409);
        // Protocol 2 continues to admit direct commands but cannot claim Minitap discovery.
        let old_direct = OpenPhoneRequest {
            id: Uuid::new_v4(),
            build_id: None,
            profile_id: None,
        };
        owner
            .write(server.post(&url))
            .json(&old_direct)
            .await
            .assert_status_ok();
        let direct_lease = server
            .post("/api/worker/phone-claims")
            .add_header("authorization", format!("Bearer {token}"))
            .json(&PhoneClaimRequest {
                protocol_version: 2,
                claim_id: Uuid::new_v4(),
            })
            .await
            .json::<PhoneClaimResponse>()
            .lease
            .unwrap();
        let direct_update = format!("/api/worker/phones/{}/update", old_direct.id);
        let direct_publish = || {
            server
                .post(&direct_update)
                .add_header("authorization", format!("Bearer {token}"))
                .add_header("x-lease-token", direct_lease.lease_token.clone())
        };
        status.state = PhoneState::Ready;
        status.clean = false;
        status.frame = Some(frame.clone());
        direct_publish().json(&status).await.assert_status_ok();
        owner
            .write(server.post(&format!("/api/phones/{}/commands", old_direct.id)))
            .json(&unsupported)
            .await
            .assert_status_ok();
        let mut old_generation = legacy_generation.clone();
        old_generation["session_id"] = serde_json::json!(old_direct.id);
        assert_eq!(
            owner
                .write(server.post(&format!("/api/apps/{app}/test-generations")))
                .json(&old_generation)
                .await
                .status_code()
                .as_u16(),
            409
        );
        owner
            .write(server.post(&format!("/api/phones/{}/stop", old_direct.id)))
            .await
            .assert_status_ok();
        status.state = PhoneState::Closed;
        status.clean = true;
        direct_publish().json(&status).await.assert_status_ok();
        let next = OpenPhoneRequest {
            id: Uuid::new_v4(),
            build_id: None,
            profile_id: None,
        };
        owner
            .write(server.post(&url))
            .json(&next)
            .await
            .assert_status_ok();
        let claimed = server
            .post("/api/worker/phone-claims")
            .add_header("authorization", format!("Bearer {token}"))
            .json(&PhoneClaimRequest {
                protocol_version,
                claim_id: Uuid::new_v4(),
            })
            .await;
        claimed.assert_status_ok();
        let lease2 = claimed.json::<PhoneClaimResponse>().lease.unwrap();
        let update2 = format!("/api/worker/phones/{}/update", next.id);
        let publish2 = || {
            server
                .post(&update2)
                .add_header("authorization", format!("Bearer {token}"))
                .add_header("x-lease-token", lease2.lease_token.clone())
        };
        status.state = PhoneState::Ready;
        status.clean = false;
        status.frame = s.frame.clone();
        // Obtain a fresh synthetic frame, not a fabricated Android execution result.
        status.frame = Some(PhoneFrame {
            id: Uuid::new_v4(),
            width: 1080,
            height: 1920,
            png_base64: {
                use base64::Engine;
                let mut p = b"\x89PNG\r\n\x1a\n00000000".to_vec();
                p.extend(1080u32.to_be_bytes());
                p.extend(1920u32.to_be_bytes());
                base64::engine::general_purpose::STANDARD.encode(p)
            },
            controls: vec![],
        });
        publish2().json(&status).await.assert_status_ok();
        let sequence:AutomationSequence=serde_json::from_value(serde_json::json!({"actions":[{"id":"back","checkpoint_id":"back","kind":"direct","instruction":"","command":{"operation":"back"}}],"checks":[]})).unwrap();
        let command = PhoneCommandRequest {
            id: Uuid::new_v4(),
            expected_revision: 0,
            frame_id: status.frame.as_ref().map(|f| f.id),
            title: "Go back without AI".into(),
            sequence: sequence.clone(),
        };
        let commands = format!("/api/phones/{}/commands", next.id);
        assert!(!other
            .write(server.post(&commands))
            .json(&command)
            .await
            .status_code()
            .is_success());
        let queued = owner.write(server.post(&commands)).json(&command).await;
        queued.assert_status_ok();
        let admitted = queued.json::<PhoneSession>();
        assert_eq!(admitted.revision, 1);
        owner
            .write(server.post(&commands))
            .json(&command)
            .await
            .assert_status_ok();
        let changed = PhoneCommandRequest {
            title: "Different".into(),
            ..command.clone()
        };
        assert_eq!(
            owner
                .write(server.post(&commands))
                .json(&changed)
                .await
                .status_code()
                .as_u16(),
            409
        );
        let concurrent = PhoneCommandRequest {
            id: Uuid::new_v4(),
            ..command.clone()
        };
        assert_eq!(
            owner
                .write(server.post(&commands))
                .json(&concurrent)
                .await
                .status_code()
                .as_u16(),
            409
        );
        let mut task = admitted.tasks[0].clone();
        task.state = PhoneTaskState::Acting;
        task.steps.push(StepReceipt {
            action_id: "back".into(),
            state: StepState::Completed,
            message: "Done".into(),
        });
        status.task = Some(task.clone());
        assert_eq!(publish2().json(&status).await.status_code().as_u16(), 409);
        task.steps[0].state = StepState::Started;
        status.task = Some(task.clone());
        publish2().json(&status).await.assert_status_ok();
        let mut altered = task.clone();
        altered.sequence.as_mut().unwrap().actions[0].command = Some(DirectCommand::Restart {});
        status.task = Some(altered);
        assert_eq!(publish2().json(&status).await.status_code().as_u16(), 409);
        task.steps[0].state = StepState::Completed;
        task.state = PhoneTaskState::Completed;
        status.task = Some(task.clone());
        publish2().json(&status).await.assert_status_ok();
        publish2().json(&status).await.assert_status_ok();
        task.steps[0].message = "Rewritten".into();
        status.task = Some(task);
        assert_eq!(publish2().json(&status).await.status_code().as_u16(), 409);
        status.task = None;
        let stale = PhoneCommandRequest {
            id: Uuid::new_v4(),
            ..command.clone()
        };
        assert_eq!(
            owner
                .write(server.post(&commands))
                .json(&stale)
                .await
                .status_code()
                .as_u16(),
            409
        );
        let templates = owner
            .read(server.get(&format!("/api/apps/{app}/test-templates")))
            .await;
        templates.assert_status_ok();
        let catalog = templates.json::<TestTemplates>();
        assert_eq!(catalog.items.len(), 4);
        let save = SaveAuthoredTestsRequest {
            mutation_id: Uuid::new_v4(),
            source_task_id: None,
            expectations_confirmed: false,
            tests: vec![SaveAuthoredTest {
                template_id: Some(catalog.items[0].id.clone()),
                proposal_id: None,
                title: "My smoke test".into(),
                requirement: String::new(),
                sequence: AutomationSequence {
                    actions: catalog.items[0].definition.actions.clone(),
                    checks: catalog.items[0].definition.checks.clone(),
                },
            }],
        };
        let save_path = format!("/api/apps/{app}/test-library/from-recording");
        let result = owner.write(server.post(&save_path)).json(&save).await;
        result.assert_status_ok();
        let saved = result.json::<SavedAuthoredTests>();
        assert_eq!(
            owner
                .write(server.post(&save_path))
                .json(&save)
                .await
                .json::<SavedAuthoredTests>()
                .entry_ids,
            saved.entry_ids
        );
        let draft = owner
            .read(server.get(&format!(
                "/api/apps/{app}/test-library/{}/draft",
                saved.entry_ids[0]
            )))
            .await
            .json::<LibraryDraftResponse>();
        assert!(!draft.issues.is_empty());
        assert!(!draft.entry.ai_generated);
        let before = rows(
            &ctx.db,
            "SELECT id FROM test_library_entries WHERE app_id=$1",
            vec![app.into()],
        )
        .await
        .unwrap()
        .len();
        let mut invalid = save.clone();
        invalid.mutation_id = Uuid::new_v4();
        invalid.tests.push(SaveAuthoredTest {
            template_id: None,
            proposal_id: None,
            title: String::new(),
            requirement: String::new(),
            sequence,
        });
        assert!(!owner
            .write(server.post(&save_path))
            .json(&invalid)
            .await
            .status_code()
            .is_success());
        assert_eq!(
            rows(
                &ctx.db,
                "SELECT id FROM test_library_entries WHERE app_id=$1",
                vec![app.into()]
            )
            .await
            .unwrap()
            .len(),
            before
        );
        let generation = GenerateTestsRequest {
            engine: Some(DiscoveryEngine::MinitapV1),
            id: Uuid::new_v4(),
            session_id: next.id,
            expected_revision: 1,
            category: CoverageKind::Smoke,
            journey: "Go back in the sample app".into(),
            allow_writes: true,
            reuse_job_id: None,
        };
        let gen_path = format!("/api/apps/{app}/test-generations");
        owner
            .write(server.post(&gen_path))
            .json(&generation)
            .await
            .assert_status_ok();
        owner
            .write(server.post(&gen_path))
            .json(&generation)
            .await
            .assert_status_ok();
        assert!(!other
            .read(server.get(&format!("{gen_path}/{}", generation.id)))
            .await
            .status_code()
            .is_success());
        owner
            .read(server.get(&format!("{gen_path}/{}", generation.id)))
            .await
            .assert_status_ok();
        // Discovery intent and outcome cross the real route independently.
        let mut discovery = owner
            .read(server.get(&format!("{gen_path}/{}", generation.id)))
            .await
            .json::<PhoneTask>();
        discovery.state = PhoneTaskState::Acting;
        let source = DiscoverySnapshot {
            fingerprint: Some("synthetic".into()),
            id: Uuid::new_v4(),
            frame: status.frame.clone().unwrap(),
        };
        discovery.progress = Some(GenerationProgress {
            engine: Some(DiscoveryEngine::MinitapV1),
            source_job_id: None,
            journal: vec![],
            state: GenerationState::Discovering,
            proposals: vec![],
            snapshots: vec![source.clone()],
            trace: vec![],
            gaps: vec![],
            usage: AuthoringUsage::default(),
        });
        status.task = Some(discovery.clone());
        publish2().json(&status).await.assert_status_ok();
        let receipt = DiscoveryReceipt {
            id: Uuid::new_v4(),
            before_id: source.id,
            after_id: None,
            command: DirectCommand::Back {},
            outcome: DiscoveryOutcome::Pending,
        };
        discovery
            .progress
            .as_mut()
            .unwrap()
            .journal
            .push(receipt.clone());
        let mut premature = discovery.clone();
        premature.progress.as_mut().unwrap().journal[0].outcome = DiscoveryOutcome::Completed;
        premature.progress.as_mut().unwrap().journal[0].after_id = Some(source.id);
        premature
            .progress
            .as_mut()
            .unwrap()
            .trace
            .push(DirectCommand::Back {});
        status.task = Some(premature.clone());
        assert_eq!(publish2().json(&status).await.status_code().as_u16(), 409);
        status.task = Some(discovery.clone());
        publish2().json(&status).await.assert_status_ok();
        discovery = premature;
        status.task = Some(discovery.clone());
        publish2().json(&status).await.assert_status_ok();
        // Cursor/clock redraws can create a newer observation without a device action.
        let mut redraw = source.clone();
        redraw.id = Uuid::new_v4();
        discovery.progress.as_mut().unwrap().snapshots.push(redraw.clone());
        let second_receipt = DiscoveryReceipt {
            id: Uuid::new_v4(),
            before_id: redraw.id,
            after_id: None,
            command: DirectCommand::Back {},
            outcome: DiscoveryOutcome::Pending,
        };
        discovery.progress.as_mut().unwrap().journal.push(second_receipt);
        status.task = Some(discovery.clone());
        publish2().json(&status).await.assert_status_ok();
        let progress = discovery.progress.as_mut().unwrap();
        progress.journal[1].outcome = DiscoveryOutcome::Completed;
        progress.journal[1].after_id = Some(source.id);
        progress.trace.push(DirectCommand::Back {});
        status.task = Some(discovery.clone());
        assert_eq!(publish2().json(&status).await.status_code().as_u16(), 409);
        discovery.progress.as_mut().unwrap().journal[1].after_id = Some(redraw.id);
        status.task = Some(discovery.clone());
        publish2().json(&status).await.assert_status_ok();
        discovery.progress.as_mut().unwrap().state = GenerationState::Drafting;
        status.task = Some(discovery.clone());
        publish2().json(&status).await.assert_status_ok();
        let sequence:AutomationSequence=serde_json::from_value(serde_json::json!({"actions":[{"id":"back","checkpoint_id":"back","kind":"direct","instruction":"","command":{"operation":"back"}}],"checks":[]})).unwrap();
        let proposal = GenerationProposal {
            path_ids: vec![receipt.id],
            id: Uuid::new_v4(),
            title: "Sample navigation smoke".into(),
            category: CoverageKind::Smoke,
            sequence: sequence.clone(),
            requirement: "Review navigation".into(),
            questions: vec!["Confirm expected behavior".into()],
            source_ids: vec![source.id],
        };
        discovery.progress.as_mut().unwrap().proposals = vec![proposal.clone()];
        discovery.progress.as_mut().unwrap().state = GenerationState::NeedsInput;
        discovery.state = PhoneTaskState::Completed;
        let mut invented = discovery.clone();
        invented.progress.as_mut().unwrap().proposals[0].path_ids = vec![Uuid::new_v4()];
        status.task = Some(invented);
        assert_eq!(publish2().json(&status).await.status_code().as_u16(), 409);
        status.task = Some(discovery.clone());
        publish2().json(&status).await.assert_status_ok();
        let generated_save = SaveAuthoredTestsRequest {
            mutation_id: Uuid::new_v4(),
            source_task_id: Some(generation.id),
            expectations_confirmed: true,
            tests: vec![SaveAuthoredTest {
                template_id: None,
                proposal_id: Some(proposal.id),
                title: proposal.title,
                requirement: proposal.requirement,
                sequence,
            }],
        };
        let saved_generated = owner
            .write(server.post(&save_path))
            .json(&generated_save)
            .await;
        saved_generated.assert_status_ok();
        let ids = saved_generated.json::<SavedAuthoredTests>().entry_ids;
        let generated_entry = owner
            .read(server.get(&format!("/api/apps/{app}/test-library/{}", ids[0])))
            .await;
        generated_entry.assert_status_ok();
        assert!(generated_entry.json::<LibraryEntryResponse>().ai_generated);
        // Earlier authoring releases used the longer proposal marker; preserve their origin badge.
        exec(
            &ctx.db,
            "UPDATE test_library_drafts SET payload=jsonb_set(payload,'{content,provenance}',to_jsonb($2::text)) WHERE entry_id=$1",
            vec![ids[0].into(), format!("authored-task:{}:proposal:{}", generation.id, proposal.id).into()],
        ).await.unwrap();
        let legacy_entry = owner
            .read(server.get(&format!("/api/apps/{app}/test-library/{}", ids[0])))
            .await;
        legacy_entry.assert_status_ok();
        assert!(legacy_entry.json::<LibraryEntryResponse>().ai_generated);

        let listed = owner.read(server.get(&format!("/api/apps/{app}/test-library"))).await;
        listed.assert_status_ok();
        assert!(listed.json::<LibraryListResponse>().items.iter().any(|entry| entry.id == ids[0] && entry.ai_generated));

        assert_eq!(
            owner
                .write(server.post(&save_path))
                .json(&generated_save)
                .await
                .json::<SavedAuthoredTests>()
                .entry_ids,
            ids
        );
        let repeat = GenerateTestsRequest {
            id: Uuid::new_v4(),
            expected_revision: 2,
            reuse_job_id: Some(generation.id),
            ..generation.clone()
        };
        let reused = owner.write(server.post(&gen_path)).json(&repeat).await;
        reused.assert_status_ok();
        let reused_task = reused
            .json::<PhoneSession>()
            .tasks
            .into_iter()
            .find(|t| t.id == repeat.id)
            .unwrap();
        let progress = reused_task.progress.unwrap();
        assert_eq!(progress.source_job_id, Some(generation.id));
        assert_eq!(progress.journal.len(), 2);
        assert_eq!(progress.usage.calls, 0);
        status.task = None;
        exec(
            &ctx.db,
            "UPDATE phone_sessions SET expires_at=now()-interval '1 second' WHERE id=$1",
            vec![next.id.into()],
        )
        .await
        .unwrap();
        let expired = owner
            .read(server.get(&format!("/api/phones/{}", next.id)))
            .await;
        assert_eq!(
            expired.json::<PhoneSession>().state,
            PhoneState::Quarantined
        );
        assert_eq!(
            rows(
                &ctx.db,
                "SELECT resource FROM execution_reservations WHERE session_id=$1",
                vec![next.id.into()]
            )
            .await
            .unwrap()
            .len(),
            2
        );
    })
    .await;
}
