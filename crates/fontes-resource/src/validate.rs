use crate::{
    CONTRACTS, Diagnostic, NAMESPACE, Report,
    input::{Snapshot, safe_path},
};
use anyhow::Result;
use roxmltree::{Document, Node, ParsingOptions};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Default)]
pub struct ValidationOptions {
    /// Explicit local registry snapshot for concept resolution; no network fetch.
    pub taxonomy: Option<PathBuf>,
    pub deny_warnings: bool,
}

fn at(
    file: &str,
    node: Node<'_, '_>,
    attribute: Option<&str>,
    code: &'static str,
    message: impl Into<String>,
    help: &str,
) -> Diagnostic {
    let offset = attribute
        .and_then(|name| node.attribute_node(name))
        .map(|a| a.range().start)
        .unwrap_or(node.range().start);
    let position = node.document().text_pos_at(offset);
    Diagnostic {
        severity: "error",
        code,
        file: file.into(),
        line: Some(position.row),
        column: Some(position.col),
        message: message.into(),
        help: help.into(),
    }
}

fn warning(mut diagnostic: Diagnostic) -> Diagnostic {
    diagnostic.severity = "warning";
    diagnostic
}

pub fn validate(path: &Path, options: &ValidationOptions) -> Result<Report> {
    let mut report = Report::default();
    if let Some(source) = Snapshot::load(path, &mut report)? {
        snapshot(&source, options, &mut report)?;
    }
    Ok(report)
}

fn parsed<'a>(text: &'a str, file: &str, report: &mut Report) -> Option<Document<'a>> {
    match Document::parse_with_options(
        text,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: 1_000_000,
            ..Default::default()
        },
    ) {
        Ok(doc) => {
            if let Some(node) = doc.descendants().find(|node| node.is_pi()) {
                report.diagnostic(at(file, node, None, "XML_INSTRUCTION", "XML processing instructions are not supported.", "Remove stylesheet or other processing instructions; use the supported content vocabulary."));
                return None;
            }
            if let Some(node) = doc
                .descendants()
                .find(|node| node.ancestors().filter(|n| n.is_element()).count() > 32)
            {
                report.diagnostic(at(
                    file,
                    node,
                    None,
                    "XML_DEPTH",
                    "XML nesting exceeds 32 elements.",
                    "Reduce nesting or split the content into smaller blocks.",
                ));
                return None;
            }
            Some(doc)
        }
        Err(error) => {
            report.diagnostic(Diagnostic { severity: "error", code: "XML_PARSE", file: file.into(), line: Some(error.pos().row), column: Some(error.pos().col), message: error.to_string(), help: "Correct the XML syntax at this position; close elements, escape & as &amp;, and remove DTD/entity declarations.".into() });
            None
        }
    }
}

fn schema_check(
    path: &Path,
    file: &str,
    schema: &str,
    schemas: &Path,
    doc: &Document<'_>,
    report: &mut Report,
) -> Result<()> {
    let output = Command::new("xmllint")
        .args(["--nonet", "--noout", "--schema"])
        .arg(schemas.join(schema))
        .arg(path)
        .env("XML_CATALOG_FILES", "")
        .env("SGML_CATALOG_FILES", "")
        .output()?;
    if output.status.success() {
        return Ok(());
    }
    let prefix = format!("{}:", path.display());
    let mut found = false;
    for line in String::from_utf8_lossy(&output.stderr).lines() {
        let Some(rest) = line.strip_prefix(&prefix) else {
            continue;
        };
        let Some((number, message)) = rest.split_once(':') else {
            continue;
        };
        let Ok(row) = number.parse::<u32>() else {
            continue;
        };
        let element = message
            .trim()
            .strip_prefix("element ")
            .and_then(|s| s.split_once(':'))
            .map(|(name, _)| name);
        let column = element
            .and_then(|name| {
                doc.descendants().find(|n| {
                    n.is_element()
                        && n.tag_name().name() == name
                        && doc.text_pos_at(n.range().start).row == row
                })
            })
            .map(|n| doc.text_pos_at(n.range().start).col);
        let help = if message.contains("enumeration") {
            "Use one of the allowed values listed in this error; check the matching bundled XSD."
        } else if message.contains("attribute") {
            "Correct the named attribute's spelling/type, supply required attributes, and remove unsupported attributes."
        } else {
            "Use only supported elements in the required order; compare this file with the matching XSD and authoring example."
        };
        report.diagnostic(Diagnostic {
            severity: "error",
            code: "XML_SCHEMA",
            file: file.into(),
            line: Some(row),
            column,
            message: message.trim().into(),
            help: help.into(),
        });
        found = true;
    }
    if !found {
        report.global("XML_SCHEMA", file, "XML schema validation failed without a source location.", "Check the bundled schemas and xmllint installation; run xmllint manually for system diagnostics.");
    }
    Ok(())
}

pub(crate) fn snapshot(
    source: &Snapshot,
    options: &ValidationOptions,
    report: &mut Report,
) -> Result<()> {
    if Command::new("xmllint").arg("--version").output().is_err() {
        report.global("XMLLINT_UNAVAILABLE", "", "xmllint is required for strict XSD validation.", "Install libxml2's xmllint and put it on PATH; see the CLI guide for platform instructions.");
        return Ok(());
    }
    let schemas = tempfile::tempdir()?;
    for (name, text) in CONTRACTS {
        fs::write(schemas.path().join(name), text)?;
    }
    let mut texts = BTreeMap::new();
    for file in &source.files {
        if file.ends_with(".xml") {
            match fs::read_to_string(source.root.path().join(file)) {
                Ok(text) => {
                    texts.insert(file.clone(), text);
                }
                Err(_) => report.global(
                    "XML_ENCODING",
                    file,
                    "XML is not valid UTF-8.",
                    "Save this XML file as UTF-8.",
                ),
            }
        }
    }
    let mut documents = BTreeMap::new();
    for (file, text) in &texts {
        if let Some(doc) = parsed(text, file, report) {
            let schema = match file.as_str() {
                "metadata.xml" => Some("metadata.xsd"),
                "index.xml" => Some("index.xsd"),
                "taxonomy/concepts.xml" => Some("concepts.xsd"),
                _ if file.starts_with("content/") => Some("content.xsd"),
                _ => None,
            };
            if let Some(schema) = schema {
                schema_check(
                    &source.root.path().join(file),
                    file,
                    schema,
                    schemas.path(),
                    &doc,
                    report,
                )?;
            } else {
                report.global(
                    "UNEXPECTED_XML",
                    file,
                    "XML file is outside the supported source layout.",
                    "Use metadata.xml, index.xml, content/*.xml, or taxonomy/concepts.xml.",
                );
            }
            if doc.root_element().tag_name().namespace() != Some(NAMESPACE) {
                report.diagnostic(at(
                    file,
                    doc.root_element(),
                    None,
                    "XML_NAMESPACE",
                    "Wrong or missing Fontes namespace.",
                    "Set xmlns=\"urn:fontes:resource:1\" on the root element.",
                ));
            }
            documents.insert(file.as_str(), doc);
        }
    }
    for file in [
        "metadata.xml",
        "index.xml",
        "rights/LICENSE.txt",
        "rights/ATTRIBUTION.txt",
    ] {
        if !source.files.contains(file) {
            report.global(
                "REQUIRED_FILE",
                file,
                "Required source file is missing.",
                "Add this file at the archive root-relative path shown.",
            );
        }
    }
    // Continue collecting schema errors across files, then stop semantic checks
    // when structural assumptions are unsafe. Semantic failures aggregate below.
    if !report.valid {
        return Ok(());
    }
    let metadata = &documents["metadata.xml"];
    let root = metadata.root_element();
    let resource = root.attribute("id").unwrap_or("");
    let version = root.attribute("version").unwrap_or("");
    report.resource_id = Some(resource.into());
    report.resource_version = Some(version.into());
    let publisher_node = child(root, "publisher").unwrap();
    let publisher = publisher_node.attribute("id").unwrap_or("");
    if !resource.starts_with(&format!("{publisher}.")) {
        report.diagnostic(at(
            "metadata.xml",
            root,
            Some("id"),
            "RESOURCE_NAMESPACE",
            "Resource ID is not owned by the declared publisher namespace.",
            "Prefix the resource ID with the publisher ID followed by a dot.",
        ));
    }
    // XSD token normalization must not turn distinct serialized identities into
    // the same key; use exact canonical lexical values throughout the compiler.
    for (name, value) in [
        ("id", resource),
        ("version", version),
        ("language", root.attribute("language").unwrap_or("")),
    ] {
        if value.trim() != value || value.contains(char::is_whitespace) {
            report.diagnostic(at(
                "metadata.xml",
                root,
                Some(name),
                "IDENTITY_WHITESPACE",
                "Identity/version/language contains whitespace.",
                "Remove whitespace from machine-readable identifiers.",
            ));
        }
    }
    let mut required: BTreeSet<String> = [
        "metadata.xml",
        "index.xml",
        "rights/LICENSE.txt",
        "rights/ATTRIBUTION.txt",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    for file in ["rights/LICENSE.txt", "rights/ATTRIBUTION.txt"] {
        match fs::read_to_string(source.root.path().join(file)) {
            Ok(text) if !text.trim().is_empty() => {
                if text.contains("FONTES-TODO") {
                    report.diagnostic(Diagnostic { severity:"warning", code:"PLACEHOLDER", file:file.into(), line:None, column:None, message:"Template rights text remains.".into(), help:"Replace FONTES-TODO text with the verified license terms and required attribution.".into() });
                }
            }
            _ => report.global(
                "RIGHTS_TEXT",
                file,
                "Rights file must contain nonempty UTF-8 text.",
                "Supply the actual license terms or attribution as UTF-8 text.",
            ),
        }
    }
    let cover = child(root, "cover").unwrap();
    if let Some(path) = target(
        source,
        "metadata.xml",
        cover,
        "href",
        "assets/",
        &mut required,
        report,
    ) {
        if !matches!(path.as_str(), "assets/cover.png" | "assets/cover.jpg") {
            report.diagnostic(at(
                "metadata.xml",
                cover,
                Some("href"),
                "COVER_PATH",
                "Cover must be assets/cover.png or assets/cover.jpg.",
                "Rename the cover and update metadata.xml to the fixed cover path.",
            ));
        }
        image_check(
            source,
            &path,
            "metadata.xml",
            cover,
            "href",
            true,
            cover.attribute("media-type"),
            report,
        )?;
    }
    let permissions = root
        .descendants()
        .find(|n| n.has_tag_name((NAMESPACE, "permissions")))
        .unwrap();
    if permissions.attribute("offline-display") != Some("allow")
        || permissions.attribute("keyword-search") != Some("allow")
    {
        report.diagnostic(warning(at("metadata.xml", permissions, None, "PUBLIC_CATALOG_RIGHTS", "Offline display and keyword search are not both granted.", "For public catalog submission, obtain both grants and then update permissions. Never infer permission from a free download.")));
    }
    for (file, doc) in &documents {
        for node in doc.descendants().filter(|node| node.is_element()) {
            if matches!(
                node.tag_name().name(),
                "description" | "scope-note" | "copyright" | "attribution"
            ) && node.text().is_none_or(|text| text.trim().is_empty())
            {
                report.diagnostic(at(file, node, None, "EMPTY_VALUE", "Required descriptive text contains only whitespace.", "Supply a meaningful description, scope note, copyright statement, or attribution."));
            }
            if node
                .attribute("alt")
                .is_some_and(|text| text.trim().is_empty())
            {
                report.diagnostic(at(
                    file,
                    node,
                    Some("alt"),
                    "EMPTY_VALUE",
                    "Image alternative text contains only whitespace.",
                    "Describe the image in its alt attribute.",
                ));
            }
            for attr in node
                .attributes()
                .filter(|a| matches!(a.name(), "id" | "resource-id" | "publisher-id"))
            {
                if attr.value().contains(char::is_whitespace) {
                    report.diagnostic(at(
                        file,
                        node,
                        Some(attr.name()),
                        "IDENTITY_WHITESPACE",
                        "Machine-readable identity contains whitespace.",
                        "Remove whitespace from IDs; use display labels for human-readable names.",
                    ));
                }
            }
            if node.attributes().any(|a| {
                a.value().contains("FONTES-TODO") || a.value().contains("LicenseRef-Pending")
            }) || node
                .children()
                .filter(|n| n.is_text())
                .any(|n| n.text().unwrap_or("").contains("FONTES-TODO"))
            {
                report.diagnostic(warning(at(file, node, None, "PLACEHOLDER", "Template content or pending rights metadata remains.", "Replace the FONTES-TODO/LicenseRef-Pending value with verified resource information before publication.")));
            }
        }
    }
    let books: Registry =
        serde_json::from_str(include_str!("../../../data/canonical/registry-v1.json"))?;
    let anchors: BTreeSet<_> = books.anchors().collect();
    let coverage = reference_nodes("metadata.xml", metadata, &books, report);
    let mut concept_text = None;
    if let Some(path) = &options.taxonomy {
        let file = path.display().to_string();
        let bytes = fs::read(path)?;
        if bytes.len() > crate::input::MAX_XML as usize {
            report.global(
                "TAXONOMY_SIZE",
                &file,
                "Taxonomy exceeds 16 MiB.",
                "Use a bounded registry snapshot.",
            );
        } else {
            match String::from_utf8(bytes) {
                Ok(text) => concept_text = Some((file, text)),
                Err(_) => report.global(
                    "XML_ENCODING",
                    &file,
                    "Taxonomy is not UTF-8.",
                    "Save the registry as UTF-8.",
                ),
            }
        }
    }
    let taxonomy = concept_text
        .as_ref()
        .and_then(|(file, text)| parsed(text, file, report));
    if let Some(doc) = &taxonomy {
        let file = &concept_text.as_ref().unwrap().0;
        let path = schemas.path().join("registry.xml");
        fs::write(&path, &concept_text.as_ref().unwrap().1)?;
        schema_check(&path, file, "concepts.xsd", schemas.path(), doc, report)?;
        if doc.root_element().attribute("scope") != Some("registry") {
            report.diagnostic(at(file, doc.root_element(), Some("scope"), "TAXONOMY_SCOPE", "Selected taxonomy must be a registry snapshot.", "Use a registry with scope=\"registry\"; put publisher proposals inside the source package."));
        }
    }
    let mut concepts: BTreeMap<String, (String, Node<'_, '_>)> = BTreeMap::new();
    if let Some(doc) = &taxonomy {
        collect_concepts(
            &concept_text.as_ref().unwrap().0,
            doc,
            None,
            &mut concepts,
            report,
        );
    }
    if let Some(doc) = documents.get("taxonomy/concepts.xml") {
        required.insert("taxonomy/concepts.xml".into());
        let root = doc.root_element();
        if root.attribute("publisher-id") != Some(publisher)
            || root.attribute("scope") != Some("publisher")
        {
            report.diagnostic(at(
                "taxonomy/concepts.xml",
                root,
                None,
                "CONCEPT_AUTHORITY",
                "Proposal scope or publisher differs from metadata.",
                "Use scope=\"publisher\" and the same publisher-id as metadata.xml.",
            ));
        }
        collect_concepts(
            "taxonomy/concepts.xml",
            doc,
            Some(publisher),
            &mut concepts,
            report,
        );
        reference_nodes("taxonomy/concepts.xml", doc, &books, report);
    }
    concept_graph(&concepts, report);
    let index = &documents["index.xml"];
    if index.root_element().attribute("resource-id") != Some(resource) {
        report.diagnostic(at(
            "index.xml",
            index.root_element(),
            Some("resource-id"),
            "RESOURCE_MISMATCH",
            "Index resource ID differs from metadata.",
            "Use the exact metadata.xml resource ID in index.xml.",
        ));
    }
    let mut ids = HashSet::new();
    let mut expected_docs = HashMap::new();
    for entry in index
        .descendants()
        .filter(|n| n.is_element() && n != &index.root_element())
    {
        if entry
            .ancestors()
            .filter(|n| n.has_tag_name((NAMESPACE, "section")))
            .count()
            > 8
        {
            report.diagnostic(at(
                "index.xml",
                entry,
                None,
                "NAVIGATION_DEPTH",
                "Navigation exceeds eight nested sections.",
                "Flatten the index to at most eight section levels.",
            ));
        }
        if !ids.insert(entry.attribute("id").unwrap_or("").to_owned()) {
            report.diagnostic(at(
                "index.xml",
                entry,
                Some("id"),
                "DUPLICATE_ID",
                "Navigation/content ID is reused.",
                "Choose an ID unique across all sections, documents, and blocks in this resource.",
            ));
        }
        if entry.has_tag_name((NAMESPACE, "document"))
            && let Some(path) = target(
                source,
                "index.xml",
                entry,
                "href",
                "content/",
                &mut required,
                report,
            )
        {
            if !path.ends_with(".xml") {
                report.diagnostic(at(
                    "index.xml",
                    entry,
                    Some("href"),
                    "CONTENT_PATH",
                    "Indexed content must be XML.",
                    "Reference a content/*.xml document.",
                ));
            }
            expected_docs.insert(path, entry.attribute("id").unwrap_or(""));
        }
    }
    let mut mapped = BTreeSet::new();
    for (file, doc) in &documents {
        if !file.starts_with("content/") {
            continue;
        }
        let root = doc.root_element();
        if let Some(id) = expected_docs.get(*file)
            && (root.attribute("id") != Some(*id)
                || root.attribute("resource-id") != Some(resource))
        {
            report.diagnostic(at(
                file,
                root,
                None,
                "DOCUMENT_IDENTITY",
                "Document identity differs from its index entry or metadata.",
                "Match the document id to index.xml and resource-id to metadata.xml.",
            ));
        }
        for node in root
            .children()
            .filter(|n| n.has_tag_name((NAMESPACE, "block")))
        {
            if !ids.insert(node.attribute("id").unwrap_or("").to_owned()) {
                report.diagnostic(at(file, node, Some("id"), "DUPLICATE_ID", "Content ID is reused in this resource.", "Give this block a stable ID unique across every document and navigation section."));
            }
            let text = child(node, "text");
            let omitted = node.attribute("status") == Some("omitted");
            if omitted && (node.attribute("kind") != Some("verse") || text.is_some()) {
                report.diagnostic(at(
                    file,
                    node,
                    Some("status"),
                    "OMITTED_VERSE",
                    "Only verse blocks without text may be marked omitted.",
                    "Remove invented text from an omitted source verse, or use status=\"present\".",
                ));
            } else if !omitted
                && !text.is_some_and(|text| {
                    text.descendants().any(|n| {
                        n.is_text() && !n.text().unwrap_or("").trim().is_empty()
                            || n.has_tag_name((NAMESPACE, "image"))
                    })
                })
            {
                report.diagnostic(at(
                    file,
                    node,
                    None,
                    "BLOCK_CONTENT",
                    "Present block has no text or image content.",
                    "Add a nonempty <text> element, or mark an actual omitted verse explicitly.",
                ));
            }
            if node.attribute("kind") == Some("verse") {
                let mappings: Vec<_> = node
                    .descendants()
                    .filter(|n| {
                        n.has_tag_name((NAMESPACE, "scripture"))
                            && n.attribute("relation") == Some("text-of")
                    })
                    .collect();
                if node
                    .attribute("source-ref")
                    .is_none_or(|s| s.trim().is_empty())
                    || mappings.is_empty()
                {
                    report.diagnostic(at(file, node, None, "VERSE_MAPPING", "Verse requires an original source-ref and text-of canonical mapping.", "Retain the source numbering in source-ref and add reviewed text-of scripture annotation(s)."));
                }
                for mapping in mappings {
                    if let Some((start, end)) = reference_range(&books, mapping) {
                        let verse_anchors = anchors.range(start..=end);
                        let outside = verse_anchors
                            .clone()
                            .any(|id| !coverage.iter().any(|(a, b)| id >= a && id <= b));
                        mapped.extend(verse_anchors.copied());
                        if root_kind(metadata) == "translation" && outside {
                            report.diagnostic(at(file, mapping, Some("start"), "COVERAGE_SCOPE", "Translation verse mapping lies outside declared coverage.", "Correct the mapping or extend metadata.xml coverage to include it."));
                        }
                    }
                }
            }
        }
        reference_nodes(file, doc, &books, report);
        for node in doc.descendants().filter(|n| n.is_element()) {
            match node.tag_name().name() {
                "image" => {
                    if let Some(path) =
                        target(source, file, node, "src", "media/", &mut required, report)
                    {
                        image_check(source, &path, file, node, "src", false, None, report)?;
                    }
                }
                "concept" | "term" => {
                    let attr = if node.tag_name().name() == "term" {
                        "concept"
                    } else {
                        "id"
                    };
                    if !concepts.contains_key(node.attribute(attr).unwrap_or("")) {
                        report.diagnostic(at(file, node, Some(attr), "UNKNOWN_CONCEPT", "Concept is not defined in publisher proposals or the selected registry.", "Use an existing ID with --taxonomy PATH, or declare a publisher-owned concept in taxonomy/concepts.xml."));
                    }
                }
                "strongs" => {
                    let code = node.attribute("code").unwrap_or("");
                    let number = code.get(1..).and_then(|v| v.parse::<u32>().ok());
                    if !number.is_some_and(|n| {
                        n > 0
                            && n <= if code.starts_with('G') { 5624 } else { 8674 }
                            && code == format!("{}{n:04}", &code[..1])
                    }) {
                        report.diagnostic(at(
                            file,
                            node,
                            Some("code"),
                            "STRONGS_CODE",
                            "Strong's code is out of range or not normalized.",
                            "Use G0001–G5624 or H0001–H8674 with four-digit padding.",
                        ));
                    }
                }
                _ => {}
            }
        }
    }
    if root_kind(metadata) == "translation"
        && child(metadata.root_element(), "coverage").and_then(|n| n.attribute("mode"))
            == Some("complete")
    {
        let missing = books
            .anchors()
            .filter(|id| {
                coverage.iter().any(|(start, end)| id >= start && id <= end) && !mapped.contains(id)
            })
            .count();
        if missing > 0 {
            report.diagnostic(at("metadata.xml", child(metadata.root_element(), "coverage").unwrap(), Some("mode"), "INCOMPLETE_COVERAGE", format!("Translation declares complete coverage but {missing} canonical anchors lack verse blocks or explicit omissions."), "Add the missing mapped verses/omissions, or declare selected coverage."));
        }
    }
    for file in source.files.difference(&required) {
        report.global("UNLISTED_FILE", file, "File is not part of the declared source payload.", "List content once in index.xml, reference media from content, or remove the extra file.");
    }
    if options.deny_warnings && report.diagnostics.iter().any(|d| d.severity == "warning") {
        report.global(
            "WARNINGS_DENIED",
            "",
            "Warnings are treated as validation failures.",
            "Resolve the warnings before packaging or omit --deny-warnings for local draft work.",
        );
    }
    report.diagnostics.sort_by(|a, b| {
        (&a.file, a.line, a.column, a.code, &a.message)
            .cmp(&(&b.file, b.line, b.column, b.code, &b.message))
    });
    Ok(())
}

fn root_kind<'a>(document: &'a Document<'_>) -> &'a str {
    document.root_element().attribute("kind").unwrap_or("")
}
fn child<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
    node.children().find(|n| n.has_tag_name((NAMESPACE, name)))
}

fn target(
    source: &Snapshot,
    file: &str,
    node: Node<'_, '_>,
    attribute: &str,
    prefix: &str,
    required: &mut BTreeSet<String>,
    report: &mut Report,
) -> Option<String> {
    let path = node.attribute(attribute).unwrap_or("");
    if !safe_path(path) || !path.starts_with(prefix) {
        report.diagnostic(at(file, node, Some(attribute), "REFERENCE_PATH", format!("Reference must be a safe root-relative path under {prefix}."), "Use a package-root path such as media/figure.png or content/chapter.xml; do not use ../ or external URLs."));
        return None;
    }
    required.insert(path.into());
    if !source.files.contains(path) {
        report.diagnostic(at(
            file,
            node,
            Some(attribute),
            "MISSING_FILE",
            format!("Referenced file '{path}' does not exist in the package."),
            "Add the file at this exact case-sensitive path, or correct/remove the XML reference.",
        ));
        return None;
    }
    Some(path.into())
}

#[allow(clippy::too_many_arguments)]
fn image_check(
    source: &Snapshot,
    path: &str,
    file: &str,
    node: Node<'_, '_>,
    attribute: &str,
    cover: bool,
    declared_type: Option<&str>,
    report: &mut Report,
) -> Result<()> {
    let full = source.root.path().join(path);
    let len = fs::metadata(&full)?.len();
    let bytes_limit = if cover {
        4 * 1024 * 1024
    } else {
        16 * 1024 * 1024
    };
    if len > bytes_limit {
        report.diagnostic(at(
            file,
            node,
            Some(attribute),
            "IMAGE_SIZE",
            "Image exceeds the source asset byte limit.",
            "Reduce the cover to at most 4 MiB, or content image to at most 16 MiB.",
        ));
        return Ok(());
    }
    let bytes = fs::read(full)?;
    let decoded = (|| -> Result<_> {
        let format = image::guess_format(&bytes)?;
        anyhow::ensure!(
            matches!(format, image::ImageFormat::Png | image::ImageFormat::Jpeg),
            "only PNG/JPEG are supported"
        );
        anyhow::ensure!(
            match format {
                image::ImageFormat::Png => path.ends_with(".png"),
                _ => path.ends_with(".jpg") || path.ends_with(".jpeg"),
            },
            "file extension disagrees with decoded media type"
        );
        if let Some(declared) = declared_type {
            anyhow::ensure!(
                declared
                    == if format == image::ImageFormat::Png {
                        "image/png"
                    } else {
                        "image/jpeg"
                    },
                "declared media type disagrees with image bytes"
            );
        }
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(8192);
        limits.max_image_height = Some(8192);
        limits.max_alloc = Some(128 * 1024 * 1024);
        let mut reader = image::ImageReader::with_format(Cursor::new(&bytes), format);
        reader.limits(limits.clone());
        let (width, height) = reader.into_dimensions()?;
        anyhow::ensure!(
            u64::from(width) * u64::from(height) <= 16_777_216,
            "image exceeds 16 megapixels"
        );
        let mut reader = image::ImageReader::with_format(Cursor::new(&bytes), format);
        reader.limits(limits);
        let decoded = reader.decode()?;
        Ok((decoded.width(), decoded.height()))
    })();
    match decoded {
        Ok((width,height)) if !cover || (width,height) == (1200,1800) => {}
        Ok((width,height)) => report.diagnostic(at(file, node, Some(attribute), "COVER_DIMENSIONS", format!("Actual cover is {width}x{height}, expected 1200x1800."), "Resize or pad the cover canvas to exactly 1200x1800 without stretching artwork.")),
        Err(error) => report.diagnostic(at(file, node, Some(attribute), "IMAGE_DECODE", format!("Cannot validate image '{path}': {error}"), "Supply a valid PNG/JPEG with matching extension/media type, at most 8192 per dimension and 16 megapixels.")),
    }
    Ok(())
}

#[derive(Deserialize)]
struct Registry {
    books: Vec<Book>,
}
#[derive(Deserialize)]
struct Book {
    id: u64,
    osis: String,
    verse_counts: Vec<u64>,
}
impl Registry {
    fn resolve(&self, value: &str, end: bool) -> Option<u64> {
        let parts: Vec<_> = value.split('.').collect();
        let book = self.books.iter().find(|b| b.osis == parts[0])?;
        if parts.len() > 3 {
            return None;
        }
        let chapter: usize = if let Some(number) = parts.get(1) {
            number.parse().ok()?
        } else if end {
            book.verse_counts.len()
        } else {
            1
        };
        let count = *book.verse_counts.get(chapter.checked_sub(1)?)?;
        let verse = if let Some(number) = parts.get(2) {
            number.parse().ok()?
        } else if end {
            count
        } else {
            1
        };
        if verse == 0 || verse > count {
            return None;
        }
        Some(book.id * 1_000_000 + chapter as u64 * 1000 + verse)
    }
    fn anchors(&self) -> impl Iterator<Item = u64> + '_ {
        self.books.iter().flat_map(|b| {
            b.verse_counts
                .iter()
                .enumerate()
                .flat_map(move |(chapter, count)| {
                    (1..=*count)
                        .map(move |verse| b.id * 1_000_000 + (chapter as u64 + 1) * 1000 + verse)
                })
        })
    }
}
fn reference_range(books: &Registry, node: Node<'_, '_>) -> Option<(u64, u64)> {
    let first = node.attribute("start")?;
    let start = books.resolve(first, false)?;
    let end = books.resolve(node.attribute("end").unwrap_or(first), true)?;
    (start <= end).then_some((start, end))
}
fn reference_nodes(
    file: &str,
    doc: &Document<'_>,
    books: &Registry,
    report: &mut Report,
) -> Vec<(u64, u64)> {
    let mut ranges = vec![];
    for node in doc.descendants().filter(|n| {
        n.is_element()
            && matches!(
                n.tag_name().name(),
                "range" | "evidence" | "scripture" | "ref"
            )
    }) {
        if let Some(range) = reference_range(books, node) {
            ranges.push(range);
        } else {
            report.diagnostic(at(file, node, Some("start"), "SCRIPTURE_RANGE", "Reference is unknown, out of range, or reversed in fontes-protestant-66-v1.", "Use registry OSIS IDs such as Rom.1.3; check actual chapter/verse existence and put the earlier endpoint first."));
        }
    }
    ranges
}

fn collect_concepts<'a, 'input>(
    file: &str,
    doc: &'a Document<'input>,
    publisher: Option<&str>,
    concepts: &mut BTreeMap<String, (String, Node<'a, 'input>)>,
    report: &mut Report,
) {
    for node in doc
        .root_element()
        .children()
        .filter(|n| n.has_tag_name((NAMESPACE, "concept")))
    {
        let id = node.attribute("id").unwrap_or("");
        let prefix = publisher
            .map(|p| format!("urn:fontes:publisher:{p}:concept:"))
            .unwrap_or_else(|| "urn:fontes:concept:".into());
        if !id.starts_with(&prefix) || id.len() <= prefix.len() || id.contains(char::is_whitespace)
        {
            report.diagnostic(at(file,node,Some("id"),"CONCEPT_AUTHORITY",format!("Concept ID must use the namespace {prefix}<local-name>."),"Define only concepts owned by this publisher; reuse central concepts by reference, not by redefining them."));
        }
        if concepts.insert(id.into(), (file.into(), node)).is_some() {
            report.diagnostic(at(
                file,
                node,
                Some("id"),
                "DUPLICATE_CONCEPT",
                "Concept identity is declared more than once.",
                "Keep one definition per ID and do not redefine a selected registry concept.",
            ));
        }
        let mut labels = HashSet::new();
        let mut languages = HashSet::new();
        let mut preferred = HashMap::<String, usize>::new();
        for label in node
            .children()
            .filter(|n| n.has_tag_name((NAMESPACE, "label")))
        {
            let language = label
                .attribute("language")
                .unwrap_or("")
                .to_ascii_lowercase();
            languages.insert(language.clone());
            if label.attribute("kind") == Some("preferred") {
                *preferred.entry(language.clone()).or_default() += 1;
            }
            if !labels.insert((language, label.text().unwrap_or("").to_owned())) {
                report.diagnostic(at(
                    file,
                    label,
                    None,
                    "DUPLICATE_LABEL",
                    "Concept label is repeated for this language.",
                    "Remove duplicate labels; choose one preferred label per language.",
                ));
            }
        }
        if languages
            .iter()
            .any(|language| preferred.get(language) != Some(&1))
        {
            report.diagnostic(at(
                file,
                node,
                None,
                "PREFERRED_LABEL",
                "Each included language needs exactly one preferred label.",
                "Add one preferred label for each language and mark other labels alternative.",
            ));
        }
    }
}

fn concept_graph(concepts: &BTreeMap<String, (String, Node<'_, '_>)>, report: &mut Report) {
    let mut pending: BTreeMap<&str, usize> = concepts.keys().map(|id| (id.as_str(), 0)).collect();
    let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
    for (id, (file, node)) in concepts {
        let mut seen = HashSet::new();
        for link in node
            .children()
            .filter(|n| n.is_element() && matches!(n.tag_name().name(), "broader" | "related"))
        {
            let target = link.attribute("id").unwrap_or("");
            if !concepts.contains_key(target)
                || target == id
                || !seen.insert((link.tag_name().name(), target))
            {
                report.diagnostic(at(
                    file,
                    link,
                    Some("id"),
                    "CONCEPT_RELATION",
                    "Concept relation has an unknown, self, or duplicate target.",
                    "Reference a distinct declared concept and list each relationship once.",
                ));
            } else if link.tag_name().name() == "broader" {
                *pending.get_mut(id.as_str()).unwrap() += 1;
                children.entry(target).or_default().push(id);
            }
        }
    }
    let mut queue: VecDeque<_> = pending
        .iter()
        .filter_map(|(id, count)| (*count == 0).then_some(*id))
        .collect();
    while let Some(id) = queue.pop_front() {
        for child in children.get(id).into_iter().flatten() {
            let count = pending.get_mut(child).unwrap();
            *count -= 1;
            if *count == 0 {
                queue.push_back(child);
            }
        }
    }
    for (id, count) in pending {
        if count > 0 {
            let (file, node) = &concepts[id];
            report.diagnostic(at(file,*node,Some("id"),"CONCEPT_CYCLE","Concept participates in or depends on a broader relationship cycle.","Remove the circular parent relationship; broader links must form a directed acyclic graph."));
        }
    }
}
