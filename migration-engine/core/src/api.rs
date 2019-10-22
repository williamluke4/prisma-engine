mod rpc;

pub use rpc::*;

use crate::{commands::*, migration_engine::MigrationEngine};
use migration_connector::*;
use std::sync::Arc;

pub struct MigrationApi<C, D>
where
    C: MigrationConnector<DatabaseMigration = D>,
    D: DatabaseMigrationMarker + 'static,
{
    engine: MigrationEngine<C, D>,
}

impl<C, D> MigrationApi<C, D>
where
    C: MigrationConnector<DatabaseMigration = D>,
    D: DatabaseMigrationMarker + Send + Sync + 'static,
{
    pub fn new(connector: C) -> crate::Result<Self> {
        let engine = MigrationEngine::new(connector)?;

        Ok(Self { engine })
    }

    pub fn handle_command<E>(&self, input: E::Input) -> crate::Result<E::Output>
    where
        E: MigrationCommand,
    {
        let cmd = E::new(input);
        let result = futures::executor::block_on(cmd.execute(&self.engine))?;
        Ok(result)
    }
}

// This is here only to get rid of the generic type parameters due to neon not
// liking them in the exported class.
pub trait GenericApi: Send + Sync + 'static {
    fn apply_migration(&self, input: ApplyMigrationInput) -> crate::Result<MigrationStepsResultOutput>;
    fn calculate_database_steps(&self, input: CalculateDatabaseStepsInput)
        -> crate::Result<MigrationStepsResultOutput>;
    fn calculate_datamodel(&self, input: CalculateDatamodelInput) -> crate::Result<CalculateDatamodelOutput>;
    fn infer_migration_steps(&self, input: InferMigrationStepsInput) -> crate::Result<MigrationStepsResultOutput>;
    fn list_migrations(&self, input: serde_json::Value) -> crate::Result<Vec<ListMigrationStepsOutput>>;
    fn migration_progress(&self, input: MigrationProgressInput) -> crate::Result<MigrationProgressOutput>;
    fn reset(&self, input: serde_json::Value) -> crate::Result<serde_json::Value>;
    fn unapply_migration(&self, input: UnapplyMigrationInput) -> crate::Result<UnapplyMigrationOutput>;
    fn migration_persistence(&self) -> Arc<dyn MigrationPersistence>;
    fn connector_type(&self) -> &'static str;
}

impl<C, D> GenericApi for MigrationApi<C, D>
where
    C: MigrationConnector<DatabaseMigration = D>,
    D: DatabaseMigrationMarker + Send + Sync + 'static,
{
    fn apply_migration(&self, input: ApplyMigrationInput) -> crate::Result<MigrationStepsResultOutput> {
        self.handle_command::<ApplyMigrationCommand>(input)
    }

    fn calculate_database_steps(
        &self,
        input: CalculateDatabaseStepsInput,
    ) -> crate::Result<MigrationStepsResultOutput> {
        self.handle_command::<CalculateDatabaseStepsCommand>(input)
    }

    fn calculate_datamodel(&self, input: CalculateDatamodelInput) -> crate::Result<CalculateDatamodelOutput> {
        self.handle_command::<CalculateDatamodelCommand>(input)
    }

    fn infer_migration_steps(&self, input: InferMigrationStepsInput) -> crate::Result<MigrationStepsResultOutput> {
        self.handle_command::<InferMigrationStepsCommand>(input)
    }

    fn list_migrations(&self, input: serde_json::Value) -> crate::Result<Vec<ListMigrationStepsOutput>> {
        self.handle_command::<ListMigrationStepsCommand>(input)
    }

    fn migration_progress(&self, input: MigrationProgressInput) -> crate::Result<MigrationProgressOutput> {
        self.handle_command::<MigrationProgressCommand>(input)
    }

    fn reset(&self, input: serde_json::Value) -> crate::Result<serde_json::Value> {
        self.handle_command::<ResetCommand>(input)
    }

    fn unapply_migration(&self, input: UnapplyMigrationInput) -> crate::Result<UnapplyMigrationOutput> {
        self.handle_command::<UnapplyMigrationCommand>(input)
    }

    fn migration_persistence(&self) -> Arc<dyn MigrationPersistence> {
        self.engine.connector().migration_persistence()
    }

    fn connector_type(&self) -> &'static str {
        self.engine.connector().connector_type()
    }
}
