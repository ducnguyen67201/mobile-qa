//! SeaORM migration registry for the versioned app-setup schema.
//! SeaORM records applied migrations; preserve the generator marker when adding more.

pub use sea_orm_migration::prelude::*;
mod m20260912_000001_app_setup;
mod m20260912_000002_google_sign_in;
pub struct Migrator;
#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260912_000001_app_setup::Migration),
            Box::new(m20260912_000002_google_sign_in::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
