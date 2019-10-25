#![allow(non_snake_case)]
#![allow(unused)]
mod test_harness;
use pretty_assertions::{assert_eq, assert_ne};
use sql_migration_connector::{AlterIndex, CreateIndex, DropIndex, SqlFamily, SqlMigrationStep};
use sql_schema_describer::*;
use test_harness::*;

#[test]
fn adding_a_scalar_field_must_work() {
    test_each_connector(|test_setup, api| {
        let dm2 = r#"
            model Test {
                id String @id @default(cuid())
                int Int
                float Float
                boolean Boolean
                string String
                dateTime DateTime
                enum MyEnum
            }

            enum MyEnum {
                A
                B
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm2).sql_schema;
        let table = result.table_bang("Test");
        table.columns.iter().for_each(|c| assert_eq!(c.is_required(), true));

        assert_eq!(table.column_bang("int").tpe.family, ColumnTypeFamily::Int);
        assert_eq!(table.column_bang("float").tpe.family, ColumnTypeFamily::Float);
        assert_eq!(table.column_bang("boolean").tpe.family, ColumnTypeFamily::Boolean);
        assert_eq!(table.column_bang("string").tpe.family, ColumnTypeFamily::String);
        assert_eq!(table.column_bang("dateTime").tpe.family, ColumnTypeFamily::DateTime);
        assert_eq!(table.column_bang("enum").tpe.family, ColumnTypeFamily::String);
    });
}

//#[test]
//fn apply_schema() {
//    test_each_connector(|api| {
//        let dm2 = r#"
//            model Test {
//                id String @id @default(cuid())
//                int Int
//                float Float
//                boolean Boolean
//                string String
//                dateTime DateTime
//                enum MyEnum
//            }
//
//            enum MyEnum {
//                A
//                B
//            }
//        "#;
//
//        infer_and_apply(test_setup, api, &dm2);
//    });
//}

#[test]
fn adding_an_optional_field_must_work() {
    test_each_connector(|test_setup, api| {
        let dm2 = r#"
            model Test {
                id String @id @default(cuid())
                field String?
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm2).sql_schema;
        let column = result.table_bang("Test").column_bang("field");
        assert_eq!(column.is_required(), false);
    });
}

#[test]
fn adding_an_id_field_with_a_special_name_must_work() {
    test_each_connector(|test_setup, api| {
        let dm2 = r#"
            model Test {
                specialName String @id @default(cuid())
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm2).sql_schema;
        let column = result.table_bang("Test").column("specialName");
        assert_eq!(column.is_some(), true);
    });
}

#[test]
fn adding_an_id_field_of_type_int_must_work() {
    test_each_connector(|test_setup, api| {
        let dm2 = r#"
            model Test {
                myId Int @id
            }
        "#;

        let result = infer_and_apply(test_setup, api, &dm2).sql_schema;
        let column = result.table_bang("Test").column_bang("myId");
        match test_setup.sql_family {
            SqlFamily::Postgres => {
                let sequence = result.get_sequence("Test_myId_seq").expect("sequence must exist");
                let default = column.default.as_ref().expect("Must have nextval default");
                assert_eq!(default.contains(&sequence.name), true);
                assert_eq!(default, &format!("nextval('\"{}\"'::regclass)", sequence.name))
            }
            _ => assert_eq!(column.auto_increment, true),
        }
    });
}

#[test]
fn removing_a_scalar_field_must_work() {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model Test {
                id String @id @default(cuid())
                field String
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let column1 = result.table_bang("Test").column("field");
        assert_eq!(column1.is_some(), true);

        let dm2 = r#"
            model Test {
                id String @id @default(cuid())
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm2).sql_schema;
        let column2 = result.table_bang("Test").column("field");
        assert_eq!(column2.is_some(), false);
    });
}

#[test]
fn can_handle_reserved_sql_keywords_for_model_name() {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model Group {
                id String @id @default(cuid())
                field String
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let column = result.table_bang("Group").column_bang("field");
        assert_eq!(column.tpe.family, ColumnTypeFamily::String);

        let dm2 = r#"
            model Group {
                id String @id @default(cuid())
                field Int
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm2).sql_schema;
        let column = result.table_bang("Group").column_bang("field");
        assert_eq!(column.tpe.family, ColumnTypeFamily::Int);
    });
}

#[test]
fn can_handle_reserved_sql_keywords_for_field_name() {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model Test {
                id String @id @default(cuid())
                Group String
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let column = result.table_bang("Test").column_bang("Group");
        assert_eq!(column.tpe.family, ColumnTypeFamily::String);

        let dm2 = r#"
            model Test {
                id String @id @default(cuid())
                Group Int
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm2).sql_schema;
        let column = result.table_bang("Test").column_bang("Group");
        assert_eq!(column.tpe.family, ColumnTypeFamily::Int);
    });
}

#[test]
fn update_type_of_scalar_field_must_work() {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model Test {
                id String @id @default(cuid())
                field String
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let column1 = result.table_bang("Test").column_bang("field");
        assert_eq!(column1.tpe.family, ColumnTypeFamily::String);

        let dm2 = r#"
            model Test {
                id String @id @default(cuid())
                field Int
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm2).sql_schema;
        let column2 = result.table_bang("Test").column_bang("field");
        assert_eq!(column2.tpe.family, ColumnTypeFamily::Int);
    });
}

#[test]
fn changing_the_type_of_an_id_field_must_work() {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model A {
                id Int @id
                b  B   @relation(references: [id])
            }
            model B {
                id Int @id
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let table = result.table_bang("A");
        let column = table.column_bang("b");
        assert_eq!(column.tpe.family, ColumnTypeFamily::Int);
        assert_eq!(
            table.foreign_keys,
            &[ForeignKey {
                constraint_name: match test_setup.sql_family {
                    SqlFamily::Postgres => Some("A_b_fkey".to_owned()),
                    SqlFamily::Mysql => Some("A_ibfk_1".to_owned()),
                    SqlFamily::Sqlite => None,
                },
                columns: vec![column.name.clone()],
                referenced_table: "B".to_string(),
                referenced_columns: vec!["id".to_string()],
                on_delete_action: ForeignKeyAction::SetNull,
            }]
        );

        let dm2 = r#"
            model A {
                id Int @id
                b  B   @relation(references: [id])
            }
            model B {
                id String @id @default(cuid())
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm2).sql_schema;
        let table = result.table_bang("A");
        let column = table.column_bang("b");
        assert_eq!(column.tpe.family, ColumnTypeFamily::String);
        assert_eq!(
            table.foreign_keys,
            &[ForeignKey {
                constraint_name: match test_setup.sql_family {
                    SqlFamily::Postgres => Some("A_b_fkey".to_owned()),
                    SqlFamily::Mysql => Some("A_ibfk_1".to_owned()),
                    SqlFamily::Sqlite => None,
                },
                columns: vec![column.name.clone()],
                referenced_table: "B".to_string(),
                referenced_columns: vec!["id".to_string()],
                on_delete_action: ForeignKeyAction::SetNull,
            }]
        );
    });
}

#[test]
fn updating_db_name_of_a_scalar_field_must_work() {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model A {
                id String @id @default(cuid())
                field String @map(name:"name1")
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        assert_eq!(result.table_bang("A").column("name1").is_some(), true);

        let dm2 = r#"
            model A {
                id String @id @default(cuid())
                field String @map(name:"name2")
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm2).sql_schema;
        assert_eq!(result.table_bang("A").column("name1").is_some(), false);
        assert_eq!(result.table_bang("A").column("name2").is_some(), true);
    });
}

#[test]
fn changing_a_relation_field_to_a_scalar_field_must_work() {
    // this relies on link: INLINE which we don't support yet
    test_each_connector_with_ignores(&[SqlFamily::Mysql], |test_setup, api| {
        let dm1 = r#"
            model A {
                id Int @id
                b B @relation(references: [id])
            }
            model B {
                id Int @id
                a A // remove this once the implicit back relation field is implemented
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let table = result.table_bang("A");
        let column = table.column_bang("b");
        assert_eq!(column.tpe.family, ColumnTypeFamily::Int);
        assert_eq!(
            table.foreign_keys,
            &[ForeignKey {
                constraint_name: match test_setup.sql_family {
                    SqlFamily::Postgres => Some("A_b_fkey".to_owned()),
                    SqlFamily::Mysql => Some("A_ibfk_1".to_owned()),
                    SqlFamily::Sqlite => None,
                },
                columns: vec![column.name.clone()],
                referenced_table: "B".to_string(),
                referenced_columns: vec!["id".to_string()],
                on_delete_action: ForeignKeyAction::SetNull,
            }]
        );

        let dm2 = r#"
            model A {
                id Int @id
                b String
            }
            model B {
                id Int @id
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm2).sql_schema;
        let table = result.table_bang("A");
        let column = table.column_bang("b");
        assert_eq!(column.tpe.family, ColumnTypeFamily::String);
        assert_eq!(table.foreign_keys, vec![]);
    });
}

#[test]
fn changing_a_scalar_field_to_a_relation_field_must_work() {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model A {
                id Int @id
                b String
            }
            model B {
                id Int @id
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let table = result.table_bang("A");
        let column = table.column_bang("b");
        assert_eq!(column.tpe.family, ColumnTypeFamily::String);
        assert_eq!(table.foreign_keys, vec![]);

        let dm2 = r#"
            model A {
                id Int @id
                b B @relation(references: [id])
            }
            model B {
                id Int @id
                a A // remove this once the implicit back relation field is implemented
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm2).sql_schema;
        let table = result.table_bang("A");
        let column = result.table_bang("A").column_bang("b");
        assert_eq!(column.tpe.family, ColumnTypeFamily::Int);
        assert_eq!(
            table.foreign_keys,
            &[ForeignKey {
                constraint_name: match test_setup.sql_family {
                    SqlFamily::Postgres => Some("A_b_fkey".to_owned()),
                    SqlFamily::Mysql => Some("A_ibfk_1".to_owned()),
                    SqlFamily::Sqlite => None,
                },
                columns: vec![column.name.clone()],
                referenced_table: "B".to_string(),
                referenced_columns: vec!["id".to_string()],
                on_delete_action: ForeignKeyAction::SetNull,
            }]
        );
    });
}

#[test]
fn adding_a_many_to_many_relation_must_result_in_a_prisma_style_relation_table() {
    // TODO: one model should have an id of different type. Not possible right now due to barrel limitation.
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model A {
                id Int @id
                bs B[]
            }
            model B {
                id Int @id
                as A[]
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let relation_table = result.table_bang("_AToB");
        println!("{:?}", relation_table.foreign_keys);
        assert_eq!(relation_table.columns.len(), 2);

        let aColumn = relation_table.column_bang("A");
        assert_eq!(aColumn.tpe.family, ColumnTypeFamily::Int);
        let bColumn = relation_table.column_bang("B");
        assert_eq!(bColumn.tpe.family, ColumnTypeFamily::Int);

        assert_eq!(
            relation_table.foreign_keys,
            &[
                ForeignKey {
                    constraint_name: match test_setup.sql_family {
                        SqlFamily::Postgres => Some("_AToB_A_fkey".to_owned()),
                        SqlFamily::Mysql => Some("_AToB_ibfk_1".to_owned()),
                        SqlFamily::Sqlite => None,
                    },
                    columns: vec![aColumn.name.clone()],
                    referenced_table: "A".to_string(),
                    referenced_columns: vec!["id".to_string()],
                    on_delete_action: ForeignKeyAction::Cascade,
                },
                ForeignKey {
                    constraint_name: match test_setup.sql_family {
                        SqlFamily::Postgres => Some("_AToB_B_fkey".to_owned()),
                        SqlFamily::Mysql => Some("_AToB_ibfk_2".to_owned()),
                        SqlFamily::Sqlite => None,
                    },
                    columns: vec![bColumn.name.clone()],
                    referenced_table: "B".to_string(),
                    referenced_columns: vec!["id".to_string()],
                    on_delete_action: ForeignKeyAction::Cascade,
                },
            ]
        );
    });
}

#[test]
fn adding_a_many_to_many_relation_with_custom_name_must_work() {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model A {
                id Int @id
                bs B[] @relation(name: "my_relation")
            }
            model B {
                id Int @id
                as A[] @relation(name: "my_relation")
            }
        "#;

        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let relation_table = result.table_bang("_my_relation");
        assert_eq!(relation_table.columns.len(), 2);

        let aColumn = relation_table.column_bang("A");
        assert_eq!(aColumn.tpe.family, ColumnTypeFamily::Int);
        let bColumn = relation_table.column_bang("B");
        assert_eq!(bColumn.tpe.family, ColumnTypeFamily::Int);

        assert_eq!(
            relation_table.foreign_keys,
            vec![
                ForeignKey {
                    constraint_name: match test_setup.sql_family {
                        SqlFamily::Postgres => Some("_my_relation_A_fkey".to_owned()),
                        SqlFamily::Mysql => Some("_my_relation_ibfk_1".to_owned()),
                        SqlFamily::Sqlite => None,
                    },
                    columns: vec![aColumn.name.clone()],
                    referenced_table: "A".to_string(),
                    referenced_columns: vec!["id".to_string()],
                    on_delete_action: ForeignKeyAction::Cascade,
                },
                ForeignKey {
                    constraint_name: match test_setup.sql_family {
                        SqlFamily::Postgres => Some("_my_relation_B_fkey".to_owned()),
                        SqlFamily::Mysql => Some("_my_relation_ibfk_2".to_owned()),
                        SqlFamily::Sqlite => None,
                    },
                    columns: vec![bColumn.name.clone()],
                    referenced_table: "B".to_string(),
                    referenced_columns: vec!["id".to_string()],
                    on_delete_action: ForeignKeyAction::Cascade,
                }
            ]
        );
    });
}

#[test]
#[ignore]
fn adding_a_many_to_many_relation_for_exotic_id_types_must_work() {
    // TODO: add this once we have figured out what id types we support
    unimplemented!();
}

#[test]
#[ignore]
fn forcing_a_relation_table_for_a_one_to_many_relation_must_work() {
    // TODO: implement this once we have decided if this is actually possible in dm v2
    unimplemented!();
}

// #[test]
// #[ignore]
// fn forcing_a_relation_table_for_a_one_to_many_relation_must_work() {
//     // TODO: implement this once we have decided if this is actually possible in dm v2
//     unimplemented!();
// }

#[test]
#[ignore]
fn providing_an_explicit_link_table_must_work() {
    // TODO: implement this once we have decided if this is actually possible in dm v2
    unimplemented!();
}

#[test]
fn adding_an_inline_relation_must_result_in_a_foreign_key_in_the_model_table() {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model A {
                id Int @id
                b B @relation(references: [id])
            }

            model B {
                id Int @id
            }
        "#;
        let result = dbg!(infer_and_apply(test_setup, api, &dm1).sql_schema);
        let table = result.table_bang("A");
        let column = table.column_bang("b");
        assert_eq!(column.tpe.family, ColumnTypeFamily::Int);
        assert_eq!(
            table.foreign_keys,
            &[ForeignKey {
                constraint_name: match test_setup.sql_family {
                    SqlFamily::Postgres => Some("A_b_fkey".to_owned()),
                    SqlFamily::Mysql => Some("A_ibfk_1".to_owned()),
                    SqlFamily::Sqlite => None,
                },
                columns: vec![column.name.clone()],
                referenced_table: "B".to_string(),
                referenced_columns: vec!["id".to_string()],
                on_delete_action: ForeignKeyAction::SetNull,
            }]
        );
    });
}

#[test]
fn specifying_a_db_name_for_an_inline_relation_must_work() {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model A {
                id Int @id
                b B @relation(references: [id]) @map(name: "b_column")
            }

            model B {
                id Int @id
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let table = result.table_bang("A");
        let column = table.column_bang("b_column");
        assert_eq!(column.tpe.family, ColumnTypeFamily::Int);
        assert_eq!(
            table.foreign_keys,
            &[ForeignKey {
                constraint_name: match test_setup.sql_family {
                    SqlFamily::Postgres => Some("A_b_column_fkey".to_owned()),
                    SqlFamily::Mysql => Some("A_ibfk_1".to_owned()),
                    SqlFamily::Sqlite => None,
                },
                columns: vec![column.name.clone()],
                referenced_table: "B".to_string(),
                referenced_columns: vec!["id".to_string()],
                on_delete_action: ForeignKeyAction::SetNull,
            }]
        );
    });
}

#[test]
fn adding_an_inline_relation_to_a_model_with_an_exotic_id_type() {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model A {
                id Int @id
                b B @relation(references: [id])
            }

            model B {
                id String @id @default(cuid())
            }
        "#;
        let result = dbg!(infer_and_apply(test_setup, api, &dm1).sql_schema);
        let table = result.table_bang("A");
        let column = table.column_bang("b");
        assert_eq!(column.tpe.family, ColumnTypeFamily::String);
        assert_eq!(
            table.foreign_keys,
            &[ForeignKey {
                constraint_name: match test_setup.sql_family {
                    SqlFamily::Postgres => Some("A_b_fkey".to_owned()),
                    SqlFamily::Mysql => Some("A_ibfk_1".to_owned()),
                    SqlFamily::Sqlite => None,
                },
                columns: vec![column.name.clone()],
                referenced_table: "B".to_string(),
                referenced_columns: vec!["id".to_string()],
                on_delete_action: ForeignKeyAction::SetNull,
            }]
        );
    });
}

#[test]
fn removing_an_inline_relation_must_work() {
    test_each_connector_with_ignores(vec![SqlFamily::Mysql], |test_setup, api| {
        let dm1 = r#"
            model A {
                id Int @id
                b B @relation(references: [id])
            }

            model B {
                id Int @id
            }
        "#;
        let result = dbg!(infer_and_apply(test_setup, api, &dm1).sql_schema);
        let column = result.table_bang("A").column("b");
        assert_eq!(column.is_some(), true);

        let dm2 = r#"
            model A {
                id Int @id
            }

            model B {
                id Int @id
            }
        "#;
        let result = dbg!(infer_and_apply(test_setup, api, &dm2).sql_schema);
        let column = result.table_bang("A").column("b");
        assert_eq!(column.is_some(), false);
    });
}

#[test]
fn moving_an_inline_relation_to_the_other_side_must_work() {
    test_each_connector_with_ignores(vec![SqlFamily::Mysql], |test_setup, api| {
        let dm1 = r#"
            model A {
                id Int @id
                b B @relation(references: [id])
            }

            model B {
                id Int @id
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let table = result.table_bang("A");
        assert_eq!(
            table.foreign_keys,
            &[ForeignKey {
                constraint_name: match test_setup.sql_family {
                    SqlFamily::Postgres => Some("A_b_fkey".to_owned()),
                    SqlFamily::Sqlite => None,
                    SqlFamily::Mysql => unreachable!(),
                },
                columns: vec!["b".to_string()],
                referenced_table: "B".to_string(),
                referenced_columns: vec!["id".to_string()],
                on_delete_action: ForeignKeyAction::SetNull,
            }]
        );

        let dm2 = r#"
            model A {
                id Int @id
            }

            model B {
                id Int @id
                a A @relation(references: [id])
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm2).sql_schema;
        let table = result.table_bang("B");
        assert_eq!(
            table.foreign_keys,
            &[ForeignKey {
                constraint_name: match test_setup.sql_family {
                    SqlFamily::Postgres => Some("B_a_fkey".to_owned()),
                    SqlFamily::Sqlite => None,
                    SqlFamily::Mysql => unreachable!(),
                },
                columns: vec!["a".to_string()],
                referenced_table: "A".to_string(),
                referenced_columns: vec!["id".to_string()],
                on_delete_action: ForeignKeyAction::SetNull,
            }]
        );
    });
}

#[test]
fn adding_a_new_unique_field_must_work() {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model A {
                id Int @id
                field String @unique
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let index = result.table_bang("A").indices.iter().find(|i| i.columns == &["field"]);
        assert!(index.is_some());
        assert_eq!(index.unwrap().tpe, IndexType::Unique);
    });
}

#[test]
fn adding_new_fields_with_multi_column_unique_must_work() {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model A {
                id Int @id
                field String
                secondField String

                @@unique([field, secondField])
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let index = result
            .table_bang("A")
            .indices
            .iter()
            .find(|i| i.columns == vec!["field", "secondField"]);
        assert!(index.is_some());
        assert_eq!(index.unwrap().tpe, IndexType::Unique);
    });
}

#[test]
fn unique_in_conjunction_with_custom_column_name_must_work() {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model A {
                id Int @id
                field String @unique @map("custom_field_name")
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let index = result
            .table_bang("A")
            .indices
            .iter()
            .find(|i| i.columns == &["custom_field_name"]);
        assert!(index.is_some());
        assert_eq!(index.unwrap().tpe, IndexType::Unique);
    });
}

#[test]
fn multi_column_unique_in_conjunction_with_custom_column_name_must_work() {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model A {
                id Int @id
                field String @map("custom_field_name")
                secondField String @map("second_custom_field_name")

                @@unique([field, secondField])
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let index = result
            .table_bang("A")
            .indices
            .iter()
            .find(|i| i.columns == &["custom_field_name", "second_custom_field_name"]);
        assert!(index.is_some());
        assert_eq!(index.unwrap().tpe, IndexType::Unique);
    });
}

#[test]
fn sqlite_must_recreate_indexes() {
    // SQLite must go through a complicated migration procedure which requires dropping and recreating indexes. This test checks that.
    // We run them still against each connector.
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model A {
                id Int @id
                field String @unique
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let index = result
            .table_bang("A")
            .indices
            .iter()
            .find(|i| i.columns == vec!["field"]);
        assert!(index.is_some());
        assert_eq!(index.unwrap().tpe, IndexType::Unique);

        let dm2 = r#"
            model A {
                id    Int    @id
                field String @unique
                other String
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm2).sql_schema;
        let index = result
            .table_bang("A")
            .indices
            .iter()
            .find(|i| i.columns == vec!["field"]);
        assert!(index.is_some());
        assert_eq!(index.unwrap().tpe, IndexType::Unique);
    });
}

#[test]
fn sqlite_must_recreate_multi_field_indexes() {
    // SQLite must go through a complicated migration procedure which requires dropping and recreating indexes. This test checks that.
    // We run them still against each connector.
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model A {
                id Int @id
                field String
                secondField Int

                @@unique([field, secondField])
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let index = result
            .table_bang("A")
            .indices
            .iter()
            .find(|i| i.columns == &["field", "secondField"]);
        assert!(index.is_some());
        assert_eq!(index.unwrap().tpe, IndexType::Unique);

        let dm2 = r#"
            model A {
                id    Int    @id
                field String
                secondField Int
                other String

                @@unique([field, secondField])
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm2).sql_schema;
        let index = result
            .table_bang("A")
            .indices
            .iter()
            .find(|i| i.columns == &["field", "secondField"]);
        assert!(index.is_some());
        assert_eq!(index.unwrap().tpe, IndexType::Unique);
    });
}

#[test]
fn removing_an_existing_unique_field_must_work() {
    //    test_only_connector(SqlFamily::Postgres, |test_setup, api| {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model A {
                id    Int    @id
                field String @unique
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let index = result
            .table_bang("A")
            .indices
            .iter()
            .find(|i| i.columns == vec!["field"]);
        assert!(index.is_some());
        assert_eq!(index.unwrap().tpe, IndexType::Unique);

        let dm2 = r#"
            model A {
                id    Int    @id
            }
        "#;
        let result = dbg!(infer_and_apply(test_setup, api, &dm2).sql_schema);
        let index = result
            .table_bang("A")
            .indices
            .iter()
            .find(|i| i.columns == vec!["field"]);
        assert_eq!(index.is_some(), false);
    });
}

#[test]
fn adding_unique_to_an_existing_field_must_work() {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model A {
                id    Int    @id
                field String
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let index = result
            .table_bang("A")
            .indices
            .iter()
            .find(|i| i.columns == vec!["field"]);
        assert_eq!(index.is_some(), false);

        let dm2 = r#"
            model A {
                id    Int    @id
                field String @unique
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm2).sql_schema;
        let index = result
            .table_bang("A")
            .indices
            .iter()
            .find(|i| i.columns == vec!["field"]);
        assert!(index.is_some());
        assert_eq!(index.unwrap().tpe, IndexType::Unique);
    });
}

#[test]
fn removing_unique_from_an_existing_field_must_work() {
    //    test_only_connector(SqlFamily::Postgres, |test_setup, api| {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model A {
                id    Int    @id
                field String @unique
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let index = result.table_bang("A").indices.iter().find(|i| i.columns == &["field"]);
        assert!(index.is_some());
        assert_eq!(index.unwrap().tpe, IndexType::Unique);

        let dm2 = r#"
            model A {
                id    Int    @id
                field String
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm2).sql_schema;
        let index = result.table_bang("A").indices.iter().find(|i| i.columns == &["field"]);
        assert!(!index.is_some());
    });
}

#[test]
fn removing_multi_field_unique_index_must_work() {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model A {
                id    Int    @id
                field String
                secondField Int

                @@unique([field, secondField])
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let index = result
            .table_bang("A")
            .indices
            .iter()
            .find(|i| i.columns == &["field", "secondField"]);
        assert!(index.is_some());
        assert_eq!(index.unwrap().tpe, IndexType::Unique);

        let dm2 = r#"
            model A {
                id    Int    @id
                field String
                secondField Int
            }
        "#;
        let result = dbg!(infer_and_apply(test_setup, api, &dm2).sql_schema);
        let index = result
            .table_bang("A")
            .indices
            .iter()
            .find(|i| i.columns == &["field", "secondField"]);
        assert!(index.is_none());
    });
}

#[test]
fn index_renaming_must_work() {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model A {
                id Int @id
                field String
                secondField Int

                @@unique([field, secondField], name: "customName")
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let index = result
            .table_bang("A")
            .indices
            .iter()
            .find(|i| i.name == "customName" && i.columns == &["field", "secondField"]);
        assert!(index.is_some());
        assert_eq!(index.unwrap().tpe, IndexType::Unique);

        let dm2 = r#"
            model A {
                id Int @id
                field String
                secondField Int

                @@unique([field, secondField], name: "customNameA")
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm2);
        let indexes = result
            .sql_schema
            .table_bang("A")
            .indices
            .iter()
            .filter(|i| i.columns == &["field", "secondField"] && i.name == "customNameA");
        assert_eq!(indexes.count(), 1);

        // Test that we are not dropping and recreating the index. Except in SQLite, because there we are.
        if test_setup.sql_family != SqlFamily::Sqlite {
            let expected_steps = vec![SqlMigrationStep::AlterIndex(AlterIndex {
                table: "A".into(),
                index_new_name: "customNameA".into(),
                index_name: "customName".into(),
            })];
            let actual_steps = result.sql_migration();
            assert_eq!(actual_steps, expected_steps);
        }
    });
}

#[test]
fn index_renaming_must_work_when_renaming_to_default() {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model A {
                id Int @id
                field String
                secondField Int

                @@unique([field, secondField], name: "customName")
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1);
        let index = result
            .sql_schema
            .table_bang("A")
            .indices
            .iter()
            .find(|i| i.columns == &["field", "secondField"]);
        assert!(index.is_some());
        assert_eq!(index.unwrap().tpe, IndexType::Unique);

        let dm2 = r#"
            model A {
                id Int @id
                field String
                secondField Int

                @@unique([field, secondField])
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm2);
        let indexes = result
            .sql_schema
            .table_bang("A")
            .indices
            .iter()
            .filter(|i| i.columns == &["field", "secondField"] && i.name == "A.field_secondField");
        assert_eq!(indexes.count(), 1);

        // Test that we are not dropping and recreating the index. Except in SQLite, because there we are.
        if test_setup.sql_family != SqlFamily::Sqlite {
            let expected_steps = vec![SqlMigrationStep::AlterIndex(AlterIndex {
                table: "A".into(),
                index_new_name: "A.field_secondField".into(),
                index_name: "customName".into(),
            })];
            let actual_steps = result.sql_migration();
            assert_eq!(actual_steps, expected_steps);
        }
    });
}

#[test]
fn index_renaming_must_work_when_renaming_to_custom() {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model A {
                id Int @id
                field String
                secondField Int

                @@unique([field, secondField])
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1);
        let index = result
            .sql_schema
            .table_bang("A")
            .indices
            .iter()
            .find(|i| i.columns == &["field", "secondField"]);
        assert!(index.is_some());
        assert_eq!(index.unwrap().tpe, IndexType::Unique);

        let dm2 = r#"
            model A {
                id Int @id
                field String
                secondField Int

                @@unique([field, secondField], name: "somethingCustom")
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm2);
        let indexes = result
            .sql_schema
            .table_bang("A")
            .indices
            .iter()
            .filter(|i| i.columns == &["field", "secondField"] && i.name == "somethingCustom");
        assert_eq!(indexes.count(), 1);

        // Test that we are not dropping and recreating the index. Except in SQLite, because there we are.
        if test_setup.sql_family != SqlFamily::Sqlite {
            let expected_steps = vec![SqlMigrationStep::AlterIndex(AlterIndex {
                table: "A".into(),
                index_name: "A.field_secondField".into(),
                index_new_name: "somethingCustom".into(),
            })];
            let actual_steps = result.sql_migration();
            assert_eq!(actual_steps, expected_steps);
        }
    });
}

#[test]
fn index_updates_with_rename_must_work() {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model A {
                id Int @id
                field String
                secondField Int

                @@unique([field, secondField], name: "customName")
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let index = result
            .table_bang("A")
            .indices
            .iter()
            .find(|i| i.name == "customName" && i.columns == &["field", "secondField"]);
        assert!(index.is_some());
        assert_eq!(index.unwrap().tpe, IndexType::Unique);

        let dm2 = r#"
            model A {
                id Int @id
                field String
                secondField Int

                @@unique([field, id], name: "customNameA")
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm2);
        let indexes = result
            .sql_schema
            .table_bang("A")
            .indices
            .iter()
            .filter(|i| i.columns == &["field", "id"] && i.name == "customNameA");
        assert_eq!(indexes.count(), 1);

        // Test that we are not dropping and recreating the index. Except in SQLite, because there we are.
        if test_setup.sql_family != SqlFamily::Sqlite {
            let expected_steps = vec![
                SqlMigrationStep::DropIndex(DropIndex {
                    table: "A".into(),
                    name: "customName".into(),
                }),
                SqlMigrationStep::CreateIndex(CreateIndex {
                    table: "A".into(),
                    index: Index {
                        name: "customNameA".into(),
                        columns: vec!["field".into(), "id".into()],
                        tpe: IndexType::Unique,
                    },
                }),
            ];
            let actual_steps = result.sql_migration();
            assert_eq!(actual_steps, expected_steps);
        }
    });
}

#[test]
fn dropping_a_model_with_a_multi_field_unique_index_must_work() {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model A {
                id Int @id
                field String
                secondField Int

                @@unique([field, secondField], name: "customName")
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let index = result
            .table_bang("A")
            .indices
            .iter()
            .find(|i| i.name == "customName" && i.columns == &["field", "secondField"]);
        assert!(index.is_some());
        assert_eq!(index.unwrap().tpe, IndexType::Unique);

        let dm2 = r#""#;
        infer_and_apply(test_setup, api, &dm2);
    })
}

#[test]
fn adding_a_scalar_list_for_a_modelwith_id_type_int_must_work() {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model A {
                id Int @id
                strings String[]
                enums Status[]
            }

            enum Status {
              OK
              ERROR
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm1).sql_schema;
        let scalar_list_table_for_strings = result.table_bang("A_strings");
        let node_id_column = scalar_list_table_for_strings.column_bang("nodeId");
        assert_eq!(node_id_column.tpe.family, ColumnTypeFamily::Int);
        assert_eq!(
            scalar_list_table_for_strings.primary_key_columns(),
            vec!["nodeId", "position"]
        );
        let scalar_list_table_for_enums = result.table_bang("A_enums");
        let node_id_column = scalar_list_table_for_enums.column_bang("nodeId");
        assert_eq!(node_id_column.tpe.family, ColumnTypeFamily::Int);
        assert_eq!(
            scalar_list_table_for_enums.primary_key_columns(),
            vec!["nodeId", "position"]
        );
    });
}

#[test]
fn updating_a_model_with_a_scalar_list_to_a_different_id_type_must_work() {
    test_each_connector_with_ignores(vec![SqlFamily::Mysql], |test_setup, api| {
        let dm = r#"
            model A {
                id Int @id
                strings String[]
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm).sql_schema;
        let node_id_column = result.table_bang("A_strings").column_bang("nodeId");
        assert_eq!(node_id_column.tpe.family, ColumnTypeFamily::Int);

        let dm = r#"
            model A {
                id String @id @default(cuid())
                strings String[]
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm).sql_schema;
        let node_id_column = result.table_bang("A_strings").column_bang("nodeId");
        assert_eq!(node_id_column.tpe.family, ColumnTypeFamily::String);
    });
}

#[test]
fn reserved_sql_key_words_must_work() {
    // Group is a reserved keyword
    test_each_connector(|test_setup, api| {
        let sql_family = test_setup.sql_family;
        let dm = r#"
            model Group {
                id    String  @default(cuid()) @id
                parent Group? @relation(name: "ChildGroups")
                childGroups Group[] @relation(name: "ChildGroups")
            }
        "#;
        let result = infer_and_apply(test_setup, api, &dm).sql_schema;

        let table = result.table_bang("Group");
        let relation_column = table.column_bang("parent");
        assert_eq!(
            table.foreign_keys,
            vec![ForeignKey {
                constraint_name: match sql_family {
                    SqlFamily::Postgres => Some("Group_parent_fkey".to_owned()),
                    SqlFamily::Mysql => Some("Group_ibfk_1".to_owned()),
                    SqlFamily::Sqlite => None,
                },
                columns: vec!["parent".to_string()],
                referenced_table: "Group".to_string(),
                referenced_columns: vec!["id".to_string()],
                on_delete_action: ForeignKeyAction::SetNull,
            }]
        );
    });
}

#[test]
fn migrations_with_many_to_many_related_models_must_not_recreate_indexes() {
    // test case for https://github.com/prisma/lift/issues/148
    test_each_connector(|test_setup, api| {
        let dm_1 = r#"
            model User {
                id        String  @default(cuid()) @id
            }

            model Profile {
                id        String  @default(cuid()) @id
                user      User
                skills    Skill[]
            }

            model Skill {
                id          String  @default(cuid()) @id
                profiles    Profile[]
            }
        "#;
        let sql_schema = infer_and_apply(test_setup, api, &dm_1).sql_schema;

        let index = sql_schema
            .table_bang("_ProfileToSkill")
            .indices
            .iter()
            .find(|index| index.name == "_ProfileToSkill_AB_unique")
            .expect("index is present");
        assert_eq!(index.tpe, IndexType::Unique);

        let dm_2 = r#"
            model User {
                id        String  @default(cuid()) @id
                someField String?
            }

            model Profile {
                id        String  @default(cuid()) @id
                user      User
                skills    Skill[]
            }

            model Skill {
                id          String  @default(cuid()) @id
                profiles    Profile[]
            }
        "#;

        let result = infer_and_apply(test_setup, api, &dm_1);
        let sql_schema = result.sql_schema;

        let index = sql_schema
            .table_bang("_ProfileToSkill")
            .indices
            .iter()
            .find(|index| index.name == "_ProfileToSkill_AB_unique")
            .expect("index is present");
        assert_eq!(index.tpe, IndexType::Unique);
    })
}

#[test]
fn removing_a_relation_field_must_work() {
    test_each_connector(|test_setup, api| {
        let sql_family = test_setup.sql_family;

        let dm_1 = r#"
            model User {
                id        String  @default(cuid()) @id
                address   Address @map("address_name")
            }

            model Address {
                id        String  @default(cuid()) @id
                street    String
            }
        "#;

        let sql_schema = infer_and_apply(test_setup, api, &dm_1).sql_schema;

        let address_name_field = sql_schema
            .table_bang("User")
            .columns
            .iter()
            .find(|col| col.name == "address_name");

        assert!(address_name_field.is_some());

        let dm_2 = r#"
            model User {
                id        String  @default(cuid()) @id
            }

            model Address {
                id        String  @default(cuid()) @id
                street    String
            }
        "#;

        let sql_schema = infer_and_apply(test_setup, api, &dm_2).sql_schema;

        let address_name_field = sql_schema
            .table_bang("User")
            .columns
            .iter()
            .find(|col| col.name == "address_name");

        assert!(address_name_field.is_none());
    })
}
