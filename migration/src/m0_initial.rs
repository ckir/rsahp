//! m0 — immutable initial baseline. Reproduces the 9-table schema that
//! `create_table_from_entity` produced. DO NOT EDIT after it ships; add new
//! migrations for changes.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    // The 9-table baseline is inherently long; splitting it up would not make
    // the migration clearer, just harder to review against the original
    // `create_table_from_entity` output it reproduces.
    #[allow(clippy::too_many_lines)]
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // user_group
        manager
            .create_table(
                Table::create()
                    .table(UserGroup::Table)
                    .col(
                        ColumnDef::new(UserGroup::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(UserGroup::Name).string().not_null())
                    .col(ColumnDef::new(UserGroup::ParentId).integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(UserGroup::Table, UserGroup::ParentId)
                            .to(UserGroup::Table, UserGroup::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // user
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .col(
                        ColumnDef::new(User::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(User::Username).string().not_null())
                    .col(ColumnDef::new(User::PasswordHash).string().not_null())
                    .col(ColumnDef::new(User::IsAdmin).boolean().not_null())
                    .col(ColumnDef::new(User::IsDeleted).boolean().not_null())
                    .to_owned(),
            )
            .await?;

        // folder
        manager
            .create_table(
                Table::create()
                    .table(Folder::Table)
                    .col(
                        ColumnDef::new(Folder::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Folder::Name).string().not_null())
                    .col(ColumnDef::new(Folder::OwnerId).integer().not_null())
                    .col(ColumnDef::new(Folder::ParentFolderId).integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Folder::Table, Folder::OwnerId)
                            .to(User::Table, User::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Folder::Table, Folder::ParentFolderId)
                            .to(Folder::Table, Folder::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // document
        manager
            .create_table(
                Table::create()
                    .table(Document::Table)
                    .col(
                        ColumnDef::new(Document::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Document::Name).string().not_null())
                    .col(ColumnDef::new(Document::OwnerId).integer().not_null())
                    .col(ColumnDef::new(Document::Version).integer().not_null())
                    .col(
                        ColumnDef::new(Document::AggregationMethod)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Document::FolderId).integer().null())
                    .col(
                        ColumnDef::new(Document::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Document::Table, Document::OwnerId)
                            .to(User::Table, User::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Document::Table, Document::FolderId)
                            .to(Folder::Table, Folder::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // node
        manager
            .create_table(
                Table::create()
                    .table(Node::Table)
                    .col(
                        ColumnDef::new(Node::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Node::DocumentId).integer().not_null())
                    .col(ColumnDef::new(Node::ParentNodeId).integer().null())
                    .col(ColumnDef::new(Node::Name).string().not_null())
                    .col(ColumnDef::new(Node::NodeType).string().not_null())
                    .col(ColumnDef::new(Node::Cost).double().null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Node::Table, Node::DocumentId)
                            .to(Document::Table, Document::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Node::Table, Node::ParentNodeId)
                            .to(Node::Table, Node::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // comparison (note: node_a_id / node_b_id have NO FK in the entity Relation enum)
        manager
            .create_table(
                Table::create()
                    .table(Comparison::Table)
                    .col(
                        ColumnDef::new(Comparison::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Comparison::DocumentId).integer().not_null())
                    .col(
                        ColumnDef::new(Comparison::RespondentId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Comparison::ParentNodeId)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Comparison::NodeAId).integer().not_null())
                    .col(ColumnDef::new(Comparison::NodeBId).integer().not_null())
                    .col(ColumnDef::new(Comparison::SaatyValue).double().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Comparison::Table, Comparison::DocumentId)
                            .to(Document::Table, Document::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Comparison::Table, Comparison::ParentNodeId)
                            .to(Node::Table, Node::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Comparison::Table, Comparison::RespondentId)
                            .to(User::Table, User::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // user_group_membership
        manager
            .create_table(
                Table::create()
                    .table(UserGroupMembership::Table)
                    .col(
                        ColumnDef::new(UserGroupMembership::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(UserGroupMembership::UserId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserGroupMembership::GroupId)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(UserGroupMembership::Table, UserGroupMembership::UserId)
                            .to(User::Table, User::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(UserGroupMembership::Table, UserGroupMembership::GroupId)
                            .to(UserGroup::Table, UserGroup::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // document_user_assignment
        manager
            .create_table(
                Table::create()
                    .table(DocumentUserAssignment::Table)
                    .col(
                        ColumnDef::new(DocumentUserAssignment::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(DocumentUserAssignment::DocumentId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DocumentUserAssignment::UserId)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                DocumentUserAssignment::Table,
                                DocumentUserAssignment::DocumentId,
                            )
                            .to(Document::Table, Document::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                DocumentUserAssignment::Table,
                                DocumentUserAssignment::UserId,
                            )
                            .to(User::Table, User::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // document_group_assignment
        manager
            .create_table(
                Table::create()
                    .table(DocumentGroupAssignment::Table)
                    .col(
                        ColumnDef::new(DocumentGroupAssignment::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(DocumentGroupAssignment::DocumentId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DocumentGroupAssignment::GroupId)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                DocumentGroupAssignment::Table,
                                DocumentGroupAssignment::DocumentId,
                            )
                            .to(Document::Table, Document::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                DocumentGroupAssignment::Table,
                                DocumentGroupAssignment::GroupId,
                            )
                            .to(UserGroup::Table, UserGroup::Id),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop in reverse dependency order.
        for table in [
            DocumentGroupAssignment::Table.into_iden(),
            DocumentUserAssignment::Table.into_iden(),
            UserGroupMembership::Table.into_iden(),
            Comparison::Table.into_iden(),
            Node::Table.into_iden(),
            Document::Table.into_iden(),
            Folder::Table.into_iden(),
            User::Table.into_iden(),
            UserGroup::Table.into_iden(),
        ] {
            manager
                .drop_table(Table::drop().table(table).to_owned())
                .await?;
        }
        Ok(())
    }
}

// --- Iden definitions: table + column identifiers, names matching the entity `table_name`/field snake_case. ---

#[derive(DeriveIden)]
enum UserGroup {
    Table,
    Id,
    Name,
    ParentId,
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
    Username,
    PasswordHash,
    IsAdmin,
    IsDeleted,
}

#[derive(DeriveIden)]
enum Folder {
    Table,
    Id,
    Name,
    OwnerId,
    ParentFolderId,
}

#[derive(DeriveIden)]
enum Document {
    Table,
    Id,
    Name,
    OwnerId,
    Version,
    AggregationMethod,
    FolderId,
    CreatedAt,
}

#[derive(DeriveIden)]
#[allow(clippy::enum_variant_names)] // `NodeType` is the schema column name; not to be renamed.
enum Node {
    Table,
    Id,
    DocumentId,
    ParentNodeId,
    Name,
    NodeType,
    Cost,
}

#[derive(DeriveIden)]
enum Comparison {
    Table,
    Id,
    DocumentId,
    RespondentId,
    ParentNodeId,
    NodeAId,
    NodeBId,
    SaatyValue,
}

#[derive(DeriveIden)]
enum UserGroupMembership {
    Table,
    Id,
    UserId,
    GroupId,
}

#[derive(DeriveIden)]
enum DocumentUserAssignment {
    Table,
    Id,
    DocumentId,
    UserId,
}

#[derive(DeriveIden)]
enum DocumentGroupAssignment {
    Table,
    Id,
    DocumentId,
    GroupId,
}
