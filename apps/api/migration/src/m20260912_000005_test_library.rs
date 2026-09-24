//! Editable workspaces sit beside immutable execution versions. Backfill only links
//! historical content; it must never recompute hashes or rewrite saved manifests.
use md5::{Digest, Md5};
use sea_orm::{
    entity::prelude::{Json, Uuid},
    ActiveModelTrait, EntityTrait, Set,
};
use sea_orm_migration::prelude::*;
use std::collections::{HashMap, HashSet};

mod legacy_definition {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "execution_definitions")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub app_id: Uuid,
        pub kind: String,
        pub logical_key: String,
        pub version: i32,
        pub content_hash: String,
        pub payload: Json,
        pub author_id: Uuid,
        pub created_at: DateTimeUtc,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

mod legacy_approval {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "execution_approvals")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub definition_id: Uuid,
        #[sea_orm(primary_key, auto_increment = false)]
        pub purpose: String,
        pub actor_id: Uuid,
        pub content_hash: String,
        pub approved_at: DateTimeUtc,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

mod legacy_profile {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "execution_profiles")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub app_id: Uuid,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

mod entry {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "test_library_entries")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub app_id: Uuid,
        pub kind: String,
        pub logical_key: String,
        pub next_version: i32,
        pub revision: i32,
        pub archived_at: Option<DateTimeUtc>,
        pub actor_id: Uuid,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

mod version {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "test_library_versions")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub definition_id: Uuid,
        pub entry_id: Uuid,
        pub review_state: String,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

mod review_event {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "test_library_review_events")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub entry_id: Uuid,
        pub definition_id: Uuid,
        pub actor_id: Uuid,
        pub purpose: String,
        pub decision: String,
        pub content_hash: String,
        pub reason: Option<String>,
        pub created_at: DateTimeUtc,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

mod library_default {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "test_library_defaults")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub app_id: Uuid,
        pub plan_version_id: Uuid,
        pub revision: i32,
        pub actor_id: Uuid,
        pub updated_at: DateTimeUtc,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

#[derive(DeriveMigrationName)]
pub struct Migration;

fn referenced_ids(payload: &Json, pointer: &str, field: Option<&str>) -> Result<Vec<Uuid>, DbErr> {
    let Some(values) = payload.pointer(pointer).and_then(Json::as_array) else {
        return Ok(Vec::new());
    };
    values
        .iter()
        .map(|value| {
            let value = match field {
                Some(field) => value.get(field),
                None => Some(value),
            };
            value
                .and_then(Json::as_str)
                .and_then(|value| Uuid::parse_str(value).ok())
                .ok_or_else(|| {
                    DbErr::Custom(
                        "Legacy execution references or approval hashes are inconsistent".into(),
                    )
                })
        })
        .collect()
}

fn review_event_id(approval: &legacy_approval::Model) -> Uuid {
    // Preserve PostgreSQL `md5(definition_id::text || purpose)::uuid` byte-for-byte.
    let digest = Md5::digest(format!("{}{}", approval.definition_id, approval.purpose).as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest);
    Uuid::from_bytes(bytes)
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_schema(manager).await?;
        backfill(manager).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            "test_library_mutations",
            "test_library_defaults",
            "test_library_review_events",
            "test_library_versions",
            "test_library_drafts",
            "test_library_entries",
        ] {
            manager
                .drop_table(Table::drop().table(Alias::new(table)).to_owned())
                .await?;
        }
        Ok(())
    }
}

async fn create_schema(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Alias::new("test_library_entries"))
                .col(
                    ColumnDef::new(Alias::new("id"))
                        .uuid()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(Alias::new("app_id")).uuid().not_null())
                .col(
                    ColumnDef::new(Alias::new("kind"))
                        .text()
                        .not_null()
                        .check(Expr::col(Alias::new("kind")).is_in(["case", "suite", "plan"])),
                )
                .col(ColumnDef::new(Alias::new("logical_key")).text().not_null())
                .col(
                    ColumnDef::new(Alias::new("next_version"))
                        .integer()
                        .not_null()
                        .check(Expr::col(Alias::new("next_version")).gt(0)),
                )
                .col(
                    ColumnDef::new(Alias::new("revision"))
                        .integer()
                        .not_null()
                        .default(1)
                        .check(Expr::col(Alias::new("revision")).gt(0)),
                )
                .col(ColumnDef::new(Alias::new("archived_at")).timestamp_with_time_zone())
                .col(ColumnDef::new(Alias::new("actor_id")).uuid().not_null())
                .col(
                    ColumnDef::new(Alias::new("created_at"))
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(Alias::new("updated_at"))
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .index(
                    Index::create()
                        .unique()
                        .col(Alias::new("app_id"))
                        .col(Alias::new("kind"))
                        .col(Alias::new("logical_key")),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(Alias::new("test_library_entries"), Alias::new("app_id"))
                        .to(Alias::new("apps"), Alias::new("id")),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(Alias::new("test_library_entries"), Alias::new("actor_id"))
                        .to(Alias::new("users"), Alias::new("id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_table(
            Table::create()
                .table(Alias::new("test_library_drafts"))
                .col(
                    ColumnDef::new(Alias::new("entry_id"))
                        .uuid()
                        .not_null()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(Alias::new("version"))
                        .integer()
                        .not_null()
                        .check(Expr::col(Alias::new("version")).gt(0)),
                )
                .col(ColumnDef::new(Alias::new("source_version_id")).uuid())
                .col(
                    ColumnDef::new(Alias::new("payload"))
                        .json_binary()
                        .not_null(),
                )
                .col(ColumnDef::new(Alias::new("editor_id")).uuid().not_null())
                .col(
                    ColumnDef::new(Alias::new("updated_at"))
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(Alias::new("test_library_drafts"), Alias::new("entry_id"))
                        .to(Alias::new("test_library_entries"), Alias::new("id")),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(
                            Alias::new("test_library_drafts"),
                            Alias::new("source_version_id"),
                        )
                        .to(Alias::new("execution_definitions"), Alias::new("id")),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(Alias::new("test_library_drafts"), Alias::new("editor_id"))
                        .to(Alias::new("users"), Alias::new("id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_table(
            Table::create()
                .table(Alias::new("test_library_versions"))
                .col(
                    ColumnDef::new(Alias::new("definition_id"))
                        .uuid()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(Alias::new("entry_id")).uuid().not_null())
                .col(
                    ColumnDef::new(Alias::new("review_state"))
                        .text()
                        .not_null()
                        .check(Expr::col(Alias::new("review_state")).is_in([
                            "in_review",
                            "needs_input",
                            "rejected",
                            "approved",
                        ])),
                )
                .col(
                    ColumnDef::new(Alias::new("created_at"))
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(Alias::new("updated_at"))
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(
                            Alias::new("test_library_versions"),
                            Alias::new("definition_id"),
                        )
                        .to(Alias::new("execution_definitions"), Alias::new("id")),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(Alias::new("test_library_versions"), Alias::new("entry_id"))
                        .to(Alias::new("test_library_entries"), Alias::new("id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_table(
            Table::create()
                .table(Alias::new("test_library_review_events"))
                .col(
                    ColumnDef::new(Alias::new("id"))
                        .uuid()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(Alias::new("entry_id")).uuid().not_null())
                .col(
                    ColumnDef::new(Alias::new("definition_id"))
                        .uuid()
                        .not_null(),
                )
                .col(ColumnDef::new(Alias::new("actor_id")).uuid().not_null())
                .col(
                    ColumnDef::new(Alias::new("purpose"))
                        .text()
                        .not_null()
                        .check(
                            Expr::col(Alias::new("purpose")).is_in(["business", "executability"]),
                        ),
                )
                .col(
                    ColumnDef::new(Alias::new("decision"))
                        .text()
                        .not_null()
                        .check(Expr::col(Alias::new("decision")).is_in([
                            "approve",
                            "needs_input",
                            "reject",
                        ])),
                )
                .col(ColumnDef::new(Alias::new("content_hash")).text().not_null())
                .col(ColumnDef::new(Alias::new("reason")).text())
                .col(
                    ColumnDef::new(Alias::new("created_at"))
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(
                            Alias::new("test_library_review_events"),
                            Alias::new("entry_id"),
                        )
                        .to(Alias::new("test_library_entries"), Alias::new("id")),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(
                            Alias::new("test_library_review_events"),
                            Alias::new("definition_id"),
                        )
                        .to(Alias::new("execution_definitions"), Alias::new("id")),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(
                            Alias::new("test_library_review_events"),
                            Alias::new("actor_id"),
                        )
                        .to(Alias::new("users"), Alias::new("id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_table(
            Table::create()
                .table(Alias::new("test_library_defaults"))
                .col(
                    ColumnDef::new(Alias::new("app_id"))
                        .uuid()
                        .not_null()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(Alias::new("plan_version_id"))
                        .uuid()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(Alias::new("revision"))
                        .integer()
                        .not_null()
                        .check(Expr::col(Alias::new("revision")).gt(0)),
                )
                .col(ColumnDef::new(Alias::new("actor_id")).uuid().not_null())
                .col(
                    ColumnDef::new(Alias::new("updated_at"))
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(Alias::new("test_library_defaults"), Alias::new("app_id"))
                        .to(Alias::new("apps"), Alias::new("id")),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(
                            Alias::new("test_library_defaults"),
                            Alias::new("plan_version_id"),
                        )
                        .to(Alias::new("execution_definitions"), Alias::new("id")),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(Alias::new("test_library_defaults"), Alias::new("actor_id"))
                        .to(Alias::new("users"), Alias::new("id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_table(
            Table::create()
                .table(Alias::new("test_library_mutations"))
                .col(ColumnDef::new(Alias::new("app_id")).uuid().not_null())
                .col(ColumnDef::new(Alias::new("actor_id")).uuid().not_null())
                .col(ColumnDef::new(Alias::new("mutation_id")).uuid().not_null())
                .col(ColumnDef::new(Alias::new("fingerprint")).text().not_null())
                .col(
                    ColumnDef::new(Alias::new("response"))
                        .json_binary()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(Alias::new("created_at"))
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .primary_key(
                    Index::create()
                        .col(Alias::new("app_id"))
                        .col(Alias::new("actor_id"))
                        .col(Alias::new("mutation_id")),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(Alias::new("test_library_mutations"), Alias::new("app_id"))
                        .to(Alias::new("apps"), Alias::new("id")),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(Alias::new("test_library_mutations"), Alias::new("actor_id"))
                        .to(Alias::new("users"), Alias::new("id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("test_library_catalog")
                .table(Alias::new("test_library_entries"))
                .col(Alias::new("app_id"))
                .col(Alias::new("kind"))
                .col(Alias::new("archived_at"))
                .col(Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("test_library_history")
                .table(Alias::new("test_library_versions"))
                .col(Alias::new("entry_id"))
                .col(Alias::new("definition_id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("test_library_audit")
                .table(Alias::new("test_library_review_events"))
                .col(Alias::new("definition_id"))
                .col(Alias::new("created_at"))
                .col(Alias::new("id"))
                .to_owned(),
        )
        .await
}

async fn backfill(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let db = manager.get_connection();
    let definitions = legacy_definition::Entity::find().all(db).await?;
    let approvals = legacy_approval::Entity::find().all(db).await?;
    let profiles = legacy_profile::Entity::find().all(db).await?;
    let definitions_by_id: HashMap<_, _> = definitions.iter().map(|row| (row.id, row)).collect();
    let profile_keys: HashSet<_> = profiles
        .into_iter()
        .map(|row| (row.id, row.app_id))
        .collect();

    for approval in &approvals {
        if definitions_by_id
            .get(&approval.definition_id)
            .is_none_or(|definition| approval.content_hash != definition.content_hash)
        {
            return Err(DbErr::Custom(
                "Legacy execution references or approval hashes are inconsistent".into(),
            ));
        }
    }
    for definition in &definitions {
        if matches!(definition.kind.as_str(), "suite" | "plan") {
            for target_id in referenced_ids(
                &definition.payload,
                "/content/cases",
                Some("case_version_id"),
            )? {
                if definitions_by_id.get(&target_id).is_none_or(|target| {
                    target.app_id != definition.app_id || target.kind != "case"
                }) {
                    return Err(DbErr::Custom(
                        "Legacy execution references or approval hashes are inconsistent".into(),
                    ));
                }
            }
        }
        if definition.kind == "plan" {
            for target_id in
                referenced_ids(&definition.payload, "/content/suite_version_ids", None)?
            {
                if definitions_by_id.get(&target_id).is_none_or(|target| {
                    target.app_id != definition.app_id || target.kind != "suite"
                }) {
                    return Err(DbErr::Custom(
                        "Legacy execution references or approval hashes are inconsistent".into(),
                    ));
                }
            }
            let profile_id = definition
                .payload
                .pointer("/content/profile_id")
                .and_then(Json::as_str)
                .and_then(|value| Uuid::parse_str(value).ok())
                .ok_or_else(|| {
                    DbErr::Custom(
                        "Legacy execution references or approval hashes are inconsistent".into(),
                    )
                })?;
            if !profile_keys.contains(&(profile_id, definition.app_id)) {
                return Err(DbErr::Custom(
                    "Legacy execution references or approval hashes are inconsistent".into(),
                ));
            }
        }
    }

    let mut grouped: HashMap<(Uuid, String, String), Vec<&legacy_definition::Model>> =
        HashMap::new();
    for definition in &definitions {
        grouped
            .entry((
                definition.app_id,
                definition.kind.clone(),
                definition.logical_key.clone(),
            ))
            .or_default()
            .push(definition);
    }
    let mut entry_by_definition = HashMap::new();
    for ((app_id, kind, logical_key), mut group) in grouped {
        group.sort_by_key(|row| (row.version, row.id));
        let first = group[0];
        let updated_at = group
            .iter()
            .map(|row| row.created_at)
            .max()
            .unwrap_or(first.created_at);
        let next_version = group.iter().map(|row| row.version).max().unwrap_or(0) + 1;
        entry::ActiveModel {
            id: Set(first.id),
            app_id: Set(app_id),
            kind: Set(kind),
            logical_key: Set(logical_key),
            next_version: Set(next_version),
            revision: Set(1),
            archived_at: Set(None),
            actor_id: Set(first.author_id),
            created_at: Set(first.created_at),
            updated_at: Set(updated_at),
        }
        .insert(db)
        .await?;
        for definition in group {
            entry_by_definition.insert(definition.id, first.id);
        }
    }

    let approval_counts: HashMap<Uuid, usize> =
        approvals
            .iter()
            .fold(HashMap::new(), |mut counts, approval| {
                *counts.entry(approval.definition_id).or_default() += 1;
                counts
            });
    let mut approved = HashSet::new();
    for definition in &definitions {
        let is_approved = approval_counts.get(&definition.id).copied().unwrap_or(0) == 2;
        if is_approved {
            approved.insert(definition.id);
        }
        version::ActiveModel {
            definition_id: Set(definition.id),
            entry_id: Set(entry_by_definition[&definition.id]),
            review_state: Set(if is_approved { "approved" } else { "in_review" }.into()),
            created_at: Set(definition.created_at),
            updated_at: Set(definition.created_at),
        }
        .insert(db)
        .await?;
    }
    for approval in &approvals {
        review_event::ActiveModel {
            id: Set(review_event_id(approval)),
            entry_id: Set(entry_by_definition[&approval.definition_id]),
            definition_id: Set(approval.definition_id),
            actor_id: Set(approval.actor_id),
            purpose: Set(approval.purpose.clone()),
            decision: Set("approve".into()),
            content_hash: Set(approval.content_hash.clone()),
            reason: Set(None),
            created_at: Set(approval.approved_at),
        }
        .insert(db)
        .await?;
    }

    let mut plans: HashMap<Uuid, &legacy_definition::Model> = HashMap::new();
    for definition in definitions
        .iter()
        .filter(|row| row.kind == "plan" && approved.contains(&row.id))
    {
        if plans.get(&definition.app_id).is_none_or(|current| {
            (definition.created_at, definition.id) > (current.created_at, current.id)
        }) {
            plans.insert(definition.app_id, definition);
        }
    }
    for (app_id, definition) in plans {
        library_default::ActiveModel {
            app_id: Set(app_id),
            plan_version_id: Set(definition.id),
            revision: Set(1),
            actor_id: Set(definition.author_id),
            updated_at: Set(definition.created_at),
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_event_ids_preserve_the_legacy_postgres_md5_value() {
        let approval = legacy_approval::Model {
            definition_id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
            purpose: "business".into(),
            actor_id: Uuid::nil(),
            content_hash: "unchanged".into(),
            approved_at: "2026-01-01T00:00:00Z".parse().unwrap(),
        };
        assert_eq!(
            review_event_id(&approval).to_string(),
            "e0a28393-b929-f230-9dcb-8ecf0e320d23"
        );
    }
}
