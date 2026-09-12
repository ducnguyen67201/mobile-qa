//! Workspace creation checks current account approval and commits ownership atomically.
use crate::{
    errors::{ApiFailure, ApiResult},
    models::_entities::{memberships, organizations, users},
    services::{apps, auth},
};
use chrono::Utc;
use loco_rs::app::AppContext;
use mobile_qa_contracts::browser::{
    CreateWorkspaceRequest, MembershipRole, OrganizationMembership,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set, TransactionTrait,
};
use uuid::Uuid;

pub async fn create(
    ctx: &AppContext,
    user_id: Uuid,
    input: CreateWorkspaceRequest,
) -> ApiResult<OrganizationMembership> {
    let name = apps::text(&input.name, 100, "Workspace name")?;
    auth::rate_limit(ctx, "workspace-create", &user_id.to_string(), 20).await?;
    let tx = ctx.db.begin().await?;
    let user = users::Entity::find_by_id(user_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::unauthorized)?;
    if user.disabled_at.is_some() {
        return Err(ApiFailure::unauthorized());
    }
    if user.approval_status != "approved" {
        return Err(ApiFailure::new(
            403,
            "approval_required",
            "Your account must be approved before you can create a workspace",
        ));
    }
    if let Some(existing) = organizations::Entity::find_by_id(input.id).one(&tx).await? {
        let owned = memberships::Entity::find()
            .filter(memberships::Column::UserId.eq(user_id))
            .filter(memberships::Column::OrganizationId.eq(input.id))
            .filter(memberships::Column::Active.eq(true))
            .filter(memberships::Column::Role.eq("operator"))
            .one(&tx)
            .await?;
        if owned.is_none() || existing.name != name {
            return Err(ApiFailure::new(
                409,
                "workspace_conflict",
                "Workspace request conflicts with an existing record",
            ));
        }
    } else {
        organizations::ActiveModel {
            id: Set(input.id),
            name: Set(name.clone()),
            created_at: Set(Utc::now()),
        }
        .insert(&tx)
        .await?;
        memberships::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            organization_id: Set(input.id),
            role: Set("operator".into()),
            active: Set(true),
        }
        .insert(&tx)
        .await?;
    }
    tx.commit().await?;
    Ok(OrganizationMembership {
        organization_id: input.id,
        name,
        role: MembershipRole::Operator,
    })
}
