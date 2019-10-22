use clap::ArgMatches;
use failure::Fail;
use itertools::Itertools;
use migration_connector::*;
use sql_migration_connector::{SqlError, SqlMigrationConnector};
use std::collections::HashMap;
use url::Url;

#[derive(Debug, Fail, PartialEq)]
pub enum CliError {
    #[fail(display = "Database '{}' does not exist.", _0)]
    DatabaseDoesNotExist(String),
    #[fail(display = "Access denied to database '{}'", _0)]
    DatabaseAccessDenied(String),
    #[fail(display = "Authentication failed for user '{}'", _0)]
    AuthenticationFailed(String),
    #[fail(display = "Database '{}' already exists", _0)]
    DatabaseAlreadyExists(String),
    #[fail(display = "Error connecting to the database")]
    ConnectionError,
    #[fail(display = "No command defined")]
    NoCommandDefined,
    #[fail(display = "Connect timed out")]
    ConnectTimeout,
    #[fail(display = "Operation timed out")]
    Timeout,
    #[fail(display = "Error opening a TLS connection. {}", _0)]
    TlsError(String),
    #[fail(display = "Unknown error occured: {}", _0)]
    Other(String),
}

impl From<ConnectorError> for CliError {
    fn from(e: ConnectorError) -> Self {
        match e {
            ConnectorError::DatabaseDoesNotExist { db_name } => Self::DatabaseDoesNotExist(db_name),
            ConnectorError::DatabaseAccessDenied { db_name } => Self::DatabaseAccessDenied(db_name),
            ConnectorError::DatabaseAlreadyExists { db_name } => CliError::DatabaseAlreadyExists(db_name),
            ConnectorError::AuthenticationFailed { user } => CliError::AuthenticationFailed(user),
            ConnectorError::ConnectTimeout => CliError::ConnectTimeout,
            ConnectorError::Timeout => CliError::Timeout,
            ConnectorError::TlsError { message } => CliError::TlsError(message),
            _ => CliError::ConnectionError,
        }
    }
}

impl From<crate::Error> for CliError {
    fn from(e: crate::Error) -> Self {
        match e {
            crate::Error::ConnectorError(e) => e.into(),
            e => Self::Other(format!("{}", e)),
        }
    }
}

pub fn run(matches: &ArgMatches, datasource: &str) -> std::result::Result<String, CliError> {
    if matches.is_present("can_connect_to_database") {
        create_conn(datasource, false)?;
        Ok("Connection successful".into())
    } else if matches.is_present("create_database") {
        let (db_name, conn) = create_conn(datasource, true).unwrap();
        futures::executor::block_on(conn.create_database(&db_name))?;
        Ok(format!("Database '{}' created successfully.", db_name))
    } else {
        Err(CliError::NoCommandDefined)
    }
}

fn fetch_db_name(url: &Url, default: &str) -> String {
    let result = match url.path_segments() {
        Some(mut segments) => segments.next().unwrap_or(default),
        None => default,
    };

    String::from(result)
}

fn create_conn(
    datasource: &str,
    admin_mode: bool,
) -> crate::Result<(
    String,
    Box<dyn MigrationConnector<DatabaseMigration = impl DatabaseMigrationMarker>>,
)> {
    let mut url = Url::parse(datasource).expect("Invalid url in the datasource");

    match url.scheme() {
        "file" | "sqlite" => {
            let inner = SqlMigrationConnector::sqlite(datasource)?;

            Ok((String::new(), Box::new(inner)))
        }
        "postgresql" | "postgres" => {
            let db_name = fetch_db_name(&url, "postgres");

            let connector = if admin_mode {
                create_postgres_admin_conn(url)?
            } else {
                SqlMigrationConnector::postgres(url.as_str(), false)?
            };

            Ok((db_name, Box::new(connector)))
        }
        "mysql" => {
            let db_name = fetch_db_name(&url, "mysql");

            if admin_mode {
                url.set_path("");
            }

            let inner = SqlMigrationConnector::mysql(url.as_str(), false)?;
            Ok((db_name, Box::new(inner)))
        }
        x => unimplemented!("Connector {} is not supported yet", x),
    }
}

/// Try to connect as an admin to a postgres database. We try to pick a default database from which
/// we can create another database.
fn create_postgres_admin_conn(mut url: Url) -> crate::Result<SqlMigrationConnector> {
    let candidate_default_databases = &["postgres", "template1"];

    let mut params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    params.remove("schema");
    let params = params.into_iter().map(|(k, v)| format!("{}={}", k, v)).join("&");
    url.set_query(Some(&params));

    let inner = candidate_default_databases
        .iter()
        .filter_map(|database_name| {
            url.set_path(database_name);
            match SqlMigrationConnector::postgres(url.as_str(), false) {
                // If the database does not exist, try the next one.
                Err(SqlError::DatabaseDoesNotExist { .. }) => None,
                // If the outcome is anything else, use this.
                other_outcome => Some(other_outcome),
            }
        })
        .next()
        .ok_or_else(|| {
            ConnectorError::DatabaseCreationFailed {
                explanation: "Prisma could not connect to a default database (`postgres` or `template1`), it cannot create the specified database.".to_owned()
            }
        })??;

    Ok(inner)
}

#[cfg(test)]
mod tests {
    use super::CliError;
    use prisma_query::connector::{MysqlParams, PostgresParams};
    use sql_migration_connector::migration_database::*;
    use std::convert::TryFrom;

    fn with_cli<F>(matches: Vec<&str>, f: F) -> Result<(), Box<dyn std::any::Any + Send + 'static>>
    where
        F: FnOnce(&clap::ArgMatches) -> () + std::panic::UnwindSafe,
    {
        let matches = clap::App::new("cli")
            .arg(
                clap::Arg::with_name("can_connect_to_database")
                    .long("can_connect_to_database")
                    .takes_value(false)
                    .required(false),
            )
            .arg(
                clap::Arg::with_name("create_database")
                    .long("create_database")
                    .help("Create an empty database defined in the configuration string.")
                    .takes_value(false)
                    .required(false),
            )
            .get_matches_from(matches);

        std::panic::catch_unwind(|| f(&matches))
    }

    fn postgres_url(db: Option<&str>) -> String {
        postgres_url_with_scheme(db, "postgresql")
    }

    fn postgres_url_with_scheme(db: Option<&str>, scheme: &str) -> String {
        match std::env::var("IS_BUILDKITE") {
            Ok(_) => format!(
                "{scheme}://postgres:prisma@test-db-postgres:5432/{db_name}",
                scheme = scheme,
                db_name = db.unwrap_or("postgres")
            ),
            _ => format!(
                "{scheme}://postgres:prisma@127.0.0.1:5432/{db_name}?schema=migration-engine",
                scheme = scheme,
                db_name = db.unwrap_or("postgres")
            ),
        }
    }

    fn mysql_url(db: Option<&str>) -> String {
        match std::env::var("IS_BUILDKITE") {
            Ok(_) => format!("mysql://root:prisma@test-db-mysql-5-7:3306/{}", db.unwrap_or("")),
            _ => format!("mysql://root:prisma@127.0.0.1:3306/{}", db.unwrap_or("")),
        }
    }

    #[test]
    fn test_with_missing_command() {
        with_cli(vec!["cli"], |matches| {
            assert_eq!(Err(CliError::NoCommandDefined), super::run(&matches, &mysql_url(None)));
        })
        .unwrap();
    }

    #[test]
    fn test_connecting_with_a_working_mysql_connection_string() {
        with_cli(vec!["cli", "--can_connect_to_database"], |matches| {
            assert_eq!(
                Ok(String::from("Connection successful")),
                super::run(&matches, &mysql_url(None))
            );
        })
        .unwrap();
    }

    #[test]
    fn test_connecting_with_a_non_working_mysql_connection_string() {
        let dm = mysql_url(Some("this_does_not_exist"));

        with_cli(vec!["cli", "--can_connect_to_database"], |matches| {
            assert_eq!(
                Err(CliError::DatabaseDoesNotExist(String::from("this_does_not_exist"))),
                super::run(&matches, &dm)
            );
        })
        .unwrap();
    }

    #[test]
    fn test_connecting_with_a_working_psql_connection_string() {
        with_cli(vec!["cli", "--can_connect_to_database"], |matches| {
            assert_eq!(
                Ok(String::from("Connection successful")),
                super::run(&matches, &postgres_url(None))
            );
        })
        .unwrap();
    }

    #[test]
    fn test_connecting_with_a_working_psql_connection_string_with_postgres_scheme() {
        with_cli(vec!["cli", "--can_connect_to_database"], |matches| {
            assert_eq!(
                Ok(String::from("Connection successful")),
                super::run(&matches, &postgres_url_with_scheme(None, "postgres"))
            );
        })
        .unwrap();
    }

    #[test]
    fn test_connecting_with_a_non_working_psql_connection_string() {
        let dm = postgres_url(Some("this_does_not_exist"));

        with_cli(vec!["cli", "--can_connect_to_database"], |matches| {
            assert_eq!(
                Err(CliError::DatabaseDoesNotExist(String::from("this_does_not_exist"))),
                super::run(&matches, &dm)
            );
        })
        .unwrap();
    }

    #[test]
    fn test_create_mysql_database() {
        let url = mysql_url(Some("this_should_exist"));

        let res = with_cli(vec!["cli", "--create_database"], |matches| {
            assert_eq!(
                Ok(String::from("Database 'this_should_exist' created successfully.")),
                super::run(&matches, &url)
            );
        });

        if let Ok(()) = res {
            let res = with_cli(vec!["cli", "--can_connect_to_database"], |matches| {
                assert_eq!(Ok(String::from("Connection successful")), super::run(&matches, &url));
            });

            {
                let uri = url::Url::parse(&mysql_url(None)).unwrap();
                let conn = Mysql::new(MysqlParams::try_from(uri).unwrap(), false).unwrap();

                conn.execute_raw("", "DROP DATABASE `this_should_exist`", &[]).unwrap();
            }

            res.unwrap();
        } else {
            res.unwrap();
        }
    }

    #[test]
    fn test_create_psql_database() {
        let url = postgres_url(Some("this_should_exist"));

        let res = with_cli(vec!["cli", "--create_database"], |matches| {
            assert_eq!(
                Ok(String::from("Database 'this_should_exist' created successfully.")),
                super::run(&matches, &url)
            );
        });

        if let Ok(()) = res {
            let res = with_cli(vec!["cli", "--can_connect_to_database"], |matches| {
                assert_eq!(
                    Ok(String::from("Connection successful")),
                    super::run(&matches, dbg!(&url))
                );
            });

            {
                let uri = url::Url::parse(&postgres_url(None)).unwrap();
                let conn = PostgreSql::new(PostgresParams::try_from(uri).unwrap(), false).unwrap();

                conn.execute_raw("", "DROP DATABASE \"this_should_exist\"", &[])
                    .unwrap();
            }

            res.unwrap();
        } else {
            res.unwrap();
        }
    }

    #[test]
    fn test_fetch_db_name() {
        let url: url::Url = "postgresql://postgres:prisma@127.0.0.1:5432/pgres?schema=test_schema"
            .parse()
            .unwrap();
        let db_name = super::fetch_db_name(&url, "postgres");
        assert_eq!(db_name, "pgres");
    }

    #[test]
    fn test_fetch_db_name_with_postgres_scheme() {
        let url: url::Url = "postgres://postgres:prisma@127.0.0.1:5432/pgres?schema=test_schema"
            .parse()
            .unwrap();
        let db_name = super::fetch_db_name(&url, "postgres");
        assert_eq!(db_name, "pgres");
    }
}
