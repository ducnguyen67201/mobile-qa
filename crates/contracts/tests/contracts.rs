use mobile_qa_contracts::worker::{ContractProbe, FakeExecutionRequest};
use serde_json::{json, Value};
fn fixture(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../contracts/fixtures")
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
#[test]
fn probe_roundtrips_without_losing_precision() {
    let raw = fixture("probe.json");
    let probe: ContractProbe = serde_json::from_value(raw.clone()).unwrap();
    probe.validate().unwrap();
    assert_eq!(serde_json::to_value(probe).unwrap(), raw);
}
#[test]
fn optional_null_is_accepted_then_omitted() {
    let mut raw = fixture("probe.json");
    raw["optional_note"] = Value::Null;
    let probe: ContractProbe = serde_json::from_value(raw).unwrap();
    assert!(serde_json::to_value(probe)
        .unwrap()
        .get("optional_note")
        .is_none());
}
#[test]
fn rejects_invalid_probe_boundaries() {
    for (key, bad) in [
        ("run_id", json!("bad")),
        ("observed_at", json!("yesterday")),
        ("scenario", json!({"kind":"unknown"})),
        ("counter", json!(9007199254740993_u64)),
    ] {
        let mut raw = fixture("probe.json");
        raw[key] = bad;
        assert!(
            serde_json::from_value::<ContractProbe>(raw).is_err(),
            "{key}"
        );
    }
    let mut raw = fixture("probe.json");
    raw.as_object_mut().unwrap().remove("nullable_note");
    assert!(serde_json::from_value::<ContractProbe>(raw).is_err());
    for counter in ["", "01", "-1", "1.2"] {
        let mut raw = fixture("probe.json");
        raw["counter"] = json!(counter);
        assert!(serde_json::from_value::<ContractProbe>(raw)
            .unwrap()
            .validate()
            .is_err());
    }
}
#[test]
fn scenario_and_version_are_validated() {
    for kind in ["pass", "fail", "blocked"] {
        serde_json::from_value::<FakeExecutionRequest>(fixture(&format!("{kind}.json")))
            .unwrap()
            .validate()
            .unwrap();
    }
    for version in [json!(0), json!(2), json!("1"), Value::Null] {
        let mut raw = fixture("pass.json");
        raw["version"] = version;
        assert!(serde_json::from_value::<FakeExecutionRequest>(raw)
            .map(|r| r.validate().is_err())
            .unwrap_or(true));
    }
    let mut raw = fixture("pass.json");
    raw.as_object_mut().unwrap().remove("version");
    assert!(serde_json::from_value::<FakeExecutionRequest>(raw).is_err());
}
