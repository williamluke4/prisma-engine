use super::MigrationDatabase;
use super::SqlFamily;
use barrel::types;
use chrono::*;
use migration_connector::*;
use prisma_query::ast::*;
use prisma_query::connector::ResultSet;
use serde_json;
use std::sync::Arc;

pub struct SqlMigrationPersistence {
    pub sql_family: SqlFamily,
    pub connection: Arc<dyn MigrationDatabase + Send + Sync + 'static>,
    pub schema_name: String,
    pub file_path: Option<String>,
}

#[async_trait::async_trait]
impl MigrationPersistence for SqlMigrationPersistence {
    async fn init(&self) {
        let sql_str = match self.sql_family {
            SqlFamily::Sqlite => {
                let mut m = barrel::Migration::new().schema(self.schema_name.clone());
                m.create_table_if_not_exists(TABLE_NAME, migration_table_setup_sqlite);
                m.make_from(barrel::SqlVariant::Sqlite)
            }
            SqlFamily::Postgres => {
                let mut m = barrel::Migration::new().schema(self.schema_name.clone());
                m.create_table(TABLE_NAME, migration_table_setup_postgres);
                m.make_from(barrel::SqlVariant::Pg)
            }
            SqlFamily::Mysql => {
                // work around barrels missing quoting
                let mut m = barrel::Migration::new().schema(format!("{}", self.schema_name.clone()));
                m.create_table(format!("{}", TABLE_NAME), migration_table_setup_mysql);
                m.make_from(barrel::SqlVariant::Mysql)
            }
        };

        let _ = self.connection.query_raw(&self.schema_name, dbg!(&sql_str), &[]);
    }

    async fn reset(&self) {
        let sql_str = format!(r#"DELETE FROM "{}"."_Migration";"#, self.schema_name); // TODO: this is not vendor agnostic yet
        let _ = self.connection.query_raw(&self.schema_name, &sql_str, &[]);

        // TODO: this is the wrong place to do that
        match self.sql_family {
            SqlFamily::Postgres => {
                let sql_str = format!(r#"DROP SCHEMA "{}" CASCADE;"#, self.schema_name);
                debug!("{}", sql_str);

                let _ = self.connection.query_raw(&self.schema_name, &sql_str, &[]);
            }
            SqlFamily::Sqlite => {
                if let Some(ref file_path) = self.file_path {
                    let _ = std::fs::remove_file(file_path); // ignore potential errors
                }
            }
            SqlFamily::Mysql => {
                let sql_str = format!(r#"DROP SCHEMA `{}`;"#, self.schema_name);
                debug!("{}", sql_str);
                let _ = self.connection.query_raw(&self.schema_name, &sql_str, &[]);
            }
        }
    }

    async fn last(&self) -> Option<Migration> {
        let conditions = STATUS_COLUMN.equals(MigrationStatus::MigrationSuccess.code());
        let query = Select::from_table(self.table())
            .so_that(conditions)
            .order_by(REVISION_COLUMN.descend());

        let result_set = self.connection.query(&self.schema_name, query.into()).unwrap();
        parse_rows_new(result_set).into_iter().next()
    }

    async fn load_all(&self) -> Vec<Migration> {
        let query = Select::from_table(self.table()).order_by(REVISION_COLUMN.ascend());

        let result_set = self.connection.query(&self.schema_name, query.into()).unwrap();
        parse_rows_new(result_set)
    }

    async fn by_name(&self, name: &str) -> Option<Migration> {
        let conditions = NAME_COLUMN.equals(name);
        let query = Select::from_table(self.table())
            .so_that(conditions)
            .order_by(REVISION_COLUMN.descend());

        let result_set = self.connection.query(&self.schema_name, query.into()).unwrap();
        parse_rows_new(result_set).into_iter().next()
    }

    async fn create(&self, migration: Migration) -> Migration {
        let mut cloned = migration.clone();
        let model_steps_json = serde_json::to_string(&migration.datamodel_steps).unwrap();
        let database_migration_json = serde_json::to_string(&migration.database_migration).unwrap();
        let errors_json = serde_json::to_string(&migration.errors).unwrap();
        let serialized_datamodel = datamodel::render_datamodel_to_string(&migration.datamodel).unwrap();

        let insert = Insert::single_into(self.table())
            .value(NAME_COLUMN, migration.name)
            .value(DATAMODEL_COLUMN, serialized_datamodel)
            .value(STATUS_COLUMN, migration.status.code())
            .value(APPLIED_COLUMN, migration.applied)
            .value(ROLLED_BACK_COLUMN, migration.rolled_back)
            .value(DATAMODEL_STEPS_COLUMN, model_steps_json)
            .value(DATABASE_MIGRATION_COLUMN, database_migration_json)
            .value(ERRORS_COLUMN, errors_json)
            .value(STARTED_AT_COLUMN, self.convert_datetime(migration.started_at))
            .value(FINISHED_AT_COLUMN, ParameterizedValue::Null);

        match self.sql_family {
            SqlFamily::Sqlite | SqlFamily::Mysql => {
                let id = self.connection.execute(&self.schema_name, insert.into()).unwrap();
                match id {
                    Some(prisma_query::ast::Id::Int(id)) => cloned.revision = id,
                    _ => panic!("This insert must return an int"),
                };
            }
            SqlFamily::Postgres => {
                let returning_insert = Insert::from(insert).returning(vec!["revision"]);
                let result_set = self
                    .connection
                    .query(&self.schema_name, returning_insert.into())
                    .unwrap();
                result_set.into_iter().next().map(|row| {
                    cloned.revision = row["revision"].as_i64().unwrap() as usize;
                });
            }
        }
        cloned
    }

    async fn update(&self, params: &MigrationUpdateParams) {
        let finished_at_value = match params.finished_at {
            Some(x) => self.convert_datetime(x),
            None => ParameterizedValue::Null,
        };
        let errors_json = serde_json::to_string(&params.errors).unwrap();
        let query = Update::table(self.table())
            .set(NAME_COLUMN, params.new_name.clone())
            .set(STATUS_COLUMN, params.status.code())
            .set(APPLIED_COLUMN, params.applied)
            .set(ROLLED_BACK_COLUMN, params.rolled_back)
            .set(ERRORS_COLUMN, errors_json)
            .set(FINISHED_AT_COLUMN, finished_at_value)
            .so_that(
                NAME_COLUMN
                    .equals(params.name.clone())
                    .and(REVISION_COLUMN.equals(params.revision)),
            );

        let _ = self.connection.query(&self.schema_name, query.into()).unwrap();
    }
}

fn migration_table_setup_sqlite(t: &mut barrel::Table) {
    migration_table_setup(t, types::date(), types::custom("TEXT"));
}

fn migration_table_setup_postgres(t: &mut barrel::Table) {
    migration_table_setup(t, types::custom("timestamp(3)"), types::custom("TEXT"));
}

fn migration_table_setup_mysql(t: &mut barrel::Table) {
    migration_table_setup(t, types::custom("datetime(3)"), types::custom("LONGTEXT"));
}

fn migration_table_setup(
    t: &mut barrel::Table,
    datetime_type: barrel::types::Type,
    unlimited_text_type: barrel::types::Type,
) {
    t.add_column(REVISION_COLUMN, types::primary());
    t.add_column(NAME_COLUMN, types::text());
    t.add_column(DATAMODEL_COLUMN, unlimited_text_type.clone());
    t.add_column(STATUS_COLUMN, types::text());
    t.add_column(APPLIED_COLUMN, types::integer());
    t.add_column(ROLLED_BACK_COLUMN, types::integer());
    t.add_column(DATAMODEL_STEPS_COLUMN, unlimited_text_type.clone());
    t.add_column(DATABASE_MIGRATION_COLUMN, unlimited_text_type.clone());
    t.add_column(ERRORS_COLUMN, unlimited_text_type.clone());
    t.add_column(STARTED_AT_COLUMN, datetime_type.clone());
    t.add_column(FINISHED_AT_COLUMN, datetime_type.clone().nullable(true));
}

impl SqlMigrationPersistence {
    fn table(&self) -> Table {
        match self.sql_family {
            SqlFamily::Sqlite => {
                // sqlite case. Otherwise prisma-query produces invalid SQL
                TABLE_NAME.to_string().into()
            }
            _ => (self.schema_name.to_string(), TABLE_NAME.to_string()).into(),
        }
    }

    fn convert_datetime(&self, datetime: DateTime<Utc>) -> ParameterizedValue {
        match self.sql_family {
            SqlFamily::Sqlite => ParameterizedValue::Integer(datetime.timestamp_millis()),
            SqlFamily::Postgres => ParameterizedValue::DateTime(datetime),
            SqlFamily::Mysql => ParameterizedValue::DateTime(datetime),
        }
    }
}

fn convert_parameterized_date_value(db_value: &ParameterizedValue) -> DateTime<Utc> {
    match db_value {
        ParameterizedValue::Integer(x) => timestamp_to_datetime(*x),
        ParameterizedValue::DateTime(x) => x.clone(),
        x => unimplemented!("Got unsupported value {:?} in date conversion", x),
    }
}

fn timestamp_to_datetime(timestamp: i64) -> DateTime<Utc> {
    let nsecs = ((timestamp % 1000) * 1_000_000) as u32;
    let secs = (timestamp / 1000) as i64;
    let naive = chrono::NaiveDateTime::from_timestamp(secs, nsecs);
    let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);

    datetime
}

fn parse_rows_new(result_set: ResultSet) -> Vec<Migration> {
    result_set
        .into_iter()
        .map(|row| {
            let datamodel_string: String = row[DATAMODEL_COLUMN].to_string().unwrap();
            let datamodel_steps_json: String = row[DATAMODEL_STEPS_COLUMN].to_string().unwrap();

            let database_migration_string: String = row[DATABASE_MIGRATION_COLUMN].to_string().unwrap();
            let errors_json: String = row[ERRORS_COLUMN].to_string().unwrap();

            let finished_at = match &row[FINISHED_AT_COLUMN] {
                ParameterizedValue::Null => None,
                x => Some(convert_parameterized_date_value(x)),
            };

            let datamodel_steps = serde_json::from_str(&datamodel_steps_json).unwrap();
            let datamodel = datamodel::parse_datamodel(&datamodel_string).unwrap();

            let database_migration_json = serde_json::from_str(&database_migration_string).unwrap();
            let errors: Vec<String> = serde_json::from_str(&errors_json).unwrap();

            Migration {
                name: row[NAME_COLUMN].to_string().unwrap(),
                revision: row[REVISION_COLUMN].as_i64().unwrap() as usize,
                datamodel,
                status: MigrationStatus::from_str(row[STATUS_COLUMN].to_string().unwrap()),
                applied: row[APPLIED_COLUMN].as_i64().unwrap() as usize,
                rolled_back: row[ROLLED_BACK_COLUMN].as_i64().unwrap() as usize,
                datamodel_steps,
                database_migration: database_migration_json,
                errors,
                started_at: convert_parameterized_date_value(&row[STARTED_AT_COLUMN]),
                finished_at,
            }
        })
        .collect()
}

static TABLE_NAME: &str = "_Migration";
static NAME_COLUMN: &str = "name";
static REVISION_COLUMN: &str = "revision";
static DATAMODEL_COLUMN: &str = "datamodel";
static STATUS_COLUMN: &str = "status";
static APPLIED_COLUMN: &str = "applied";
static ROLLED_BACK_COLUMN: &str = "rolled_back";
static DATAMODEL_STEPS_COLUMN: &str = "datamodel_steps";
static DATABASE_MIGRATION_COLUMN: &str = "database_migration";
static ERRORS_COLUMN: &str = "errors";
static STARTED_AT_COLUMN: &str = "started_at";
static FINISHED_AT_COLUMN: &str = "finished_at";
