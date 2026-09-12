//! Trusted, explicit execution maintenance. JSON files contain definitions, never secrets.
use crate::{
    errors::{ApiFailure, ApiResult},
    services::{scheduler, test_definitions as defs, worker_auth},
};
use async_trait::async_trait;
use loco_rs::{
    app::AppContext,
    task::{Task, TaskInfo, Vars},
};
use mobile_qa_contracts::execution::*;
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
fn purpose(v: &Vars) -> ApiResult<ApprovalPurpose> {
    match arg(v, "purpose")? {
        "business" => Ok(ApprovalPurpose::Business),
        "executability" => Ok(ApprovalPurpose::Executability),
        _ => Err(ApiFailure::invalid(
            "Purpose must be business or executability",
        )),
    }
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
        "grant-reviewer" => defs::grant(ctx, actor, app, id(v, "user")?, purpose(v)?).await?,
        "approve" => {
            defs::approve(
                ctx,
                actor,
                app,
                id(v, "definition")?,
                arg(v, "hash")?,
                purpose(v)?,
            )
            .await?;
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
            detail: "Import and review tests; register workers/profiles; reconcile leases".into(),
        }
    }
    async fn run(&self, ctx: &AppContext, vars: &Vars) -> loco_rs::Result<()> {
        execute(ctx, vars)
            .await
            .map_err(|e| loco_rs::Error::string(&e.message))
    }
}
