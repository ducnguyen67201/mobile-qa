use mobile_qa_contracts::execution::*;
use mobile_qa_contracts::model_registry::{ModelBinding, ModelReference};
use uuid::Uuid;
fn case() -> TestDefinition {
    serde_json::from_str(include_str!(
        "../../../contracts/fixtures/execution/persistence-case.json"
    ))
    .unwrap()
}
#[test]
fn approved_import_shape_has_separate_actions_and_checks() {
    let d = case();
    d.validate().unwrap();
    let raw = serde_json::to_value(&d).unwrap();
    assert_eq!(serde_json::from_value::<TestDefinition>(raw).unwrap(), d);
}
#[test]
fn invalid_definition_references_and_paths_are_rejected() {
    let TestDefinition::Case(c) = case() else {
        panic!()
    };
    for mutation in 0..7 {
        let mut bad = c.clone();
        match mutation {
            0 => bad.actions[0].checkpoint_id = "../escape".into(),
            1 => bad.checks[1].checkpoint_id = "absent".into(),
            2 => bad.checks[0].prerequisite_check_ids = vec!["persisted".into()],
            3 => bad.actions[0].instruction = "${password}".into(),
            4 => bad.budget.max_steps = 0,
            5 => bad.checks[1].resource_id = "foreign:id/task".into(),
            _ => bad.checks[1].required = false,
        };
        assert!(TestDefinition::Case(bad).validate().is_err());
    }
}
#[test]
fn unknown_fields_are_not_imported() {
    let mut v = serde_json::to_value(case()).unwrap();
    v["content"]["approved"] = true.into();
    assert!(serde_json::from_value::<TestDefinition>(v).is_err());
}
#[test]
fn manual_methods_are_defined_but_not_implicitly_executable() {
    let TestDefinition::Case(mut c) = case() else {
        panic!()
    };
    c.checks[1].method = CheckMethod::Manual;
    TestDefinition::Case(c).validate().unwrap();
}
#[test]
fn real_profile_cannot_overstate_demo_installer_capacity() {
    let mut profile = ExecutionProfile {
        execution_context: None,
        id: uuid::Uuid::nil(),
        name: "Demo".into(),
        driver: Driver::Minitap,
        package: "ai.mobileqa.demo".into(),
        adapter: "demo_persistence_v1".into(),
        device_identity: "local".into(),
        image: "pinned".into(),
        model: Some(ModelBinding::Registered(ModelReference {
            key: "pinned".into(),
            revision: 1,
        })),
        qualified: false,
        qualification_reference: String::new(),
        max_apk_bytes: 104857600,
    };
    profile.validate().unwrap();
    profile.max_apk_bytes += 1;
    assert!(profile.validate().is_err());
}

#[test]
fn qualified_context_is_explicit_and_legacy_serialization_stays_absent() {
    use mobile_qa_contracts::execution_lifecycle::*;
    let mut p: ExecutionProfile = serde_json::from_value(serde_json::json!({
        "id":Uuid::new_v4(),"name":"Local notes","driver":"direct","package":"com.example.notes",
        "adapter":"android_direct_v1","device_identity":"phone-one","image":"system-images;android-35;google_apis;x86_64",
        "model":"","qualified":true,"qualification_reference":"qualified-local-state-v1","max_apk_bytes":104857600
    })).unwrap();
    assert!(!serde_json::to_value(&p)
        .unwrap()
        .as_object()
        .unwrap()
        .contains_key("execution_context"));
    assert!(p.validate().is_err());
    let check = ExpectedCheck {
        id: "empty".into(),
        checkpoint_id: "preflight".into(),
        description: "Input starts empty".into(),
        method: CheckMethod::UiPropertyEqualsV1,
        resource_id: "com.example.notes:id/input".into(),
        text_filter: String::new(),
        property: UiProperty::Text,
        expected: String::new(),
        ready_resource_id: "com.example.notes:id/input".into(),
        prerequisite_check_ids: vec![],
        required: true,
        observation_seconds: 5,
    };
    p.execution_context = Some(ExecutionContextV1 {
        schema_version: 1,
        adapter_revision: "android_direct_v1".into(),
        verifier_revision: "ui_v1".into(),
        worker_runtime_revision: "direct_v1".into(),
        reset_policy_hash: "a".repeat(64),
        qualified_profile_id: p.id,
        package: p.package.clone(),
        launch_component: "com.example.notes/.MainActivity".into(),
        image: p.image.clone(),
        abi: "x86_64".into(),
        width: 1080,
        height: 1920,
        density: 420,
        locale: "en-US".into(),
        timezone: "Etc/UTC".into(),
        state_scope: "local_only".into(),
        qualification_reference: p.qualification_reference.clone(),
        starting_checks: vec![check],
        stages: StageBudgets {
            boot_seconds: 180,
            install_seconds: 90,
            start_seconds: 30,
            cleanup_seconds: 240,
        },
    });
    assert!(p.validate().is_ok());
    for value in ["remote", ""] {
        let mut changed = p.clone();
        changed.execution_context.as_mut().unwrap().state_scope = value.into();
        assert!(changed.validate().is_err());
    }
    let mut changed = p.clone();
    changed.execution_context.as_mut().unwrap().starting_checks[0].expected =
        "${task_title}".into();
    assert!(changed.validate().is_err());
    let mut changed = p.clone();
    changed.execution_context.as_mut().unwrap().package = "another.app".into();
    assert!(changed.validate().is_err());
    let mut changed = p.clone();
    changed.driver = Driver::Minitap;
    assert!(changed.validate().is_err());
    let mut changed = p.clone();
    changed
        .execution_context
        .as_mut()
        .unwrap()
        .stages
        .boot_seconds = 0;
    assert!(changed.validate().is_err());
}
