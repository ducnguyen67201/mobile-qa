//! Editable workspaces sit beside immutable execution versions. Backfill only links
//! historical content; it must never recompute hashes or rewrite saved manifests.
use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(r#"
CREATE TABLE test_library_entries (
 id UUID PRIMARY KEY, app_id UUID NOT NULL REFERENCES apps(id),
 kind TEXT NOT NULL CHECK(kind IN ('case','suite','plan')), logical_key TEXT NOT NULL,
 next_version INTEGER NOT NULL CHECK(next_version>0), revision INTEGER NOT NULL DEFAULT 1 CHECK(revision>0),
 archived_at TIMESTAMPTZ, actor_id UUID NOT NULL REFERENCES users(id),
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(), updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 UNIQUE(app_id,kind,logical_key));
CREATE TABLE test_library_drafts (
 entry_id UUID PRIMARY KEY REFERENCES test_library_entries(id), version INTEGER NOT NULL CHECK(version>0),
 source_version_id UUID REFERENCES execution_definitions(id), payload JSONB NOT NULL,
 editor_id UUID NOT NULL REFERENCES users(id), updated_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE TABLE test_library_versions (
 definition_id UUID PRIMARY KEY REFERENCES execution_definitions(id), entry_id UUID NOT NULL REFERENCES test_library_entries(id),
 review_state TEXT NOT NULL CHECK(review_state IN ('in_review','needs_input','rejected','approved')),
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(), updated_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE TABLE test_library_review_events (
 id UUID PRIMARY KEY, entry_id UUID NOT NULL REFERENCES test_library_entries(id),
 definition_id UUID NOT NULL REFERENCES execution_definitions(id), actor_id UUID NOT NULL REFERENCES users(id),
 purpose TEXT NOT NULL CHECK(purpose IN ('business','executability')),
 decision TEXT NOT NULL CHECK(decision IN ('approve','needs_input','reject')),
 content_hash TEXT NOT NULL, reason TEXT, created_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE TABLE test_library_defaults (
 app_id UUID PRIMARY KEY REFERENCES apps(id), plan_version_id UUID NOT NULL REFERENCES execution_definitions(id),
 revision INTEGER NOT NULL CHECK(revision>0), actor_id UUID NOT NULL REFERENCES users(id),
 updated_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE TABLE test_library_mutations (
 app_id UUID NOT NULL REFERENCES apps(id), actor_id UUID NOT NULL REFERENCES users(id), mutation_id UUID NOT NULL,
 fingerprint TEXT NOT NULL, response JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 PRIMARY KEY(app_id,actor_id,mutation_id));
CREATE INDEX test_library_catalog ON test_library_entries(app_id,kind,archived_at,id);
CREATE INDEX test_library_history ON test_library_versions(entry_id,definition_id);
CREATE INDEX test_library_audit ON test_library_review_events(definition_id,created_at,id);

-- Reject inconsistent historical approval/reference data instead of manufacturing
-- usable catalog entries from broken execution records.
DO $$ BEGIN
 IF EXISTS(SELECT 1 FROM execution_approvals a JOIN execution_definitions d ON d.id=a.definition_id WHERE a.content_hash<>d.content_hash)
 OR EXISTS(SELECT 1 FROM execution_definitions d CROSS JOIN LATERAL jsonb_array_elements(COALESCE(d.payload->'content'->'cases','[]'::jsonb)) c
   WHERE d.kind IN ('suite','plan') AND NOT EXISTS(SELECT 1 FROM execution_definitions target WHERE target.id=(c->>'case_version_id')::uuid AND target.app_id=d.app_id AND target.kind='case'))
 OR EXISTS(SELECT 1 FROM execution_definitions d CROSS JOIN LATERAL jsonb_array_elements_text(COALESCE(d.payload->'content'->'suite_version_ids','[]'::jsonb)) s
   WHERE d.kind='plan' AND NOT EXISTS(SELECT 1 FROM execution_definitions target WHERE target.id=s::uuid AND target.app_id=d.app_id AND target.kind='suite'))
 OR EXISTS(SELECT 1 FROM execution_definitions d WHERE d.kind='plan' AND NOT EXISTS(SELECT 1 FROM execution_profiles p WHERE p.id=(d.payload->'content'->>'profile_id')::uuid AND p.app_id=d.app_id))
 THEN RAISE EXCEPTION 'Legacy execution references or approval hashes are inconsistent'; END IF;
END $$;

-- Reuse the oldest definition ID as the catalog ID: deterministic and no extension required.
INSERT INTO test_library_entries(id,app_id,kind,logical_key,next_version,actor_id,created_at,updated_at)
SELECT first.id, first.app_id, first.kind, first.logical_key, totals.maximum+1, first.author_id, first.created_at, totals.updated_at
FROM (SELECT DISTINCT ON (app_id,kind,logical_key) * FROM execution_definitions ORDER BY app_id,kind,logical_key,version,id) first
JOIN (SELECT app_id,kind,logical_key,max(version) maximum,max(created_at) updated_at FROM execution_definitions GROUP BY app_id,kind,logical_key) totals
USING(app_id,kind,logical_key);
INSERT INTO test_library_versions(definition_id,entry_id,review_state,created_at)
SELECT d.id,e.id,CASE WHEN (SELECT count(*) FROM execution_approvals a WHERE a.definition_id=d.id AND a.content_hash=d.content_hash)=2 THEN 'approved' ELSE 'in_review' END,d.created_at
FROM execution_definitions d JOIN test_library_entries e USING(app_id,kind,logical_key);
INSERT INTO test_library_review_events(id,entry_id,definition_id,actor_id,purpose,decision,content_hash,created_at)
SELECT md5(a.definition_id::text||a.purpose)::uuid,v.entry_id,a.definition_id,a.actor_id,a.purpose,'approve',a.content_hash,a.approved_at
FROM execution_approvals a JOIN test_library_versions v ON v.definition_id=a.definition_id;
-- One-time compatibility choice. Later approvals cannot move the user's default.
INSERT INTO test_library_defaults(app_id,plan_version_id,revision,actor_id)
SELECT DISTINCT ON (d.app_id) d.app_id,d.id,1,d.author_id FROM execution_definitions d
JOIN test_library_versions v ON v.definition_id=d.id WHERE d.kind='plan' AND v.review_state='approved'
ORDER BY d.app_id,d.created_at DESC,d.id DESC;
"#).await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared("DROP TABLE test_library_mutations, test_library_defaults, test_library_review_events, test_library_versions, test_library_drafts, test_library_entries;").await?;
        Ok(())
    }
}
