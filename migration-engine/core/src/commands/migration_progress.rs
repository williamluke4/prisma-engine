use crate::commands::command::*;
use crate::migration_engine::MigrationEngine;
use chrono::{DateTime, Utc};
use migration_connector::*;
use serde::{Deserialize, Serialize};

pub struct MigrationProgressCommand {
    input: MigrationProgressInput,
}

#[async_trait::async_trait]
impl MigrationCommand for MigrationProgressCommand {
    type Input = MigrationProgressInput;
    type Output = MigrationProgressOutput;

    fn new(input: Self::Input) -> Box<Self> {
        Box::new(MigrationProgressCommand { input })
    }

    async fn execute<C, D>(&self, engine: &MigrationEngine<C, D>) -> CommandResult<Self::Output>
    where
        C: MigrationConnector<DatabaseMigration = D>,
        D: DatabaseMigrationMarker + 'static,
    {
        let migration_persistence = engine.connector().migration_persistence();

        let migration = migration_persistence.by_name(&self.input.migration_id).ok_or_else(|| {
            let error = format!(
                "Could not load migration from database. Migration name was: {}",
                &self.input.migration_id
            );

            CommandError::Input { code: 1002, error }
        })?;

        Ok(MigrationProgressOutput {
            status: migration.status,
            steps: migration.datamodel_steps.len(),
            applied: migration.applied,
            rolled_back: migration.rolled_back,
            errors: migration.errors,
            started_at: migration.started_at,
            finished_at: migration.finished_at,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationProgressInput {
    pub migration_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationProgressOutput {
    status: MigrationStatus,
    steps: usize,
    applied: usize,
    rolled_back: usize,
    errors: Vec<String>,
    started_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
}
