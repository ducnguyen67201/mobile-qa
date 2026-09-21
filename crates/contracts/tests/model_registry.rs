use mobile_qa_contracts::model_registry::*;

fn definition() -> ModelDefinition {
    ModelDefinition {
        reference: ModelReference {
            key: "openai.navigation".into(),
            revision: 1,
        },
        display_name: "Navigation model".into(),
        provider: ModelProvider::OpenAi,
        provider_model: "synthetic-model".into(),
        capabilities: vec![
            ModelCapability::MinitapNavigation,
            ModelCapability::StructuredAuthoring,
        ],
    }
}

#[test]
fn validates_stable_identity_and_unique_capabilities() {
    definition().validate().unwrap();
    for key in ["", "UPPER", ".leading", "space key"] {
        let mut invalid = definition();
        invalid.reference.key = key.into();
        assert!(invalid.validate().is_err());
    }
    let mut invalid = definition();
    invalid.reference.revision = 0;
    assert!(invalid.validate().is_err());
    let mut invalid = definition();
    invalid
        .capabilities
        .push(ModelCapability::MinitapNavigation);
    assert!(invalid.validate().is_err());
}

#[test]
fn legacy_binding_round_trips_without_changing_new_shape() {
    let legacy: ModelBinding = serde_json::from_str("\"gpt-4.1\"").unwrap();
    assert_eq!(serde_json::to_string(&legacy).unwrap(), "\"gpt-4.1\"");
    let registered = ModelBinding::Registered(definition().reference);
    assert_eq!(
        serde_json::to_value(registered).unwrap(),
        serde_json::json!({"key":"openai.navigation","revision":1})
    );
}
