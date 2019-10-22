use crate::commands::CommandResult;
use crate::migration::datamodel_calculator::*;
use crate::migration::datamodel_migration_steps_inferrer::*;
use datamodel::dml::*;
use migration_connector::*;
use std::sync::Arc;

pub struct MigrationEngine<C, D>
where
    C: MigrationConnector<DatabaseMigration = D>,
    D: DatabaseMigrationMarker + 'static,
{
    datamodel_migration_steps_inferrer: Arc<dyn DataModelMigrationStepsInferrer>,
    datamodel_calculator: Arc<dyn DataModelCalculator>,
    connector: C,
}

impl<C, D> MigrationEngine<C, D>
where
    C: MigrationConnector<DatabaseMigration = D>,
    D: DatabaseMigrationMarker + 'static,
{
    pub fn new(connector: C) -> crate::Result<Self> {
        let engine = MigrationEngine {
            datamodel_migration_steps_inferrer: Arc::new(DataModelMigrationStepsInferrerImplWrapper {}),
            datamodel_calculator: Arc::new(DataModelCalculatorImpl {}),
            connector,
        };

        futures::executor::block_on(engine.init())?;

        Ok(engine)
    }

    pub async fn init(&self) -> CommandResult<()> {
        self.connector().initialize().await?;
        Ok(())
    }

    pub async fn reset(&self) -> CommandResult<()> {
        self.connector().reset().await?;
        Ok(())
    }

    pub fn connector(&self) -> &C {
        &self.connector
    }

    pub fn datamodel_migration_steps_inferrer(&self) -> &Arc<dyn DataModelMigrationStepsInferrer> {
        &self.datamodel_migration_steps_inferrer
    }

    pub fn datamodel_calculator(&self) -> &Arc<dyn DataModelCalculator> {
        &self.datamodel_calculator
    }

    pub fn render_datamodel(&self, datamodel: &Datamodel) -> String {
        datamodel::render_datamodel_to_string(&datamodel).expect("Rendering the Datamodel failed.")
    }
}
