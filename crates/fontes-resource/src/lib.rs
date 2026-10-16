//! Source package authoring tools. This does not compile delivery SQLite or publish resources.
mod input;
pub mod publishing;
mod scaffold;
mod validate;

pub use scaffold::{InitOptions, ResourceKind, init};
pub use validate::{ValidationOptions, validate};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::Path};

const CONTRACTS: &[(&str, &str)] = &[
    (
        "common.xsd",
        include_str!("../../../specs/resource-packages/v1/common.xsd"),
    ),
    (
        "metadata.xsd",
        include_str!("../../../specs/resource-packages/v1/metadata.xsd"),
    ),
    (
        "index.xsd",
        include_str!("../../../specs/resource-packages/v1/index.xsd"),
    ),
    (
        "content.xsd",
        include_str!("../../../specs/resource-packages/v1/content.xsd"),
    ),
    (
        "concepts.xsd",
        include_str!("../../../specs/resource-packages/v1/concepts.xsd"),
    ),
];
const NAMESPACE: &str = "urn:fontes:resource:1";

#[derive(Clone, Debug, Serialize)]
pub struct Diagnostic {
    pub severity: &'static str,
    pub code: &'static str,
    pub file: String,
    /// One-based XML positions. Non-XML/global errors use no invented location.
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub message: String,
    pub help: String,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub report_version: u32,
    pub valid: bool,
    pub resource_id: Option<String>,
    pub resource_version: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Default for Report {
    fn default() -> Self {
        Self {
            report_version: 1,
            valid: true,
            resource_id: None,
            resource_version: None,
            diagnostics: vec![],
        }
    }
}

impl Report {
    fn diagnostic(&mut self, diagnostic: Diagnostic) {
        if diagnostic.severity == "error" {
            self.valid = false;
        }
        self.diagnostics.push(diagnostic);
    }

    fn global(&mut self, code: &'static str, file: &str, message: impl Into<String>, help: &str) {
        self.diagnostic(Diagnostic {
            severity: "error",
            code,
            file: file.into(),
            line: None,
            column: None,
            message: message.into(),
            help: help.into(),
        });
    }

    pub fn text(&self) -> String {
        let mut text = String::new();
        for d in &self.diagnostics {
            text.push_str(&d.file);
            if let Some(line) = d.line {
                text.push_str(&format!(":{line}"));
            }
            if let Some(column) = d.column {
                text.push_str(&format!(":{column}"));
            }
            text.push_str(&format!(
                ": {}[{}]: {}\n  help: {}\n",
                d.severity, d.code, d.message, d.help
            ));
        }
        text.push_str(if self.valid {
            "Package validation passed.\n"
        } else {
            "Package validation failed.\n"
        });
        text
    }
}

#[derive(Debug, Serialize)]
pub struct PackResult {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}

/// Validate a stable snapshot and package exactly those bytes in sorted order.
/// No existing output is replaced; validation failure creates no archive.
pub fn pack(source: &Path, output: &Path, options: &ValidationOptions) -> Result<PackResult> {
    let mut report = Report::default();
    let Some(snapshot) = input::Snapshot::load(source, &mut report)? else {
        bail!("{}", report.text());
    };
    validate::snapshot(&snapshot, options, &mut report)?;
    if !report.valid {
        bail!("{}", report.text());
    }
    let name = format!(
        "{}--{}.fontes-source.zip",
        report
            .resource_id
            .as_deref()
            .context("missing resource identity")?,
        report
            .resource_version
            .as_deref()
            .context("missing resource version")?
    );
    if output.file_name().and_then(|s| s.to_str()) != Some(&name) {
        bail!("output filename must be {name}");
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    {
        let mut zip = zip::ZipWriter::new(temp.as_file_mut());
        let settings = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .last_modified_time(zip::DateTime::default())
            .unix_permissions(0o644);
        for path in &snapshot.files {
            zip.start_file(path, settings)?;
            std::io::copy(
                &mut fs::File::open(snapshot.root.path().join(path))?,
                &mut zip,
            )?;
        }
        zip.finish()?;
    }
    temp.flush()?;
    temp.as_file().sync_all()?;
    let mut hash = Sha256::new();
    std::io::copy(&mut fs::File::open(temp.path())?, &mut hash)?;
    let bytes = temp.as_file().metadata()?.len();
    let sha256 = hex::encode(hash.finalize());
    temp.persist_noclobber(output)
        .map_err(|e| e.error)
        .context("output exists or cannot be created; choose a new destination")?;
    Ok(PackResult {
        path: output.display().to_string(),
        sha256,
        bytes,
    })
}

pub fn write_schemas(destination: &Path) -> Result<()> {
    if destination.exists() {
        bail!("schema destination already exists; choose a new directory");
    }
    fs::create_dir_all(destination)?;
    for (name, text) in CONTRACTS {
        fs::write(destination.join(name), text)?;
    }
    Ok(())
}
