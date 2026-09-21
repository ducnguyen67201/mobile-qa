//! Trusted, explicit execution maintenance. JSON files contain definitions, never secrets.
use crate::{
    errors::{ApiFailure, ApiResult},
    services::{model_registry, scheduler, test_definitions as defs, worker_auth},
};
use async_trait::async_trait;
use loco_rs::{
    app::AppContext,
    task::{Task, TaskInfo, Vars},
};
use mobile_qa_contracts::execution::*;
use mobile_qa_contracts::model_registry::{ModelDefinition, ModelReference};
use uuid::Uuid;
pub struct Execution;
fn arg<'a>(v: &'a Vars, name: &str) -> ApiResult<&'a str> {
    v.cli_arg(name)
        .map_err(|_| ApiFailure::invalid(format!("Missing {name}")))
}
fn id(v: &Vars, name: &str) -> ApiResult<Uuid> {
    arg(v, name)?
        .parse()
        .map_err(|_| ApiFailure::invalid(format!("Invalid {name} UUID")))
}
fn read<T: serde::de::DeserializeOwned>(path: &str) -> ApiResult<T> {
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > 1048576 {
        return Err(ApiFailure::invalid("Import exceeds 1 MiB"));
    }
    serde_json::from_slice(&std::fs::read(path)?)
        .map_err(|_| ApiFailure::invalid("Invalid import document"))
}
pub async fn execute(ctx: &AppContext, v: &Vars) -> ApiResult<()> {
    let action = arg(v, "action")?;
    if action == "reconcile" {
        scheduler::reconcile(ctx).await?;
        return crate::services::run_artifacts::cleanup_pending(ctx).await;
    }
    if action == "register-model" {
        let definition: ModelDefinition = read(arg(v, "file")?)?;
        let resolved = model_registry::register(&ctx.db, &definition).await?;
        println!(
            "{}",
            serde_json::to_string(&resolved).map_err(|_| ApiFailure::internal())?
        );
        return Ok(());
    }
    if matches!(action, "show-model" | "retire-model") {
        let reference = ModelReference {
            key: arg(v, "key")?.to_owned(),
            revision: arg(v, "revision")?
                .parse()
                .map_err(|_| ApiFailure::invalid("Invalid model revision"))?,
        };
        if action == "retire-model" {
            model_registry::retire(&ctx.db, &reference).await?;
        } else {
            let (resolved, retired) = model_registry::resolve(&ctx.db, &reference).await?;
            if retired {
                return Err(ApiFailure::invalid("Model revision is retired"));
            }
            println!(
                "{}",
                serde_json::to_string(&resolved).map_err(|_| ApiFailure::internal())?
            );
        }
        return Ok(());
    }
    let actor = id(v, "actor")?;
    let app = id(v, "app")?;
    match action {
        "import" => {
            let definition = read(arg(v, "file")?)?;
            let d = defs::import(
                ctx,
                actor,
                DefinitionImport {
                    app_id: app,
                    definition,
                },
            )
            .await?;
            println!("definition_id={} content_hash={}", d.id, d.content_hash);
        }
        "register-profile" => {
            defs::register_profile(ctx, actor, app, read(arg(v, "file")?)?).await?
        }
        "register-worker" => {
            let token = std::env::var("MOBILE_QA_WORKER_TOKEN")
                .map_err(|_| ApiFailure::invalid("Inject MOBILE_QA_WORKER_TOKEN"))?;
            worker_auth::register(ctx, actor, app, id(v, "worker")?, id(v, "profile")?, &token)
                .await?;
        }
        "recover" => {
            scheduler::recover(ctx, actor, app, id(v, "attempt")?, arg(v, "evidence")?).await?
        }
        _ => return Err(ApiFailure::invalid("Unknown execution operation")),
    }
    tracing::info!(%actor,app_id=%app,%action,"Execution maintenance completed");
    Ok(())
}
#[async_trait]
impl Task for Execution {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "execution".into(),
            detail: "Import saved tests; register workers/profiles; reconcile leases".into(),
        }
    }
    async fn run(&self, ctx: &AppContext, vars: &Vars) -> loco_rs::Result<()> {
        execute(ctx, vars)
            .await
            .map_err(|e| loco_rs::Error::string(&e.message))
    }
}
