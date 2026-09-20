//! SeaORM migration registry for the versioned app-setup schema.
//! SeaORM records applied migrations; preserve the generator marker when adding more.

pub use sea_orm_migration::prelude::*;
mod m20260912_000001_app_setup;
mod m20260912_000002_google_sign_in;
mod m20260912_000003_workspaces;
mod m20260912_000004_execution;
pub mod m20260912_000005_test_library;
mod m20260913_000006_task_sessions;
mod m20260915_000007_execution_lifecycle;
pub mod m20260920_000008_save_without_reviews;
mod m20260920_000009_saved_case_runs;
pub struct Migrator;
#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260912_000001_app_setup::Migration),
            Box::new(m20260912_000002_google_sign_in::Migration),
            Box::new(m20260912_000003_workspaces::Migration),
            Box::new(m20260912_000004_execution::Migration),
            Box::new(m20260912_000005_test_library::Migration),
            Box::new(m20260913_000006_task_sessions::Migration),
            Box::new(m20260915_000007_execution_lifecycle::Migration),
            Box::new(m20260920_000008_save_without_reviews::Migration),
            Box::new(m20260920_000009_saved_case_runs::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
