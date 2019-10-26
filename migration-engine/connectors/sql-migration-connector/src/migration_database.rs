use async_trait::async_trait;
use prisma_query::{
    ast::*,
    connector::{self, MysqlParams, PostgresParams, Queryable, ResultSet, SqliteParams},
    // pool::{mysql::*, postgres::*, sqlite::*, PrismaConnectionManager},
};
use std::{
    convert::TryFrom,
    ops::DerefMut,
    sync::{Arc, Mutex},
    time::Duration,
};

#[async_trait]
pub trait MigrationDatabase: Send + Sync + 'static {
    async fn execute<'a>(&'a self, db: &str, q: Query<'a>) -> prisma_query::Result<Option<Id>>;
    async fn query<'a>(&'a self, db: &str, q: Query<'a>) -> prisma_query::Result<ResultSet>;
    async fn query_raw<'a>(&'a self, db: &str, sql: &str, params: &[ParameterizedValue<'a>]) -> prisma_query::Result<ResultSet>;
    async fn execute_raw<'a>(&'a self, db: &str, sql: &str, params: &[ParameterizedValue<'a>]) -> prisma_query::Result<u64>;
}

pub struct MigrationDatabaseWrapper {
    pub database: Arc<dyn MigrationDatabase + Send + Sync + 'static>,
}

impl sql_schema_describer::SqlConnection for MigrationDatabaseWrapper {
    fn query_raw(
        &self,
        sql: &str,
        schema: &str,
        params: &[ParameterizedValue],
    ) -> prisma_query::Result<prisma_query::connector::ResultSet> {
        unimplemented!()
        // self.database.query_raw(schema, sql, params)
    }
}

type SqlitePool = r2d2::Pool<PrismaConnectionManager<SqliteConnectionManager>>;
type PostgresPool = r2d2::Pool<PrismaConnectionManager<PostgresManager>>;
type MysqlPool = r2d2::Pool<PrismaConnectionManager<MysqlConnectionManager>>;

pub struct Sqlite {
    pool: SqlitePool,
    pub(crate) file_path: String,
}

impl Sqlite {
    pub fn new(url: &str) -> prisma_query::Result<Self> {
        unimplemented!()
        // let params = SqliteParams::try_from(url)?;
        // let file_path = params.file_path.to_str().unwrap().to_string();
        // let manager = PrismaConnectionManager::sqlite(None, &file_path)?;

        // let pool = r2d2::Pool::builder()
        //     .max_size(2)
        //     .test_on_check_out(false)
        //     .build(manager)?;

        // Ok(Self { pool, file_path })
    }

    fn with_connection<F, T>(&self, db: &str, f: F) -> T
    where
        F: FnOnce(&mut dyn Queryable) -> T,
    {
        let mut conn = self.pool.get().unwrap();

        conn.execute_raw(
            "ATTACH DATABASE ? AS ?",
            &[
                ParameterizedValue::from(self.file_path.as_str()),
                ParameterizedValue::from(db),
            ],
        )
        .unwrap();

        let res = f(conn.deref_mut());

        conn.execute_raw("DETACH DATABASE ?", &[ParameterizedValue::from(db)])
            .unwrap();

        res
    }
}

#[async_trait]
impl MigrationDatabase for Sqlite {
    async fn execute<'a>(&'a self, db: &str, q: Query<'a>) -> prisma_query::Result<Option<Id>> {
        // self.with_connection(db, |conn| conn.execute(q))
        unimplemented!()
    }

    async fn query<'a>(&'a self, db: &str, q: Query<'a>) -> prisma_query::Result<ResultSet> {
        // self.with_connection(db, |conn| conn.query(q))
        unimplemented!()
    }

    async fn query_raw<'a>(&'a self, db: &str, sql: &str, params: &[ParameterizedValue<'a>]) -> prisma_query::Result<ResultSet> {
        // self.with_connection(db, |conn| conn.query_raw(sql, params))
        unimplemented!()
    }

    async fn execute_raw<'a>(&'a self, db: &str, sql: &str, params: &[ParameterizedValue<'a>]) -> prisma_query::Result<u64> {
        // self.with_connection(db, |conn| conn.execute_raw(sql, params))
        unimplemented!()
    }
}

enum PostgresConnection {
    Pooled(PostgresPool),
    Single(Mutex<connector::PostgreSql>),
}

pub struct PostgreSql {
    conn: PostgresConnection,
}

impl PostgreSql {
    pub fn new(params: PostgresParams, pooled: bool) -> prisma_query::Result<Self> {
        unimplemented!()
        // let conn = if pooled {
        //     let manager = PrismaConnectionManager::postgres(params.config, Some(params.schema))?;

        //     let pool = r2d2::Pool::builder()
        //         .max_size(2)
        //         .connection_timeout(Duration::from_millis(1500))
        //         .test_on_check_out(false)
        //         .build(manager)?;

        //     PostgresConnection::Pooled(pool)
        // } else {
        //     let conn = connector::PostgreSql::from_params(params).await?;
        //     PostgresConnection::Single(Mutex::new(conn))
        // };

        // Ok(Self { conn })
    }

    fn with_connection<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&mut dyn Queryable) -> T,
    {
        match self.conn {
            PostgresConnection::Single(ref mutex) => f(mutex.lock().unwrap().deref_mut()),
            PostgresConnection::Pooled(ref pool) => {
                let mut conn = pool.get().unwrap();
                f(conn.deref_mut())
            }
        }
    }
}

#[async_trait]
impl MigrationDatabase for PostgreSql {
    async fn execute<'a>(&'a self, _: &str, q: Query<'a>) -> prisma_query::Result<Option<Id>> {
        // self.with_connection(|conn| conn.execute(q))
        unimplemented!()
    }

    async fn query<'a>(&'a self, _: &str, q: Query<'a>) -> prisma_query::Result<ResultSet> {
        // self.with_connection(|conn| conn.query(q))
        unimplemented!()
    }

    async fn query_raw<'a>(&'a self, _: &str, sql: &str, params: &[ParameterizedValue<'a>]) -> prisma_query::Result<ResultSet> {
        // self.with_connection(|conn| conn.query_raw(sql, params))
        unimplemented!()
    }

    async fn execute_raw<'a>(&'a self, _: &str, sql: &str, params: &[ParameterizedValue<'a>]) -> prisma_query::Result<u64> {
        // self.with_connection(|conn| conn.execute_raw(sql, params))
        unimplemented!()
    }
}

enum MysqlConnection {
    Pooled(MysqlPool),
    Single(Mutex<connector::Mysql>),
}

pub struct Mysql {
    conn: MysqlConnection,
}

impl Mysql {
    pub fn new(params: MysqlParams, pooled: bool) -> prisma_query::Result<Self> {
        unimplemented!()
        // let conn = if pooled {
        //     let manager = PrismaConnectionManager::mysql(params.config);

        //     let pool = r2d2::Pool::builder()
        //         .connection_timeout(Duration::from_millis(1500))
        //         .max_size(2)
        //         .test_on_check_out(false)
        //         .build(manager)?;

        //     MysqlConnection::Pooled(pool)
        // } else {
        //     let conn = connector::Mysql::from_params(params)?;
        //     MysqlConnection::Single(Mutex::new(conn))
        // };

        // Ok(Self { conn })
    }

    fn with_connection<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&mut dyn Queryable) -> T,
    {
        match self.conn {
            MysqlConnection::Single(ref mutex) => f(mutex.lock().unwrap().deref_mut()),
            MysqlConnection::Pooled(ref pool) => {
                let mut conn = pool.get().unwrap();
                f(conn.deref_mut())
            }
        }
    }
}

#[async_trait]
impl MigrationDatabase for Mysql {
    async fn execute<'a>(&'a self, _: &str, q: Query<'a>) -> prisma_query::Result<Option<Id>> {
        // self.with_connection(|conn| conn.execute(q))
        unimplemented!()
    }

    async fn query<'a>(&'a self, _: &str, q: Query<'a>) -> prisma_query::Result<ResultSet> {
        // self.with_connection(|conn| conn.query(q))
        unimplemented!()
    }

    async fn query_raw<'a>(&'a self, _: &str, sql: &str, params: &[ParameterizedValue<'a>]) -> prisma_query::Result<ResultSet> {
        // self.with_connection(|conn| conn.query_raw(sql, params))
        unimplemented!()
    }

    async fn execute_raw<'a>(&'a self, _: &str, sql: &str, params: &[ParameterizedValue<'a>]) -> prisma_query::Result<u64> {
        // self.with_connection(|conn| conn.execute_raw(sql, params))
        unimplemented!()
    }
}
