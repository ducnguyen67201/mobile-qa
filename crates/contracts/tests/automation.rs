//! Additive wire compatibility and semantic validation for direct commands.
use mobile_qa_contracts::{automation::*, execution::*};
use serde_json::json;
#[test]
fn legacy_action_round_trips_without_changing_serialized_bytes() {
    let old =
        r#"{"id":"open","kind":"navigate","instruction":"Open settings","checkpoint_id":"opened"}"#;
    let action: TestAction = serde_json::from_str(old).unwrap();
    action.validate("ai.mobileqa.demo").unwrap();
    assert_eq!(serde_json::to_string(&action).unwrap(), old);
}
#[test]
fn direct_commands_reject_invalid_shapes_targets_and_kind_disagreement() {
    assert!(serde_json::from_value::<DirectCommand>(
        json!({"operation":"back","target":{"by":"resource_id","value":"x"}})
    )
    .is_err());
    let mut action:TestAction=serde_json::from_value(json!({"id":"input","checkpoint_id":"input","kind":"direct","instruction":"","command":{"operation":"set_text","target":{"by":"resource_id","value":"ai.mobileqa.demo:id/input"},"text":"Xin chào\n\"quoted\""}})).unwrap();
    action.validate("ai.mobileqa.demo").unwrap();
    assert!(action.validate("another.app").is_err());
    action.kind = ActionKind::Navigate;
    assert!(action.validate("ai.mobileqa.demo").is_err());
}
#[test]
fn direct_and_mixed_sequences_expose_ai_explicitly_and_reject_duplicate_ids() {
    let action:TestAction=serde_json::from_value(json!({"id":"back","checkpoint_id":"back","kind":"direct","instruction":"","command":{"operation":"back"}})).unwrap();
    let mut sequence = AutomationSequence {
        actions: vec![action.clone()],
        checks: vec![],
    };
    sequence.validate("ai.mobileqa.demo").unwrap();
    assert!(!sequence.uses_ai());
    sequence.actions.push(action);
    assert!(sequence.validate("ai.mobileqa.demo").is_err());
    sequence.actions[1] = serde_json::from_value(
        json!({"id":"ai","checkpoint_id":"ai","kind":"navigate","instruction":"Explore"}),
    )
    .unwrap();
    sequence.validate("ai.mobileqa.demo").unwrap();
    assert!(sequence.uses_ai());
}

#[test]
fn legacy_generation_payloads_keep_absent_discovery_fields() {
    let request = json!({"id":uuid::Uuid::nil(),"session_id":uuid::Uuid::nil(),"expected_revision":0,"category":"smoke","journey":"","allow_writes":false,"reuse_job_id":null});
    let parsed: GenerateTestsRequest = serde_json::from_value(request.clone()).unwrap();
    assert_eq!(parsed.engine, None);
    assert_eq!(serde_json::to_value(parsed).unwrap(), request);
    let progress = json!({"state":"ready","proposals":[],"snapshots":[],"trace":[],"gaps":[],"usage":{"calls":1,"input_tokens":1,"output_tokens":1,"unknown_calls":0}});
    let parsed: GenerationProgress = serde_json::from_value(progress.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), progress);
    assert!(serde_json::from_value::<DiscoveryCall>(
        json!({"kind":"execute","id":uuid::Uuid::nil(),"command":{"operation":"shell","text":"no"}})
    )
    .is_err());
}
