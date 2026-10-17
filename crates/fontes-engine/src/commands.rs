use crate::{AnnotationKind, Preferences};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// IDs and argument names are the v1 frontend contract. No toolkit types cross this boundary.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "command", content = "params", deny_unknown_fields)]
pub enum Command {
    #[serde(rename = "workspace.get")]
    Workspace {},
    #[serde(rename = "reading.navigate")]
    Navigate { reference: String },
    #[serde(rename = "reading.chapter")]
    Chapter { delta: i32 },
    #[serde(rename = "reading.select")]
    Select { ref_id: i64 },
    #[serde(rename = "reading.translation")]
    Translation { id: String },
    #[serde(rename = "search.query")]
    Search {
        query: String,
        #[serde(default = "default_limit")]
        limit: u32,
        #[serde(default)]
        offset: u32,
    },
    #[serde(rename = "study.context")]
    Context { ref_id: i64 },
    #[serde(rename = "study.resource")]
    Resource { id: String },
    #[serde(rename = "study.word")]
    Word { code: String },
    #[serde(rename = "annotation.save")]
    SaveAnnotation {
        #[serde(default)]
        id: Option<String>,
        start_ref: i64,
        end_ref: i64,
        kind: AnnotationKind,
        #[serde(default)]
        color: Option<String>,
        #[serde(default)]
        content: Option<String>,
    },
    #[serde(rename = "annotation.delete")]
    DeleteAnnotation { id: String },
    #[serde(rename = "reference.favorite")]
    Favorite { ref_id: i64, enabled: bool },
    #[serde(rename = "preferences.set")]
    Preferences { preferences: Preferences },
}
fn default_limit() -> u32 {
    30
}

/// Generate each tool's input schema from exactly the Rust/Serde command definition.
pub fn command_catalog() -> Value {
    let schema = serde_json::to_value(schemars::schema_for!(Command)).expect("static schema");
    let commands = schema["oneOf"]
        .as_array()
        .expect("tagged commands")
        .iter()
        .map(|variant| {
            let id = variant["properties"]["command"]["const"]
                .as_str()
                .expect("command id");
            let mut params = variant["properties"]["params"].clone();
            if let Some(defs) = schema.get("$defs") {
                params["$defs"] = defs.clone();
            }
            json!({"name":id, "description":description(id), "inputSchema":params})
        })
        .collect::<Vec<_>>();
    json!({"protocol_version":1,"commands":commands})
}
fn description(id: &str) -> &'static str {
    match id {
        "workspace.get" => "Read the current passage, session and library metadata.",
        "reading.navigate" => {
            "Navigate to an unambiguous canonical book, chapter and optional verse."
        }
        "reading.chapter" => "Move one chapter backwards or forwards, across book boundaries.",
        "reading.select" => "Select an existing canonical verse and open its chapter.",
        "reading.translation" => {
            "Change translation while retaining canonical selection and notes."
        }
        "search.query" => {
            "Search literal terms in the active translation and installed study articles."
        }
        "study.context" => "Find typed links and actual original-language analyses for a verse.",
        "study.resource" => "Read an installed article, its provenance and scripture links.",
        "study.word" => {
            "Look up a Strong's code, definitions and the first 100 canonical occurrences."
        }
        "annotation.save" => {
            "Create or update a local note/highlight on a canonical inclusive range."
        }
        "annotation.delete" => "Delete a local annotation by its stable ID.",
        "reference.favorite" => "Set or clear a favorite canonical verse.",
        "preferences.set" => "Save reading theme, bounded typography and verse-number preferences.",
        _ => unreachable!(),
    }
}
