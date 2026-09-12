use mobile_qa_contracts::execution::*;
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
        id: uuid::Uuid::nil(),
        name: "Demo".into(),
        driver: Driver::Minitap,
        package: "ai.mobileqa.demo".into(),
        adapter: "demo_persistence_v1".into(),
        device_identity: "local".into(),
        image: "pinned".into(),
        model: "pinned".into(),
        qualified: false,
        qualification_reference: String::new(),
        max_apk_bytes: 104857600,
    };
    profile.validate().unwrap();
    profile.max_apk_bytes += 1;
    assert!(profile.validate().is_err());
}
