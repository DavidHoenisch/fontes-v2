use crate::{NAMESPACE, Report, ValidationOptions, validate};
use anyhow::{Context, Result, bail};
use clap::{Args, ValueEnum};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ResourceKind {
    Translation,
    Commentary,
    Lexicon,
    WordStudy,
    StudyGuide,
    Devotional,
    Book,
}

impl ResourceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Translation => "translation",
            Self::Commentary => "commentary",
            Self::Lexicon => "lexicon",
            Self::WordStudy => "word-study",
            Self::StudyGuide => "study-guide",
            Self::Devotional => "devotional",
            Self::Book => "book",
        }
    }
}

#[derive(Debug, Args)]
pub struct InitOptions {
    /// New directory; existing directories are never overwritten.
    pub directory: PathBuf,
    #[arg(long)]
    pub id: String,
    #[arg(long)]
    pub publisher_id: String,
    #[arg(long, value_enum, default_value = "book")]
    pub kind: ResourceKind,
    #[arg(long, default_value = "Untitled resource")]
    pub title: String,
    #[arg(long)]
    pub subtitle: Option<String>,
    #[arg(
        long,
        default_value = "FONTES-TODO: Describe this resource and its intended scope."
    )]
    pub description: String,
    /// Repeat for multiple authors.
    #[arg(long)]
    pub author: Vec<String>,
    #[arg(long, default_value = "Unspecified publisher")]
    pub publisher: String,
    #[arg(long)]
    pub publication_date: Option<String>,
    #[arg(long)]
    pub edition: Option<String>,
    #[arg(long, default_value = "en")]
    pub language: String,
    #[arg(long, default_value = "1.0.0")]
    pub version: String,
    #[arg(long, default_value = "KJV")]
    pub source_versification: String,
    /// OSIS anchor for initial selected coverage and the example block.
    #[arg(long, default_value = "Gen.1.1")]
    pub start: String,
    #[arg(long)]
    pub end: Option<String>,
    #[arg(long, default_value = "LicenseRef-Pending")]
    pub license_id: String,
    #[arg(long, default_value = "Unspecified rights holder")]
    pub rights_holder: String,
    /// Copy an existing UTF-8 license/attribution text rather than a TODO template.
    #[arg(long)]
    pub license_file: Option<PathBuf>,
    #[arg(long)]
    pub attribution_file: Option<PathBuf>,
    /// Optional 1200x1800 PNG/JPEG. Without it, create a clearly identified CC0 placeholder.
    #[arg(long)]
    pub cover: Option<PathBuf>,
    #[arg(long)]
    pub cover_license_id: Option<String>,
    #[arg(long)]
    pub cover_rights_holder: Option<String>,
    #[arg(long)]
    pub cover_attribution: Option<String>,
    #[arg(long)]
    pub cover_alt: Option<String>,
}

fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Write a validated scaffold without inferring any text redistribution grant.
pub fn init(options: &InitOptions) -> Result<Report> {
    if options.directory.exists() {
        bail!("destination already exists; choose a new directory");
    }
    let stage = tempfile::tempdir()?;
    for dir in ["assets", "rights", "content", "media", "taxonomy"] {
        fs::create_dir(stage.path().join(dir))?;
    }
    let mut cover_name = "assets/cover.png";
    let mut media_type = "image/png";
    if let Some(path) = &options.cover {
        let format = image::guess_format(&fs::read(path)?)?;
        if format == image::ImageFormat::Jpeg {
            cover_name = "assets/cover.jpg";
            media_type = "image/jpeg";
        } else if format != image::ImageFormat::Png {
            bail!("cover must be PNG or JPEG");
        }
        fs::copy(path, stage.path().join(cover_name))?;
    } else {
        fs::write(
            stage.path().join(cover_name),
            include_bytes!(
                "../../../specs/resource-packages/v1/examples/romans-study/assets/cover.png"
            ),
        )?;
    }
    let is_custom = options.cover.is_some();
    let cover_license = options.cover_license_id.as_deref().unwrap_or(if is_custom {
        "LicenseRef-Pending"
    } else {
        "CC0-1.0"
    });
    let cover_holder = options
        .cover_rights_holder
        .as_deref()
        .unwrap_or(if is_custom {
            "FONTES-TODO: Verify artwork rights holder"
        } else {
            "Fontes format example contributors"
        });
    let cover_attribution = options.cover_attribution.as_deref().unwrap_or(if is_custom { "FONTES-TODO: Supply artwork permission and credit" } else { "FONTES-TODO: Replace placeholder cover before publication. Original Fontes example artwork; CC0-1.0." });
    let cover_alt = options.cover_alt.as_deref().unwrap_or(if is_custom {
        "FONTES-TODO: Describe the cover"
    } else {
        "Placeholder blue cover with a geometric book motif"
    });
    let authors = if options.author.is_empty() {
        vec!["FONTES-TODO: Author".to_owned()]
    } else {
        options.author.clone()
    };
    let contributors = authors
        .iter()
        .map(|author| {
            format!(
                "    <contributor role=\"author\">{}</contributor>\n",
                escaped(author)
            )
        })
        .collect::<String>();
    let optional = |tag: &str, value: Option<&str>| {
        value
            .map(|text| format!("  <{tag}>{}</{tag}>\n", escaped(text)))
            .unwrap_or_default()
    };
    let end = options.end.as_deref().unwrap_or(&options.start);
    let metadata = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<resource xmlns="{NAMESPACE}" format-version="1" id="{id}" version="{version}" kind="{kind}" language="{language}">
  <title>{title}</title>
{subtitle}  <description>{description}</description>
  <contributors>
{contributors}  </contributors>
  <publisher id="{publisher_id}">{publisher}</publisher>
{edition}{date}  <cover href="{cover_name}" media-type="{media_type}" width="1200" height="1800" alt="{cover_alt}" license-id="{cover_license}" rights-holder="{cover_holder}" attribution="{cover_attribution}"/>
  <references canonical-registry="fontes-protestant-66-v1" source-versification="{versification}"/>
  <coverage mode="selected"><range start="{start}" end="{end}"/></coverage>
  <rights license-id="{license}" rights-holder="{holder}" license-file="rights/LICENSE.txt" attribution-file="rights/ATTRIBUTION.txt">
    <copyright>FONTES-TODO: Supply the actual copyright or public-domain statement.</copyright>
    <attribution>FONTES-TODO: Supply required source notices and credits.</attribution>
    <permissions offline-display="deny" keyword-search="deny" export="deny" print="deny" embeddings="deny" ai-context="deny"/>
  </rights>
</resource>
"#,
        id = escaped(&options.id),
        version = escaped(&options.version),
        kind = options.kind.as_str(),
        language = escaped(&options.language),
        title = escaped(&options.title),
        subtitle = optional("subtitle", options.subtitle.as_deref()),
        description = escaped(&options.description),
        publisher_id = escaped(&options.publisher_id),
        publisher = escaped(&options.publisher),
        edition = optional("edition", options.edition.as_deref()),
        date = optional("publication-date", options.publication_date.as_deref()),
        cover_alt = escaped(cover_alt),
        cover_license = escaped(cover_license),
        cover_holder = escaped(cover_holder),
        cover_attribution = escaped(cover_attribution),
        versification = escaped(&options.source_versification),
        start = escaped(&options.start),
        end = escaped(end),
        license = escaped(&options.license_id),
        holder = escaped(&options.rights_holder)
    );
    fs::write(stage.path().join("metadata.xml"), metadata)?;
    fs::write(
        stage.path().join("index.xml"),
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<index xmlns=\"{NAMESPACE}\" format-version=\"1\" resource-id=\"{}\">\n  <document id=\"chapter-001\" title=\"First chapter\" href=\"content/001.xml\"/>\n</index>\n",
            escaped(&options.id)
        ),
    )?;
    let (block, source, relation) = if matches!(options.kind, ResourceKind::Translation) {
        (
            "verse",
            format!(" source-ref=\"{}\"", escaped(&options.start)),
            "text-of",
        )
    } else {
        ("paragraph", String::new(), "applies-to")
    };
    fs::write(
        stage.path().join("content/001.xml"),
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<document xmlns=\"{NAMESPACE}\" id=\"chapter-001\" resource-id=\"{}\">\n  <title>First chapter</title>\n  <block id=\"chapter-001-p01\" kind=\"{block}\"{source}>\n    <text>FONTES-TODO: Replace this example with your resource text.</text>\n    <annotations><scripture start=\"{}\" end=\"{}\" relation=\"{relation}\" basis=\"author\"/></annotations>\n  </block>\n</document>\n",
            escaped(&options.id),
            escaped(&options.start),
            escaped(end)
        ),
    )?;
    for (path, source) in [
        ("rights/LICENSE.txt", &options.license_file),
        ("rights/ATTRIBUTION.txt", &options.attribution_file),
    ] {
        if let Some(source) = source {
            fs::copy(source, stage.path().join(path))?;
        } else {
            fs::write(
                stage.path().join(path),
                "FONTES-TODO: Replace with verified license terms or attribution. This template grants no rights to resource text.\n",
            )?;
        }
    }
    let report = validate(stage.path(), &ValidationOptions::default())?;
    if !report.valid {
        bail!("scaffold options are invalid:\n{}", report.text());
    }
    let parent = options
        .directory
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    fs::create_dir(&options.directory)
        .context("destination already exists or cannot be created")?;
    let result = (|| -> Result<()> {
        for entry in fs::read_dir(stage.path())? {
            let entry = entry?;
            // Copy rather than rename: the staging directory may be on another filesystem.
            copy_tree(&entry.path(), &options.directory.join(entry.file_name()))?;
        }
        Ok(())
    })();
    if let Err(error) = result {
        fs::remove_dir_all(&options.directory)?;
        return Err(error);
    }
    Ok(report)
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    if source.is_dir() {
        fs::create_dir(destination)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy_tree(&entry.path(), &destination.join(entry.file_name()))?;
        }
    } else {
        fs::copy(source, destination)?;
    }
    Ok(())
}
