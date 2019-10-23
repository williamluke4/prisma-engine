#[macro_use]
extern crate log;

pub mod migration_database;

mod error;
mod sql_database_migration_inferrer;
mod sql_database_step_applier;
mod sql_destructive_changes_checker;
mod sql_migration;
mod sql_migration_persistence;
mod sql_renderer;
mod sql_schema_calculator;
mod sql_schema_differ;

pub use error::*;
pub use sql_migration::*;

use migration_connector::*;
use migration_database::*;
use prisma_query::connector::{MysqlParams, PostgresParams};
use serde_json;
use sql_database_migration_inferrer::*;
use sql_database_step_applier::*;
use sql_destructive_changes_checker::*;
use sql_migration_persistence::*;
use sql_schema_describer::SqlSchemaDescriberBackend;
use std::{convert::TryFrom, fs, path::PathBuf, sync::Arc};
use url::Url;

pub type Result<T> = std::result::Result<T, SqlError>;

#[allow(unused, dead_code)]
pub struct SqlMigrationConnector {
    pub url: String,
    pub file_path: Option<String>,
    pub sql_family: SqlFamily,
    pub schema_name: String,
    pub database: Arc<dyn MigrationDatabase + Send + Sync + 'static>,
    pub migration_persistence: Arc<dyn MigrationPersistence>,
    pub database_migration_inferrer: Arc<dyn DatabaseMigrationInferrer<SqlMigration>>,
    pub database_migration_step_applier: Arc<dyn DatabaseMigrationStepApplier<SqlMigration>>,
    pub destructive_changes_checker: Arc<dyn DestructiveChangesChecker<SqlMigration>>,
    pub database_introspector: Arc<dyn SqlSchemaDescriberBackend + Send + Sync + 'static>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SqlFamily {
    Sqlite,
    Postgres,
    Mysql,
}

impl SqlFamily {
    fn connector_type_string(&self) -> &'static str {
        match self {
            SqlFamily::Postgres => "postgresql",
            SqlFamily::Mysql => "mysql",
            SqlFamily::Sqlite => "sqlite",
        }
    }
}

impl SqlMigrationConnector {
    pub fn postgres(url_str: &str, pooled: bool) -> crate::Result<Self> {
        let url = Url::parse(url_str)?;
        let params = PostgresParams::try_from(url.clone())?;

        let schema = params.schema.clone();
        let conn = PostgreSql::new(params, pooled)?;

        Ok(Self::create_connector(
            url_str,
            Arc::new(conn),
            SqlFamily::Postgres,
            schema,
            None,
        ))
    }

    pub fn mysql(url_str: &str, pooled: bool) -> crate::Result<Self> {
        let url = Url::parse(url_str)?;

        let schema = {
            let params = MysqlParams::try_from(url.clone())?;
            params.dbname.clone()
        };

        let params = MysqlParams::try_from(url)?;
        let conn = Mysql::new(params, pooled)?;

        Ok(Self::create_connector(
            url_str,
            Arc::new(conn),
            SqlFamily::Mysql,
            schema,
            None,
        ))
    }

    pub fn sqlite(url: &str) -> crate::Result<Self> {
        let conn = Sqlite::new(url)?;
        let file_path = conn.file_path.clone();
        let schema = String::from("lift");

        Ok(Self::create_connector(
            url,
            Arc::new(conn),
            SqlFamily::Sqlite,
            schema,
            Some(file_path),
        ))
    }

    fn create_connector(
        url: &str,
        conn: Arc<dyn MigrationDatabase + Send + Sync + 'static>,
        sql_family: SqlFamily,
        schema_name: String,
        file_path: Option<String>,
    ) -> Self {
        let introspection_connection = Arc::new(MigrationDatabaseWrapper {
            database: Arc::clone(&conn),
        });
        let inspector: Arc<dyn SqlSchemaDescriberBackend + Send + Sync + 'static> = match sql_family {
            SqlFamily::Sqlite => Arc::new(sql_schema_describer::sqlite::SqlSchemaDescriber::new(
                introspection_connection,
            )),
            SqlFamily::Postgres => Arc::new(sql_schema_describer::postgres::SqlSchemaDescriber::new(
                introspection_connection,
            )),
            SqlFamily::Mysql => Arc::new(sql_schema_describer::mysql::SqlSchemaDescriber::new(
                introspection_connection,
            )),
        };

        let migration_persistence = Arc::new(SqlMigrationPersistence {
            sql_family,
            connection: Arc::clone(&conn),
            schema_name: schema_name.clone(),
            file_path: file_path.clone(),
        });

        let database_migration_inferrer = Arc::new(SqlDatabaseMigrationInferrer {
            sql_family,
            introspector: Arc::clone(&inspector),
            schema_name: schema_name.to_string(),
        });

        let database_migration_step_applier = Arc::new(SqlDatabaseStepApplier {
            sql_family,
            schema_name: schema_name.clone(),
            conn: Arc::clone(&conn),
        });

        let destructive_changes_checker = Arc::new(SqlDestructiveChangesChecker {
            schema_name: schema_name.clone(),
            database: Arc::clone(&conn),
        });

        Self {
            url: url.to_string(),
            file_path,
            sql_family,
            schema_name,
            database: Arc::clone(&conn),
            migration_persistence,
            database_migration_inferrer,
            database_migration_step_applier,
            destructive_changes_checker,
            database_introspector: Arc::clone(&inspector),
        }
    }
}

#[async_trait::async_trait]
impl MigrationConnector for SqlMigrationConnector {
    type DatabaseMigration = SqlMigration;

    fn connector_type(&self) -> &'static str {
        self.sql_family.connector_type_string()
    }

    async fn create_database(&self, db_name: &str) -> ConnectorResult<()> {
        match self.sql_family {
            SqlFamily::Postgres => {
                self.database
                    .query_raw("", &format!("CREATE DATABASE \"{}\"", db_name), &[])?;

                Ok(())
            }
            SqlFamily::Sqlite => Ok(()),
            SqlFamily::Mysql => {
                self.database
                    .query_raw("", &format!("CREATE DATABASE `{}`", db_name), &[])?;

                Ok(())
            }
        }
    }

    async fn initialize(&self) -> ConnectorResult<()> {
        // TODO: this code probably does not ever do anything. The schema/db creation happens already in the helper functions above.
        match self.sql_family {
            SqlFamily::Sqlite => {
                if let Some(file_path) = &self.file_path {
                    let path_buf = PathBuf::from(&file_path);
                    match path_buf.parent() {
                        Some(parent_directory) => {
                            fs::create_dir_all(parent_directory).expect("creating the database folders failed")
                        }
                        None => {}
                    }
                }
            }
            SqlFamily::Postgres => {
                let schema_sql = format!("CREATE SCHEMA IF NOT EXISTS \"{}\";", &self.schema_name);

                debug!("{}", schema_sql);

                self.database.query_raw("", &schema_sql, &[])?;
            }
            SqlFamily::Mysql => {
                let schema_sql = format!(
                    "CREATE SCHEMA IF NOT EXISTS `{}` DEFAULT CHARACTER SET latin1;",
                    &self.schema_name
                );

                debug!("{}", schema_sql);

                self.database.query_raw("", &schema_sql, &[])?;
            }
        }

        self.migration_persistence.init().await;

        Ok(())
    }

    async fn reset(&self) -> ConnectorResult<()> {
        self.migration_persistence.reset().await;
        Ok(())
    }

    fn migration_persistence(&self) -> Arc<dyn MigrationPersistence> {
        Arc::clone(&self.migration_persistence)
    }

    fn database_migration_inferrer(&self) -> Arc<dyn DatabaseMigrationInferrer<SqlMigration>> {
        Arc::clone(&self.database_migration_inferrer)
    }

    fn database_migration_step_applier(&self) -> Arc<dyn DatabaseMigrationStepApplier<SqlMigration>> {
        Arc::clone(&self.database_migration_step_applier)
    }

    fn destructive_changes_checker(&self) -> Arc<dyn DestructiveChangesChecker<SqlMigration>> {
        Arc::clone(&self.destructive_changes_checker)
    }

    fn deserialize_database_migration(&self, json: serde_json::Value) -> SqlMigration {
        serde_json::from_value(json).expect("Deserializing the database migration failed.")
    }
}
