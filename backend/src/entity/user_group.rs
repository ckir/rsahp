use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user_group")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub parent_id: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user_group::Entity",
        from = "Column::ParentId",
        to = "super::user_group::Column::Id"
    )]
    ParentGroup,
    #[sea_orm(has_many = "super::user_group_membership::Entity")]
    UserGroupMembership,
    #[sea_orm(has_many = "super::document_group_assignment::Entity")]
    DocumentAssignment,
}

impl Related<super::user_group_membership::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserGroupMembership.def()
    }
}

impl Related<super::document_group_assignment::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DocumentAssignment.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
