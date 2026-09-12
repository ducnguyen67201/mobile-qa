//! SeaORM migration registry. No product schema has been added yet.
//! The framework creates migration bookkeeping even with an empty registry.
//! Add versioned migrations here as features arrive; preserve the generator marker.

pub use sea_orm_migration::prelude::*;
pub struct Migrator;
#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            // inject-above (do not remove this comment)
        ]
    }
}
