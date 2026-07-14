// SPDX-License-Identifier: LicenseRef-PolyForm-Noncommercial-1.0.0
//! User group membership entity representation.
//!
//! Defines the database model and relations for linking users to groups.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a user's membership in a group.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "user_group_membership")]
pub struct Model {
    /// The unique identifier of the membership
    #[sea_orm(primary_key)]
    pub id: i32,
    /// The user who belongs to the group
    pub user_id: i32,
    /// The group the user belongs to
    pub group_id: i32,
}

/// Defines the relationships for the UserGroupMembership entity.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Relationship to the associated User entity
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
    /// Relationship to the associated UserGroup entity
    #[sea_orm(
        belongs_to = "super::user_group::Entity",
        from = "Column::GroupId",
        to = "super::user_group::Column::Id"
    )]
    UserGroup,
}

impl Related<super::user::Entity> for Entity {
    /// Configures the relationship mapping to a User
    fn to() -> RelationDef {
        // Return the definition for User relationship
        Relation::User.def()
    }
}

impl Related<super::user_group::Entity> for Entity {
    /// Configures the relationship mapping to a UserGroup
    fn to() -> RelationDef {
        // Return the definition for UserGroup relationship
        Relation::UserGroup.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
