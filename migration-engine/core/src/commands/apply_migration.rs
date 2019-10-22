use super::MigrationStepsResultOutput;
use crate::commands::command::*;
use crate::migration_engine::MigrationEngine;
use datamodel::Datamodel;
use log::*;
use migration_connector::*;
use serde::Deserialize;

pub struct ApplyMigrationCommand {
    input: ApplyMigrationInput,
}

#[async_trait::async_trait]
impl MigrationCommand for ApplyMigrationCommand {
    type Input = ApplyMigrationInput;
    type Output = MigrationStepsResultOutput;

    fn new(input: Self::Input) -> Box<Self> {
        Box::new(ApplyMigrationCommand { input })
    }

    async fn execute<C, D>(&self, engine: &MigrationEngine<C, D>) -> CommandResult<Self::Output>
    where
        C: MigrationConnector<DatabaseMigration = D>,
        D: DatabaseMigrationMarker + Send + Sync + 'static,
    {
        debug!("{:?}", self.input);

        let connector = engine.connector();
        let migration_persistence = connector.migration_persistence();

        match migration_persistence.last() {
            Some(ref last_migration) if last_migration.is_watch_migration() && !self.input.is_watch_migration() => {
                self.handle_transition_out_of_watch_mode(&engine)
            }
            _ => self.handle_normal_migration(&engine),
        }
    }
}

impl ApplyMigrationCommand {
    fn handle_transition_out_of_watch_mode<C, D>(
        &self,
        engine: &MigrationEngine<C, D>,
    ) -> CommandResult<MigrationStepsResultOutput>
    where
        C: MigrationConnector<DatabaseMigration = D>,
        D: DatabaseMigrationMarker + Send + Sync + 'static,
    {
        let connector = engine.connector();
        let migration_persistence = connector.migration_persistence();

        let current_datamodel = migration_persistence.current_datamodel();
        let last_non_watch_datamodel = migration_persistence.last_non_watch_datamodel();
        let next_datamodel = engine
            .datamodel_calculator()
            .infer(&last_non_watch_datamodel, &self.input.steps);

        self.handle_migration(&engine, current_datamodel, next_datamodel)
    }

    fn handle_normal_migration<C, D>(&self, engine: &MigrationEngine<C, D>) -> CommandResult<MigrationStepsResultOutput>
    where
        C: MigrationConnector<DatabaseMigration = D>,
        D: DatabaseMigrationMarker + Send + Sync + 'static,
    {
        let connector = engine.connector();
        let migration_persistence = connector.migration_persistence();
        let current_datamodel = migration_persistence.current_datamodel();

        let next_datamodel = engine
            .datamodel_calculator()
            .infer(&current_datamodel, &self.input.steps);

        self.handle_migration(&engine, current_datamodel, next_datamodel)
    }

    fn handle_migration<C, D>(
        &self,
        engine: &MigrationEngine<C, D>,
        current_datamodel: Datamodel,
        next_datamodel: Datamodel,
    ) -> CommandResult<MigrationStepsResultOutput>
    where
        C: MigrationConnector<DatabaseMigration = D>,
        D: DatabaseMigrationMarker + Send + Sync + 'static,
    {
        let connector = engine.connector();
        let migration_persistence = connector.migration_persistence();

        let database_migration =
            connector
                .database_migration_inferrer()
                .infer(&current_datamodel, &next_datamodel, &self.input.steps)?; // TODO: those steps are a lie right now. Does not matter because we don't use them at the moment.

        let database_steps_json_pretty = connector
            .database_migration_step_applier()
            .render_steps_pretty(&database_migration)?;

        let database_migration_json = database_migration.serialize();

        let mut migration = Migration::new(self.input.migration_id.clone());
        migration.datamodel_steps = self.input.steps.clone();
        migration.database_migration = database_migration_json;
        migration.datamodel = next_datamodel.clone();

        let diagnostics = connector.destructive_changes_checker().check(&database_migration)?;

        match (diagnostics.has_warnings(), self.input.force.unwrap_or(false)) {
            // We have no warnings, or the force flag is passed.
            (false, _) | (true, true) => {
                let saved_migration = migration_persistence.create(migration);

                connector
                    .migration_applier()
                    .apply(&saved_migration, &database_migration)?;
            }
            // We have warnings, but no force flag was passed.
            (true, false) => (),
        }

        let DestructiveChangeDiagnostics { warnings, errors } = diagnostics;

        Ok(MigrationStepsResultOutput {
            datamodel: datamodel::render_datamodel_to_string(&next_datamodel).unwrap(),
            datamodel_steps: self.input.steps.clone(),
            database_steps: serde_json::Value::Array(database_steps_json_pretty),
            errors,
            warnings,
            general_errors: Vec::new(),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyMigrationInput {
    pub migration_id: String,
    pub steps: Vec<MigrationStep>,
    pub force: Option<bool>,
}

impl IsWatchMigration for ApplyMigrationInput {
    fn is_watch_migration(&self) -> bool {
        self.migration_id.starts_with("watch")
    }
}
