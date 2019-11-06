mod from_dmmf;
mod to_dmmf;

use serde;
use serde_json;

pub use from_dmmf::parse_from_dmmf;
pub use from_dmmf::schema_from_dmmf;
pub use to_dmmf::render_to_dmmf;
pub use to_dmmf::render_to_dmmf_value;

// This is a simple JSON serialization using Serde.
// The JSON format follows the DMMF spec.
#[serde(rename_all = "camelCase")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Field {
    pub name: String,
    pub kind: String,
    pub db_name: Option<String>,
    pub is_list: bool,
    pub is_required: bool,
    pub is_unique: bool,
    pub is_id: bool,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation_to_fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation_on_delete: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_generated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_updated_at: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Function {
    pub name: String,
    pub return_type: String,
    pub args: Vec<serde_json::Value>,
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Model {
    pub name: String,
    pub is_embedded: bool,
    pub db_name: Option<String>,
    pub fields: Vec<Field>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_generated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    pub id_fields: Vec<String>,
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Enum {
    pub name: String,
    pub values: Vec<String>,
    pub db_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Datamodel {
    pub enums: Vec<Enum>,
    pub models: Vec<Model>,
}
