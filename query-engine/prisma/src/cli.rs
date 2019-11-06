use crate::{
    data_model_loader::{load_configuration, load_data_model_components},
    dmmf, PrismaResult,
};
use clap::ArgMatches;
use query_core::{
    schema::{QuerySchemaRef, SupportedCapabilities},
    BuildMode, QuerySchemaBuilder,
};
use serde::Deserialize;
use std::sync::Arc;
use datamodel::dmmf::Datamodel;
use std::fs::File;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DmmfToDmlInput {
    pub dmmf: String,
    pub config: serde_json::Value,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DmmfFileToDmlInput {
    pub dmmf: Datamodel,
    pub config: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetConfigInput {
    pub datamodel: String,
}

pub enum CliCommand {
    Dmmf(BuildMode),
    DmmfFileToDml(DmmfFileToDmlInput),
    DmmfToDml(DmmfToDmlInput),
    GetConfig(GetConfigInput),
}

impl CliCommand {
    pub fn new(matches: &ArgMatches) -> Option<Self> {
        if matches.is_present("dmmf") {
            let build_mode = if matches.is_present("legacy") {
                BuildMode::Legacy
            } else {
                BuildMode::Modern
            };

            Some(Self::Dmmf(build_mode))
        } else if matches.is_present("dmmf_file_to_dml") {
            let path = matches.value_of("dmmf_file_to_dml").unwrap();
            let file = File::open(path)
                .expect("File should open read only");
            let input: DmmfFileToDmlInput = serde_json::from_reader(file)
                .expect("File should be proper JSON");
            Some(Self::DmmfFileToDml(input))
        } else if matches.is_present("get_config") {
            let input = matches.value_of("get_config").unwrap();
            let input: GetConfigInput = serde_json::from_str(input).unwrap();

            Some(Self::GetConfig(input))
        } else {
            None
        }
    }

    pub fn execute(self) -> PrismaResult<()> {
        match self {
            CliCommand::Dmmf(build_mode) => Self::dmmf(build_mode),
            CliCommand::DmmfFileToDml(input) => Self::dmmf_file_to_dml(input),
            CliCommand::GetConfig(input) => Self::get_config(input),
        }
    }

    fn dmmf(build_mode: BuildMode) -> PrismaResult<()> {
        let (v2components, template) = load_data_model_components()?;

        // temporary code duplication
        let internal_data_model = template.build("".into());
        let capabilities = SupportedCapabilities::empty();

        let schema_builder = QuerySchemaBuilder::new(&internal_data_model, &capabilities, build_mode);
        let query_schema: QuerySchemaRef = Arc::new(schema_builder.build());

        let dmmf = dmmf::render_dmmf(&v2components.datamodel, query_schema);
        let serialized = serde_json::to_string_pretty(&dmmf)?;

        println!("{}", serialized);

        Ok(())
    }

    fn dmmf_file_to_dml(input: DmmfFileToDmlInput) -> PrismaResult<()> {
        let datamodel = datamodel::dmmf::schema_from_dmmf(&input.dmmf);
        let config = datamodel::config_from_mcf_json_value(input.config);
        let serialized = datamodel::render_datamodel_and_config_to_string(&datamodel, &config)?;

        println!("{}", serialized);


        Ok(())
    }
    
    fn dmmf_to_dml(input: DmmfToDmlInput) -> PrismaResult<()> {
        let datamodel = datamodel::dmmf::parse_from_dmmf(&input.dmmf);
        let config = datamodel::config_from_mcf_json_value(input.config);
        let serialized = datamodel::render_datamodel_and_config_to_string(&datamodel, &config)?;

        println!("{}", serialized);

        Ok(())
    }

    fn get_config(input: GetConfigInput) -> PrismaResult<()> {
        let config = load_configuration(&input.datamodel)?;
        let json = datamodel::config_to_mcf_json_value(&config);
        let serialized = serde_json::to_string(&json)?;

        println!("{}", serialized);

        Ok(())
    }
}
