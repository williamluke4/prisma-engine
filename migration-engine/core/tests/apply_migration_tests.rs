#![allow(non_snake_case)]
#![allow(unused)]
mod test_harness;
use datamodel::dml::*;
use migration_connector::*;
use test_harness::*;

#[test]
fn single_watch_migrations_must_work() {
    test_each_connector(|test_setup, api| {
        let migration_persistence = api.migration_persistence();

        let steps = vec![
            create_model_step("Test"),
            create_id_field_step("Test", "id", ScalarType::Int),
        ];

        let db_schema_1 = apply_migration(test_setup, api, steps.clone(), "watch-0001").sql_schema;
        let migrations = migration_persistence.load_all().wait();

        assert_eq!(migrations.len(), 1);
        assert_eq!(migrations.first().unwrap().name, "watch-0001");

        let custom_migration_id = "a-custom-migration-id";
        let db_schema_2 = apply_migration(test_setup, api, steps.clone(), custom_migration_id).sql_schema;

        assert_eq!(db_schema_1, db_schema_2);

        let migrations = migration_persistence.load_all().wait();

        assert_eq!(migrations.len(), 2);
        assert_eq!(migrations[0].name, "watch-0001");
        assert_eq!(migrations[1].name, custom_migration_id);
        assert_eq!(migrations[1].status, MigrationStatus::MigrationSuccess);
        assert!(migrations[1].finished_at.is_some());
    });
}

#[test]
fn multiple_watch_migrations_must_work() {
    test_each_connector(|test_setup, api| {
        let migration_persistence = api.migration_persistence();

        let steps1 = vec![
            create_model_step("Test"),
            create_id_field_step("Test", "id", ScalarType::Int),
        ];

        let _ = apply_migration(test_setup, api, steps1.clone(), "watch-0001");
        let migrations = dbg!(migration_persistence.load_all().wait());

        assert_eq!(migrations.len(), 1);
        assert_eq!(migrations[0].name, "watch-0001");

        let steps2 = vec![create_field_step("Test", "field", ScalarType::String)];
        let db_schema_2 = apply_migration(test_setup, api, steps2.clone(), "watch-0002").sql_schema;
        let migrations = migration_persistence.load_all().wait();

        assert_eq!(migrations.len(), 2);
        assert_eq!(migrations[0].name, "watch-0001");
        assert_eq!(migrations[1].name, "watch-0002");

        let custom_migration_id = "a-custom-migration-id";
        let mut final_steps = Vec::new();

        final_steps.append(&mut steps1.clone());
        final_steps.append(&mut steps2.clone());

        let final_db_schema = apply_migration(test_setup, api, final_steps, custom_migration_id).sql_schema;

        assert_eq!(db_schema_2, final_db_schema);

        let migrations = migration_persistence.load_all().wait();

        assert_eq!(migrations.len(), 3);
        assert_eq!(migrations[0].name, "watch-0001");
        assert_eq!(migrations[1].name, "watch-0002");

        assert_eq!(migrations[2].name, custom_migration_id);
        assert_eq!(migrations[2].status, MigrationStatus::MigrationSuccess);
        assert!(migrations[2].finished_at.is_some());
    });
}

#[test]
fn steps_equivalence_criteria_is_satisfied_when_leaving_watch_mode() {
    test_each_connector(|test_setup, api| {
        let migration_persistence = api.migration_persistence();

        let steps1 = vec![
            create_model_step("Test"),
            create_id_field_step("Test", "id", ScalarType::Int),
        ];

        let db_schema1 = apply_migration(test_setup, api, steps1.clone(), "watch-0001").sql_schema;

        let steps2 = vec![create_field_step("Test", "field", ScalarType::String)];
        let _ = apply_migration(test_setup, api, steps2.clone(), "watch-0002");

        let steps3 = vec![delete_field_step("Test", "field")];
        let _ = apply_migration(test_setup, api, steps3.clone(), "watch-0003");

        let custom_migration_id = "a-custom-migration-id";
        let mut final_steps = Vec::new();
        final_steps.append(&mut steps1.clone()); // steps2 and steps3 eliminate each other

        let final_db_schema = apply_migration(test_setup, api, final_steps, custom_migration_id).sql_schema;
        assert_eq!(db_schema1, final_db_schema);
        let migrations = migration_persistence.load_all().wait();
        assert_eq!(migrations[0].name, "watch-0001");
        assert_eq!(migrations[1].name, "watch-0002");
        assert_eq!(migrations[2].name, "watch-0003");
        assert_eq!(migrations[3].name, custom_migration_id);
    });
}

#[test]
fn must_handle_additional_steps_when_transitioning_out_of_watch_mode() {
    test_each_connector(|test_setup, api| {
        let migration_persistence = api.migration_persistence();

        let steps1 = vec![
            create_model_step("Test"),
            create_id_field_step("Test", "id", ScalarType::Int),
        ];

        let _ = apply_migration(test_setup, api, steps1.clone(), "watch-0001");

        let steps2 = vec![create_field_step("Test", "field1", ScalarType::String)];
        let _ = apply_migration(test_setup, api, steps2.clone(), "watch-0002");

        let custom_migration_id = "a-custom-migration-id";
        let additional_steps = vec![create_field_step("Test", "field2", ScalarType::String)];
        let mut final_steps = Vec::new();

        final_steps.append(&mut steps1.clone());
        final_steps.append(&mut steps2.clone());
        final_steps.append(&mut additional_steps.clone());

        let final_db_schema = apply_migration(test_setup, api, final_steps, custom_migration_id).sql_schema;
        assert_eq!(final_db_schema.tables.len(), 1);
        let table = final_db_schema.table_bang("Test");
        assert_eq!(table.columns.len(), 3);
        table.column_bang("id");
        table.column_bang("field1");
        table.column_bang("field2");

        let migrations = migration_persistence.load_all().wait();
        assert_eq!(migrations[0].name, "watch-0001");
        assert_eq!(migrations[1].name, "watch-0002");
        assert_eq!(migrations[2].name, custom_migration_id);
    });
}
