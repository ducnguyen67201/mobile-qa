//! Draft shape validation must not accidentally turn missing form fields into
//! undecodable payloads, or permit those same fields into a published manifest.
use mobile_qa_contracts::{execution::*, test_library::*};
use uuid::Uuid;
fn case() -> LibraryDraftDefinition {
    let definition: TestDefinition = serde_json::from_str(include_str!(
        "../../../contracts/fixtures/execution/persistence-case.json"
    ))
    .unwrap();
    definition.into()
}
#[test]
fn drafts_round_trip_and_publication_preserves_the_execution_shape() {
    let mut draft = case();
    draft.allocate(2);
    let encoded = serde_json::to_value(&draft).unwrap();
    assert_eq!(
        serde_json::from_value::<LibraryDraftDefinition>(encoded).unwrap(),
        draft
    );
    let published = draft.published().unwrap();
    published.validate().unwrap();
    let encoded = serde_json::to_string(&published).unwrap();
    assert_eq!(
        serde_json::from_str::<TestDefinition>(&encoded).unwrap(),
        published
    );
    let TestDefinition::Case(case) = published else {
        panic!()
    };
    assert_eq!(case.provenance, "user_authored");
    assert_eq!(case.version, 2);
}
#[test]
fn incomplete_case_is_saveable_but_not_publishable() {
    let LibraryDraftDefinition::Case(mut c) = case() else {
        panic!()
    };
    c.title.clear();
    c.actions.clear();
    c.checks.clear();
    let draft = LibraryDraftDefinition::Case(c);
    draft.check_bounds().unwrap();
    assert!(draft.issues().iter().any(|i| i.field == "title"));
    assert!(draft.published().unwrap().validate().is_err());
}
#[test]
fn incomplete_plan_uses_null_instead_of_a_fake_profile_identity() {
    let mut p = PlanDraftContent {
        key: "release".into(),
        version: 1,
        title: "Release".into(),
        suite_version_ids: vec![],
        cases: vec![],
        profile_id: None,
        budget: ExecutionBudget {
            duration_seconds: 300,
            max_steps: 80,
            artifact_bytes: 10485760,
        },
        diagnostic_retries: 0,
        exclusions: vec![],
    };
    let draft = LibraryDraftDefinition::Plan(p.clone());
    draft.check_bounds().unwrap();
    assert!(draft.published().is_err());
    assert_eq!(
        serde_json::to_value(&draft).unwrap()["content"]["profile_id"],
        serde_json::Value::Null
    );
    p.profile_id = Some(Uuid::new_v4());
    p.cases.push(CaseSelection {
        case_version_id: Uuid::new_v4(),
        data_variant: "default".into(),
        required: true,
    });
    let draft = LibraryDraftDefinition::Plan(p);
    draft.published().unwrap().validate().unwrap();
}
#[test]
fn oversized_unknown_and_forged_shapes_are_rejected() {
    let LibraryDraftDefinition::Case(mut c) = case() else {
        panic!()
    };
    c.title = "x".repeat(201);
    assert!(LibraryDraftDefinition::Case(c).check_bounds().is_err());
    let mut value = serde_json::to_value(case()).unwrap();
    value["content"]["approved"] = true.into();
    assert!(serde_json::from_value::<LibraryDraftDefinition>(value).is_err());
    let mut value = serde_json::to_value(case()).unwrap();
    value["content"]["version"] = (-1).into();
    assert!(serde_json::from_value::<LibraryDraftDefinition>(value).is_err());
}
#[test]
fn suite_and_typed_error_details_round_trip() {
    let draft = LibraryDraftDefinition::Suite(SuiteDefinition {
        key: "suite".into(),
        version: 1,
        title: "".into(),
        cases: vec![],
    });
    draft.check_bounds().unwrap();
    assert!(!draft.issues().is_empty());
    assert_eq!(
        serde_json::from_value::<LibraryDraftDefinition>(serde_json::to_value(&draft).unwrap())
            .unwrap(),
        draft
    );
    let error = LibraryErrorDetails::StaleRevision {
        entry_id: Some(Uuid::new_v4()),
        current_revision: 4,
    };
    assert_eq!(
        serde_json::from_value::<LibraryErrorDetails>(serde_json::to_value(&error).unwrap())
            .unwrap(),
        error
    );
}
