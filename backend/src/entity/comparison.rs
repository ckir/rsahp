use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "comparison")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub document_id: i32,
    pub respondent_id: i32, // Identifies the user filling the evaluation
    pub parent_node_id: i32,
    pub node_a_id: i32,
    pub node_b_id: i32,
    pub saaty_value: f64, // The 1-9 value (or reciprocal 1/9 to 1)
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::document::Entity",
        from = "Column::DocumentId",
        to = "super::document::Column::Id"
    )]
    Document,
    #[sea_orm(
        belongs_to = "super::node::Entity",
        from = "Column::ParentNodeId",
        to = "super::node::Column::Id"
    )]
    ParentNode,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::RespondentId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl Related<super::document::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Document.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
