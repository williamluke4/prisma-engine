use crate::commands::command::*;
use crate::migration_engine::MigrationEngine;
use datamodel::dml::Datamodel;
use log::*;
use migration_connector::*;
use serde::{Deserialize, Serialize};

pub struct CalculateDatamodelCommand {
    input: CalculateDatamodelInput,
}

#[async_trait::async_trait]
impl MigrationCommand for CalculateDatamodelCommand {
    type Input = CalculateDatamodelInput;
    type Output = CalculateDatamodelOutput;

    fn new(input: Self::Input) -> Box<Self> {
        Box::new(CalculateDatamodelCommand { input })
    }

    async fn execute<C, D>(&self, engine: &MigrationEngine<C, D>) -> CommandResult<Self::Output>
    where
        C: MigrationConnector<DatabaseMigration = D>,
        D: DatabaseMigrationMarker + 'static,
    {
        debug!("{:?}", self.input);

        let base_datamodel = Datamodel::empty();
        let datamodel = engine.datamodel_calculator().infer(&base_datamodel, &self.input.steps);

        Ok(CalculateDatamodelOutput {
            datamodel: datamodel::render_datamodel_to_string(&datamodel).unwrap(),
        })
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CalculateDatamodelInput {
    pub steps: Vec<MigrationStep>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalculateDatamodelOutput {
    pub datamodel: String,
}
