//! Real PostgreSQL coverage for immutable nonsecret model registration.
mod support;

use mobile_qa::services::model_registry;
use mobile_qa_contracts::model_registry::*;
use support::*;

fn definition(revision: u32) -> ModelDefinition {
    ModelDefinition {
        reference: ModelReference {
            key: "synthetic.registry".into(),
            revision,
        },
        display_name: "Synthetic registry model".into(),
        provider: ModelProvider::OpenAi,
        provider_model: "synthetic-model".into(),
        capabilities: vec![ModelCapability::MinitapNavigation],
    }
}

#[tokio::test]
async fn registry_is_immutable_resolvable_and_retirable() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|_server, ctx| async move {
        let model = definition(1);
        let first = model_registry::register(&ctx.db, &model).await.unwrap();
        assert_eq!(first.reference, model.reference);
        assert_eq!(
            model_registry::register(&ctx.db, &model).await.unwrap(),
            first
        );
        let mut mutation = model.clone();
        mutation.provider_model = "changed".into();
        assert!(model_registry::register(&ctx.db, &mutation).await.is_err());
        assert!(model_registry::resolve_for_new_work(
            &ctx.db,
            Some(&ModelBinding::Registered(model.reference.clone())),
            &[ModelCapability::MinitapNavigation],
        )
        .await
        .unwrap()
        .is_some());
        model_registry::retire(&ctx.db, &model.reference)
            .await
            .unwrap();
        assert!(model_registry::resolve_for_new_work(
            &ctx.db,
            Some(&ModelBinding::Registered(model.reference)),
            &[],
        )
        .await
        .is_err());
    })
    .await;
}
