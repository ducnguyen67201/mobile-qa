//! Review decisions remain historical audit facts; saved versions no longer need them.
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use sea_orm_migration::prelude::*;

mod legacy_version {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "test_library_versions")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub definition_id: Uuid,
        pub legacy_review_state: Option<String>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("test_library_versions"))
                    .rename_column(
                        Alias::new("review_state"),
                        Alias::new("legacy_review_state"),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("test_library_versions"))
                    .modify_column(
                        ColumnDef::new(Alias::new("legacy_review_state"))
                            .text()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if legacy_version::Entity::find()
            .filter(legacy_version::Column::LegacyReviewState.is_null())
            .one(manager.get_connection())
            .await?
            .is_some()
        {
            return Err(DbErr::Custom(
                "Saved versions exist; restore a pre-upgrade backup or forward-fix instead".into(),
            ));
        }
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("test_library_versions"))
                    .modify_column(
                        ColumnDef::new(Alias::new("legacy_review_state"))
                            .text()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("test_library_versions"))
                    .rename_column(
                        Alias::new("legacy_review_state"),
                        Alias::new("review_state"),
                    )
                    .to_owned(),
            )
            .await
    }
}
