//! Shared adapter admission; no device calls and no inference from missing credentials.
use super::execution_store::decode;
use crate::{
    errors::{ApiFailure, ApiResult},
    models::_entities::execution_profiles,
};
use mobile_qa_contracts::execution::*;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

pub fn case_matches(profile: &ExecutionProfile, case: &CaseDefinition) -> ApiResult<()> {
    profile.validate().map_err(ApiFailure::invalid)?;
    if !profile.qualified
        || case.package != profile.package
        || case.adapter != profile.adapter
        || case.checks.iter().any(|c| c.method == CheckMethod::Manual)
        || (profile.execution_context.is_some()
            && case
                .actions
                .iter()
                .any(|a| a.kind == ActionKind::Navigate || a.checkpoint_id == "preflight"))
    {
        return Err(ApiFailure::invalid(
            "This test needs a compatible app setup and automatic actions",
        ));
    }
    Ok(())
}
pub fn duration(profile: &ExecutionProfile, case: &CaseDefinition) -> u64 {
    u64::from(case.budget.duration_seconds)
        + profile
            .execution_context
            .as_ref()
            .map_or(0, |c| c.stages.overhead())
}
pub async fn assigned(
    db: &impl ConnectionTrait,
    app: Uuid,
    package: &str,
) -> ApiResult<ExecutionProfile> {
    let candidates = execution_profiles::Entity::find()
        .filter(execution_profiles::Column::AppId.eq(app))
        .all(db)
        .await?;
    for row in candidates {
        let p: ExecutionProfile = decode(row.payload)?;
        if p.adapter == "android_direct_v1"
            && p.package == package
            && p.qualified
            && p.validate().is_ok()
        {
            return Ok(p);
        }
    }
    Err(ApiFailure::invalid(
        "Finish setting up this app before reviewing or running its tests",
    ))
}
pub async fn adapter(db: &impl ConnectionTrait, app: Uuid, package: &str) -> ApiResult<String> {
    // Legacy demo drafts can still be written before a device is connected.
    if package == "ai.mobileqa.demo" {
        return Ok("demo_persistence_v1".into());
    }
    Ok(assigned(db, app, package).await?.adapter)
}
