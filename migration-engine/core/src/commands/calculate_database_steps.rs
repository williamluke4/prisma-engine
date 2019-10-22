use super::MigrationStepsResultOutput;
use crate::commands::command::*;
use crate::migration_engine::MigrationEngine;
use datamodel::Datamodel;
use log::*;
use migration_connector::*;
use serde::Deserialize;

pub struct CalculateDatabaseStepsCommand {
    input: CalculateDatabaseStepsInput,
}

#[async_trait::async_trait]
impl MigrationCommand for CalculateDatabaseStepsCommand {
    type Input = CalculateDatabaseStepsInput;
    type Output = MigrationStepsResultOutput;

    fn new(input: Self::Input) -> Box<Self> {
        Box::new(CalculateDatabaseStepsCommand { input })
    }

    async fn execute<C, D>(&self, engine: &MigrationEngine<C, D>) -> CommandResult<Self::Output>
    where
        C: MigrationConnector<DatabaseMigration = D>,
        D: DatabaseMigrationMarker + Send + Sync + 'static,
    {
        debug!("{:?}", self.input);

        let connector = engine.connector();

        let assumed_datamodel = engine
            .datamodel_calculator()
            .infer(&Datamodel::empty(), &self.input.assume_to_be_applied);

        let next_datamodel = engine
            .datamodel_calculator()
            .infer(&assumed_datamodel, &self.input.steps_to_apply);

        let database_migration = connector.database_migration_inferrer().infer(
            &assumed_datamodel,
            &next_datamodel,
            &self.input.steps_to_apply,
        )?;

        let DestructiveChangeDiagnostics { warnings, errors: _ } =
            connector.destructive_changes_checker().check(&database_migration)?;

        let database_steps_json = connector
            .database_migration_step_applier()
            .render_steps_pretty(&database_migration)?;

        Ok(MigrationStepsResultOutput {
            datamodel: datamodel::render_datamodel_to_string(&next_datamodel).unwrap(),
            datamodel_steps: self.input.steps_to_apply.clone(),
            database_steps: serde_json::Value::Array(database_steps_json),
            errors: Vec::new(),
            warnings,
            general_errors: Vec::new(),
        })
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CalculateDatabaseStepsInput {
    pub assume_to_be_applied: Vec<MigrationStep>,
    pub steps_to_apply: Vec<MigrationStep>,
}
