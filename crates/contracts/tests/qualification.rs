use mobile_qa_contracts::worker::qualification::{QualificationRequest, QualificationResult};
use serde_json::{json, Value};
fn request() -> Value {
    json!({
        "version":1,"attempt_id":"00000000-0000-0000-0000-000000000001","case_id":"persist-task-v1",
        "apk_path":"/tmp/app.apk","expected_apk_sha256":"a".repeat(64),"package":"ai.mobileqa.demo",
        "activity":"ai.mobileqa.demo/.MainActivity","serial":"emulator-5554","profile_path":"/tmp/profile.toml","output_root":"/tmp/out"
    })
}
#[test]
fn rejects_invalid_request_semantics() {
    serde_json::from_value::<QualificationRequest>(request())
        .unwrap()
        .validate()
        .unwrap();
    for (key, value) in [
        ("version", json!(2)),
        ("serial", json!("emulator-5556")),
        ("expected_apk_sha256", json!("x".repeat(64))),
        ("package", json!("customer.app")),
    ] {
        let mut raw = request();
        raw[key] = value;
        assert!(serde_json::from_value::<QualificationRequest>(raw)
            .unwrap()
            .validate()
            .is_err());
    }
    for value in [json!(true), json!("1")] {
        let mut raw = request();
        raw["version"] = value;
        assert!(serde_json::from_value::<QualificationRequest>(raw).is_err());
    }
}
#[test]
fn result_cannot_pass_without_observed_evidence() {
    let mut raw = json!({"version":1,"attempt_id":"00000000-0000-0000-0000-000000000001","case_id":"persist-task-v1",
        "outcome":"blocked","reason_code":"prerequisite_unavailable","expected_behavior":"Persist","observed_behavior":"Unavailable",
        "started_at":"2026-09-12T01:00:00Z","ended_at":"2026-09-12T01:01:00Z","requested_build_sha256":"a".repeat(64),
        "observed_build_sha256":null,"device_inventory":{},"model_profile_sha256":"b".repeat(64),"reset":"not_started","phase_ms":{},"artifacts":[],"usage":[]});
    serde_json::from_value::<QualificationResult>(raw.clone())
        .unwrap()
        .validate()
        .unwrap();
    raw["outcome"] = json!("passed");
    assert!(serde_json::from_value::<QualificationResult>(raw.clone())
        .unwrap()
        .validate()
        .is_err());
    raw["outcome"] = json!("blocked");
    raw["ended_at"] = json!("2025-01-01T00:00:00Z");
    assert!(serde_json::from_value::<QualificationResult>(raw)
        .unwrap()
        .validate()
        .is_err());
}
