//! SeaORM migration registry for the versioned app-setup schema.
//! SeaORM records applied migrations; preserve the generator marker when adding more.

pub use sea_orm_migration::prelude::*;
mod m20260912_000001_app_setup;
mod m20260912_000002_google_sign_in;
mod m20260912_000003_workspaces;
mod m20260912_000004_execution;
pub mod m20260912_000005_test_library;
mod m20260913_000006_task_sessions;
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
            // inject-above (do not remove this comment)
        ]
    }
}
