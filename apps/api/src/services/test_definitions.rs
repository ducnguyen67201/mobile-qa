//! Immutable saved versions and technical execution resolution.
use super::{apps, execution_store::*};
use crate::{
    errors::{ApiFailure, ApiResult},
    models::_entities::users,
};
use loco_rs::app::AppContext;
use mobile_qa_contracts::execution::*;
use mobile_qa_contracts::model_registry::{ModelBinding, ModelCapability};
use sea_orm::{ConnectionTrait, EntityTrait, TransactionTrait};
use uuid::Uuid;

pub async fn operator(ctx: &AppContext, actor: Uuid, app: Uuid) -> ApiResult<()> {
    let app = apps::authorized(ctx, actor, app).await?;
    let user = users::Entity::find_by_id(actor)
        .one(&ctx.db)
        .await?
        .ok_or_else(ApiFailure::unauthorized)?;
    if user.disabled_at.is_some()
        || apps::membership(ctx, actor, app.organization_id)
            .await?
            .role
            != "operator"
    {
        return Err(ApiFailure::new(
            403,
            "operator_required",
            "An active operator is required",
        ));
    }
    Ok(())
}

pub async fn get(db: &impl ConnectionTrait, app: Uuid, id: Uuid) -> ApiResult<DefinitionResponse> {
    let r = one(
        db,
        "SELECT * FROM execution_definitions WHERE id=$1 AND app_id=$2",
        vec![id.into(), app.into()],
    )
    .await?;
    let approvals = rows(
        db,
        "SELECT * FROM execution_approvals WHERE definition_id=$1 ORDER BY purpose",
        vec![id.into()],
    )
    .await?
    .iter()
    .map(|r| {
        Ok(DefinitionApproval {
            purpose: decode(serde_json::Value::String(field(r, "purpose")?))?,
            actor_id: field(r, "actor_id")?,
            content_hash: field(r, "content_hash")?,
            approved_at: field(r, "approved_at")?,
        })
    })
    .collect::<ApiResult<Vec<_>>>()?;
    Ok(DefinitionResponse {
        id,
        app_id: app,
        content_hash: field(&r, "content_hash")?,
        definition: decode(field(&r, "payload")?)?,
        approvals,
    })
}

pub async fn import(
    ctx: &AppContext,
    actor: Uuid,
    input: DefinitionImport,
) -> ApiResult<DefinitionResponse> {
    operator(ctx, actor, input.app_id).await?;
    input.definition.validate().map_err(ApiFailure::invalid)?;
    let payload = json(&input.definition)?;
    let bytes = serde_json::to_vec(&payload).map_err(|_| ApiFailure::internal())?;
    if bytes.len() > 1048576 {
        return Err(ApiFailure::invalid("Definition exceeds 1 MiB"));
    }
    let digest = hash(bytes);
    let (kind, key, version) = input.definition.identity();
    if version > i32::MAX as u32 {
        return Err(ApiFailure::invalid("Version exceeds supported range"));
    }
    let tx = ctx.db.begin().await?;
    // Parent app lock serializes imports/saves against run resolution.
    one(
        &tx,
        "SELECT id FROM apps WHERE id=$1 FOR UPDATE",
        vec![input.app_id.into()],
    )
    .await?;
    match &input.definition {
        TestDefinition::Suite(s) => {
            for c in &s.cases {
                require_case(&tx, input.app_id, c.case_version_id, false).await?;
            }
        }
        TestDefinition::Plan(p) => {
            profile(&tx, input.app_id, p.profile_id).await?;
            for id in &p.suite_version_ids {
                if !matches!(
                    get(&tx, input.app_id, *id).await?.definition,
                    TestDefinition::Suite(_)
                ) {
                    return Err(ApiFailure::invalid("Plan reference must be a suite"));
                }
            }
            for c in &p.cases {
                require_case(&tx, input.app_id, c.case_version_id, false).await?;
            }
        }
        _ => {}
    }
    let id = Uuid::new_v4();
    exec(
        &tx,
        "INSERT INTO execution_definitions(id,app_id,kind,logical_key,version,content_hash,\
            payload,author_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(app_id,kind,\
            logical_key,version) DO NOTHING",
        vec![
            id.into(),
            input.app_id.into(),
            word(&kind).into(),
            key.into(),
            (version as i32).into(),
            digest.clone().into(),
            payload.into(),
            actor.into(),
        ],
    )
    .await?;
    let r = one(
        &tx,
        "SELECT id,content_hash FROM execution_definitions WHERE app_id=$1 AND kind=$2 AND \
            logical_key=$3 AND version=$4",
        vec![
            input.app_id.into(),
            word(&kind).into(),
            key.into(),
            (version as i32).into(),
        ],
    )
    .await?;
    if field::<String>(&r, "content_hash")? != digest {
        return Err(conflict(
            "Version content is immutable; import a new version",
        ));
    }
    let result = get(&tx, input.app_id, field(&r, "id")?).await?;
    super::test_library_mutations::link_import(&tx, actor, input.app_id, &result).await?;
    tx.commit().await?;
    Ok(result)
}

async fn require_case(
    db: &impl ConnectionTrait,
    app: Uuid,
    id: Uuid,
    admission: bool,
) -> ApiResult<DefinitionResponse> {
    let d = get(db, app, id).await?;
    if admission {
        super::test_library::admitted(db, app, id).await?;
    }
    if !matches!(d.definition, TestDefinition::Case(_)) {
        return Err(ApiFailure::invalid("A saved case version is required"));
    }
    Ok(d)
}

pub async fn profile(
    db: &impl ConnectionTrait,
    app: Uuid,
    id: Uuid,
) -> ApiResult<ExecutionProfile> {
    decode(field(
        &one(
            db,
            "SELECT payload FROM execution_profiles WHERE id=$1 AND app_id=$2",
            vec![id.into(), app.into()],
        )
        .await?,
        "payload",
    )?)
}

pub async fn register_profile(
    ctx: &AppContext,
    actor: Uuid,
    app: Uuid,
    p: ExecutionProfile,
) -> ApiResult<()> {
    operator(ctx, actor, app).await?;
    p.validate().map_err(ApiFailure::invalid)?;
    match p.model.as_ref() {
        Some(ModelBinding::Legacy(_)) => {
            return Err(ApiFailure::invalid(
                "New profiles must use a registered model reference",
            ))
        }
        Some(ModelBinding::Registered(_)) if matches!(p.driver, Driver::Direct | Driver::Fake) => {
            return Err(ApiFailure::invalid(
                "Direct and fake profiles must be model-free",
            ))
        }
        binding => {
            let required = if p.driver == Driver::Minitap {
                vec![ModelCapability::MinitapNavigation]
            } else {
                vec![]
            };
            super::model_registry::resolve_for_new_work(&ctx.db, binding, &required).await?;
        }
    }
    let value = json(&p)?;
    let r = rows(
        &ctx.db,
        "SELECT payload,app_id FROM execution_profiles WHERE id=$1",
        vec![p.id.into()],
    )
    .await?;
    if let Some(r) = r.first() {
        if field::<Uuid>(r, "app_id")? != app || field::<serde_json::Value>(r, "payload")? != value
        {
            return Err(conflict(
                "Profile revisions are immutable; use a new profile ID",
            ));
        }
        return Ok(());
    }
    exec(
        &ctx.db,
        "INSERT INTO execution_profiles(id,app_id,payload) VALUES($1,$2,$3)",
        vec![p.id.into(), app.into(), value.into()],
    )
    .await?;
    Ok(())
}

pub async fn resolve(
    db: &impl ConnectionTrait,
    app: Uuid,
    p: &PlanDefinition,
) -> ApiResult<Vec<ResolvedCase>> {
    let mut selections = p.cases.clone();
    for id in &p.suite_version_ids {
        let d = get(db, app, *id).await?;
        super::test_library::admitted(db, app, *id).await?;
        let TestDefinition::Suite(s) = d.definition else {
            return Err(ApiFailure::invalid("Suite reference expected"));
        };
        selections.extend(s.cases);
    }
    let mut result: Vec<ResolvedCase> = vec![];
    for sel in selections {
        let d = require_case(db, app, sel.case_version_id, true).await?;
        let TestDefinition::Case(c) = d.definition else {
            unreachable!()
        };
        if let Some(prev) = result.iter().find(|v| v.case.key == c.key) {
            if prev.definition_id != d.id
                || prev.required != sel.required
                || prev.data_variant != sel.data_variant
            {
                return Err(conflict("Conflicting case versions or selection policy"));
            }
            continue;
        }
        result.push(ResolvedCase {
            definition_id: d.id,
            content_hash: d.content_hash,
            data_variant: sel.data_variant,
            required: sel.required,
            case: c,
        });
    }
    if result.is_empty() || result.len() > 100 || !result.iter().any(|c| c.required) {
        return Err(ApiFailure::invalid(
            "Plan needs 1–100 cases and required coverage",
        ));
    }
    Ok(result)
}
