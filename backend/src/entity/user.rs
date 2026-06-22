use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub username: String,
    pub password_hash: String,
    pub is_admin: bool,
    pub is_deleted: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::user_group_membership::Entity")]
    UserGroupMembership,
    #[sea_orm(has_many = "super::document_user_assignment::Entity")]
    DocumentAssignment,
    #[sea_orm(has_many = "super::comparison::Entity")]
    Comparison,
}

impl Related<super::user_group_membership::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserGroupMembership.def()
    }
}

impl Related<super::document_user_assignment::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DocumentAssignment.def()
    }
}

impl Related<super::comparison::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Comparison.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
