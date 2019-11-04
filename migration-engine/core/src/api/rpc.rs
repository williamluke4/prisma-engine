use super::{GenericApi, MigrationApi};
use crate::commands::*;
use futures::{
    future::{err, lazy, ok, poll_fn},
    Future,
};
use jsonrpc_core::types::error::Error as JsonRpcError;
use jsonrpc_core::IoHandler;
use jsonrpc_core::*;
use jsonrpc_stdio_server::ServerBuilder;
use sql_migration_connector::SqlMigrationConnector;
use std::{io, sync::Arc};
use tokio_threadpool::blocking;

pub struct RpcApi {
    io_handler: jsonrpc_core::IoHandler<()>,
    executor: Arc<dyn GenericApi>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum RpcCommand {
    InferMigrationSteps,
    ListMigrations,
    MigrationProgress,
    ApplyMigration,
    UnapplyMigration,
    Reset,
    CalculateDatamodel,
    CalculateDatabaseSteps,
}

impl RpcCommand {
    fn name(&self) -> &'static str {
        match self {
            RpcCommand::InferMigrationSteps => "inferMigrationSteps",
            RpcCommand::ListMigrations => "listMigrations",
            RpcCommand::MigrationProgress => "migrationProgress",
            RpcCommand::ApplyMigration => "applyMigration",
            RpcCommand::UnapplyMigration => "unapplyMigration",
            RpcCommand::Reset => "reset",
            RpcCommand::CalculateDatamodel => "calculateDatamodel",
            RpcCommand::CalculateDatabaseSteps => "calculateDatabaseSteps",
        }
    }
}

static AVAILABLE_COMMANDS: &[RpcCommand] = &[
    RpcCommand::ApplyMigration,
    RpcCommand::InferMigrationSteps,
    RpcCommand::ListMigrations,
    RpcCommand::MigrationProgress,
    RpcCommand::UnapplyMigration,
    RpcCommand::Reset,
    RpcCommand::CalculateDatamodel,
    RpcCommand::CalculateDatabaseSteps,
];

impl RpcApi {
    pub fn new_async(datamodel: &str) -> crate::Result<Self> {
        let mut rpc_api = Self::new(datamodel)?;

        for cmd in AVAILABLE_COMMANDS {
            rpc_api.add_async_command_handler(*cmd);
        }

        Ok(rpc_api)
    }

    pub fn new_sync(datamodel: &str) -> crate::Result<Self> {
        let mut rpc_api = Self::new(datamodel)?;

        for cmd in AVAILABLE_COMMANDS {
            rpc_api.add_sync_command_handler(*cmd);
        }

        Ok(rpc_api)
    }

    /// Block the thread and handle IO in async until EOF.
    pub fn start_server(self) {
        ServerBuilder::new(self.io_handler).build()
    }

    /// Handle one request
    pub fn handle(&self) -> crate::Result<String> {
        let mut json_is_complete = false;
        let mut input = String::new();

        while !json_is_complete {
            io::stdin().read_line(&mut input)?;
            json_is_complete = serde_json::from_str::<serde_json::Value>(&input).is_ok();
        }

        let result = self
            .io_handler
            .handle_request_sync(&input)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Reading from stdin failed."))?;

        Ok(result)
    }

    fn new(datamodel: &str) -> crate::Result<RpcApi> {
        let config = datamodel::parse_configuration(datamodel)?;

        let source = config.datasources.first().ok_or(CommandError::DataModelErrors {
            code: 1000,
            errors: vec!["There is no datasource in the configuration.".to_string()],
        })?;

        let connector = match source.connector_type() {
            "sqlite" => SqlMigrationConnector::sqlite(&source.url().value)?,
            "postgresql" => SqlMigrationConnector::postgres(&source.url().value, true)?,
            "mysql" => SqlMigrationConnector::mysql(&source.url().value, true)?,
            x => unimplemented!("Connector {} is not supported yet", x),
        };

        Ok(Self {
            io_handler: IoHandler::default(),
            executor: Arc::new(MigrationApi::new(connector)?),
        })
    }

    fn add_sync_command_handler(&mut self, cmd: RpcCommand) {
        let executor = Arc::clone(&self.executor);

        self.io_handler.add_method(cmd.name(), move |params: Params| {
            Self::create_sync_handler(&executor, cmd, &params)
        });
    }

    fn add_async_command_handler(&mut self, cmd: RpcCommand) {
        let executor = Arc::clone(&self.executor);

        self.io_handler.add_method(cmd.name(), move |params: Params| {
            Self::create_async_handler(&executor, cmd, params)
        });
    }

    fn create_sync_handler(
        executor: &Arc<dyn GenericApi>,
        cmd: RpcCommand,
        params: &Params,
    ) -> std::result::Result<serde_json::Value, JsonRpcError> {
        let result: crate::Result<serde_json::Value> = Self::run_command(executor, cmd, params);

        result.map_err(|err| self.render_error(err))
    }

    fn create_async_handler(
        executor: &Arc<dyn GenericApi>,
        cmd: RpcCommand,
        params: Params,
    ) -> impl Future<Item = serde_json::Value, Error = JsonRpcError> {
        let executor = Arc::clone(executor);

        lazy(move || poll_fn(move || blocking(|| Self::create_sync_handler(&executor, cmd, &params)))).then(|res| {
            match res {
                // dumdidum futures 0.1 we love <3
                Ok(Ok(val)) => ok(val),
                Ok(Err(val)) => err(val),
                Err(val) => {
                    let e = crate::error::Error::from(val);
                    err(JsonRpcError::from(e))
                }
            }
        })
    }

    fn run_command(
        executor: &Arc<dyn GenericApi>,
        cmd: RpcCommand,
        params: &Params,
    ) -> crate::Result<serde_json::Value> {
        match cmd {
            RpcCommand::InferMigrationSteps => {
                let input: InferMigrationStepsInput = params.parse()?;
                render(executor.infer_migration_steps(&input)?)
            }
            RpcCommand::ListMigrations => render(executor.list_migrations(&serde_json::Value::Null)?),
            RpcCommand::MigrationProgress => {
                let input: MigrationProgressInput = params.parse()?;
                render(executor.migration_progress(&input)?)
            }
            RpcCommand::ApplyMigration => {
                let input: ApplyMigrationInput = params.parse()?;
                render(executor.apply_migration(&input)?)
            }
            RpcCommand::UnapplyMigration => {
                let input: UnapplyMigrationInput = params.parse()?;
                render(executor.unapply_migration(&input)?)
            }
            RpcCommand::Reset => render(executor.reset(&serde_json::Value::Null)?),
            RpcCommand::CalculateDatamodel => {
                let input: CalculateDatamodelInput = params.parse()?;
                render(executor.calculate_datamodel(&input)?)
            }
            RpcCommand::CalculateDatabaseSteps => {
                let input: CalculateDatabaseStepsInput = params.parse()?;
                render(executor.calculate_database_steps(&input)?)
            }
        }
    }
}

fn render(result: impl serde::Serialize) -> crate::Result<serde_json::Value> {
    Ok(serde_json::to_value(result).expect("Rendering of RPC response failed"))
}
