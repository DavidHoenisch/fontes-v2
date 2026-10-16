use fontes_resource::{ValidationOptions, pack, validate};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn copy(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if entry.path().is_dir() {
            copy(&entry.path(), &target.join(entry.file_name()));
        } else {
            fs::copy(entry.path(), target.join(entry.file_name())).unwrap();
        }
    }
}
fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    copy(
        &root().join("specs/resource-packages/v1/examples/romans-study"),
        dir.path(),
    );
    dir
}
fn options() -> ValidationOptions {
    ValidationOptions {
        taxonomy: Some(root().join("specs/resource-packages/v1/taxonomy.example.xml")),
        deny_warnings: false,
    }
}
fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fontes-resource"))
        .args(args)
        .output()
        .unwrap()
}
fn replace(path: &Path, old: &str, new: &str) {
    let text = fs::read_to_string(path).unwrap();
    assert!(text.contains(old));
    fs::write(path, text.replace(old, new)).unwrap();
}

#[test]
fn validates_actual_example_and_requires_explicit_concept_registry() {
    let source = fixture();
    let report = validate(source.path(), &options()).unwrap();
    assert!(report.valid, "{}", report.text());
    assert!(report.diagnostics.is_empty());
    let no_registry = validate(source.path(), &ValidationOptions::default()).unwrap();
    assert!(!no_registry.valid);
    assert!(
        no_registry
            .diagnostics
            .iter()
            .any(|d| d.code == "UNKNOWN_CONCEPT")
    );
}

#[test]
fn missing_image_points_to_exact_xml_attribute_and_aggregates_semantic_errors() {
    let source = fixture();
    let file = source.path().join("content/rom/001.xml");
    replace(
        &file,
        "</document>",
        "  <block id=\"image-block\" kind=\"paragraph\">\n    <text><image src=\"media/missing.png\" alt=\"Diagram\"/></text>\n    <annotations><scripture start=\"Rom.1.99\" relation=\"cites\" basis=\"author\"/><strongs code=\"G9999\" relation=\"uses\"/></annotations>\n  </block>\n</document>",
    );
    let text = fs::read_to_string(&file).unwrap();
    let (line, content) = text
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains("media/missing.png"))
        .unwrap();
    let report = validate(source.path(), &options()).unwrap();
    let image = report
        .diagnostics
        .iter()
        .find(|d| d.code == "MISSING_FILE")
        .unwrap();
    assert_eq!(image.file, "content/rom/001.xml");
    assert_eq!(image.line, Some(line as u32 + 1));
    assert_eq!(image.column, Some(content.find("src=").unwrap() as u32 + 1));
    assert!(image.help.contains("Add the file"));
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "SCRIPTURE_RANGE")
    );
    assert!(report.diagnostics.iter().any(|d| d.code == "STRONGS_CODE"));
}

#[test]
fn image_references_require_media_and_actual_decodable_files() {
    for (reference, bytes, code) in [
        ("assets/cover.png", None, "REFERENCE_PATH"),
        (
            "media/fake.png",
            Some(b"not a PNG".as_slice()),
            "IMAGE_DECODE",
        ),
    ] {
        let source = fixture();
        let file = source.path().join("content/rom/001.xml");
        replace(
            &file,
            "</document>",
            &format!(
                "<block id=\"image-block\" kind=\"paragraph\"><text><image src=\"{reference}\" alt=\"Diagram\"/></text></block></document>"
            ),
        );
        if let Some(bytes) = bytes {
            fs::create_dir_all(source.path().join("media")).unwrap();
            fs::write(source.path().join(reference), bytes).unwrap();
        }
        let report = validate(source.path(), &options()).unwrap();
        assert!(
            report.diagnostics.iter().any(|d| d.code == code),
            "{}",
            report.text()
        );
    }
}

#[test]
fn rejects_unsupported_xml_attributes_elements_and_invalid_types_with_locations() {
    for (before, after) in [
        ("kind=\"paragraph\"", "kind=\"mystery\""),
        (
            "kind=\"paragraph\"",
            "kind=\"paragraph\" unexpected=\"yes\"",
        ),
        ("<text>This example", "<text><script/>This example"),
    ] {
        let source = fixture();
        replace(&source.path().join("content/rom/001.xml"), before, after);
        let report = validate(source.path(), &options()).unwrap();
        assert!(!report.valid);
        assert!(
            report.diagnostics.iter().any(|d| d.code == "XML_SCHEMA"
                && d.file == "content/rom/001.xml"
                && d.line.is_some()
                && !d.help.is_empty()),
            "{}",
            report.text()
        );
    }
}

#[test]
fn rejects_dtd_entities_and_malformed_xml() {
    for text in [
        "<!DOCTYPE document [<!ENTITY x SYSTEM 'file:///etc/passwd'>]><document>&x;</document>",
        "<document><broken></document>",
    ] {
        let source = fixture();
        fs::write(source.path().join("content/rom/001.xml"), text).unwrap();
        let report = validate(source.path(), &options()).unwrap();
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.code == "XML_PARSE" && d.line.is_some() && d.column.is_some())
        );
    }
    let source = fixture();
    replace(
        &source.path().join("content/rom/001.xml"),
        "<document ",
        "<?xml-stylesheet href=\"https://example.invalid/style.xsl\"?><document ",
    );
    assert!(
        validate(source.path(), &options())
            .unwrap()
            .diagnostics
            .iter()
            .any(|d| d.code == "XML_INSTRUCTION" && d.line.is_some())
    );
}

#[test]
fn checks_inventory_index_identity_and_ids_across_files() {
    for mutation in 0..4 {
        let source = fixture();
        let expected = match mutation {
            0 => {
                replace(
                    &source.path().join("index.xml"),
                    "content/rom/001.xml",
                    "content/rom/missing.xml",
                );
                "MISSING_FILE"
            }
            1 => {
                replace(
                    &source.path().join("content/rom/001.xml"),
                    "resource-id=\"org.example.romans-study\"",
                    "resource-id=\"org.example.other-work\"",
                );
                "DOCUMENT_IDENTITY"
            }
            2 => {
                fs::write(source.path().join("extra.txt"), "unused").unwrap();
                "UNLISTED_FILE"
            }
            _ => {
                replace(
                    &source.path().join("content/rom/001.xml"),
                    "id=\"rom-001-heading\"",
                    "id=\"rom-intro\"",
                );
                "DUPLICATE_ID"
            }
        };
        let report = validate(source.path(), &options()).unwrap();
        assert!(
            report.diagnostics.iter().any(|d| d.code == expected),
            "{}",
            report.text()
        );
    }
}

#[test]
fn validates_cover_bytes_not_only_declared_dimensions() {
    let source = fixture();
    image::RgbImage::new(12, 18)
        .save(source.path().join("assets/cover.png"))
        .unwrap();
    let report = validate(source.path(), &options()).unwrap();
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "COVER_DIMENSIONS" && d.file == "metadata.xml" && d.line.is_some())
    );
}

#[test]
fn rejects_proposal_namespace_hijack_and_parent_cycles() {
    let source = fixture();
    replace(
        &source.path().join("taxonomy/concepts.xml"),
        "id=\"urn:fontes:publisher:org.example:concept:messianic-ancestry\"",
        "id=\"urn:fontes:concept:unapproved\"",
    );
    let report = validate(source.path(), &options()).unwrap();
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "CONCEPT_AUTHORITY")
    );
    let source = fixture();
    let path = source.path().join("taxonomy/concepts.xml");
    replace(
        &path,
        "<related id=\"urn:fontes:concept:incarnation\"/>",
        "<broader id=\"urn:fontes:publisher:org.example:concept:second\"/>",
    );
    replace(
        &path,
        "</concepts>",
        "<concept id=\"urn:fontes:publisher:org.example:concept:second\"><label language=\"en\" kind=\"preferred\">Second</label><scope-note>Cycle example</scope-note><broader id=\"urn:fontes:publisher:org.example:concept:messianic-ancestry\"/></concept></concepts>",
    );
    let report = validate(source.path(), &options()).unwrap();
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "CONCEPT_CYCLE" && d.line.is_some())
    );
}

#[test]
fn scaffold_supports_metadata_escaping_kinds_and_refuses_overwrite() {
    let temp = tempfile::tempdir().unwrap();
    for kind in [
        "translation",
        "commentary",
        "lexicon",
        "word-study",
        "study-guide",
        "devotional",
        "book",
    ] {
        let destination = temp.path().join(kind);
        let output = cli(&[
            "init",
            destination.to_str().unwrap(),
            "--id",
            "org.example.new-book",
            "--publisher-id",
            "org.example",
            "--kind",
            kind,
            "--title",
            "Grace & <Truth> — χάρις",
            "--author",
            "A & B",
            "--author",
            "Another Author",
            "--publisher",
            "Example Press",
            "--publication-date",
            "2026-10-15",
            "--edition",
            "First edition",
            "--start",
            "Rom.1.3",
        ]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = fs::read_to_string(destination.join("metadata.xml")).unwrap();
        let doc = roxmltree::Document::parse(&text).unwrap();
        assert_eq!(
            doc.descendants()
                .find(|n| n.has_tag_name("title"))
                .unwrap()
                .text(),
            Some("Grace & <Truth> — χάρις")
        );
        assert_eq!(
            doc.descendants()
                .filter(|n| n.has_tag_name("contributor"))
                .count(),
            2
        );
        assert!(destination.join("media").is_dir());
        assert!(destination.join("taxonomy").is_dir());
        let report = validate(&destination, &ValidationOptions::default()).unwrap();
        assert!(report.valid, "{}", report.text());
        let strict = validate(
            &destination,
            &ValidationOptions {
                deny_warnings: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!strict.valid);
        assert!(
            strict
                .diagnostics
                .iter()
                .any(|d| d.code == "WARNINGS_DENIED")
        );
        let again = cli(&[
            "init",
            destination.to_str().unwrap(),
            "--id",
            "org.example.new-book",
            "--publisher-id",
            "org.example",
        ]);
        assert!(!again.status.success());
        assert_eq!(
            fs::read_to_string(destination.join("metadata.xml")).unwrap(),
            text
        );
    }
}

#[test]
fn bad_scaffold_parameters_leave_no_package_and_cli_json_is_machine_readable() {
    let temp = tempfile::tempdir().unwrap();
    let destination = temp.path().join("bad");
    let bad = cli(&[
        "init",
        destination.to_str().unwrap(),
        "--id",
        "org.example.book",
        "--publisher-id",
        "org.example",
        "--publication-date",
        "2026-02-30",
    ]);
    assert!(!bad.status.success());
    assert!(!destination.exists());
    let source = fixture();
    replace(
        &source.path().join("content/rom/001.xml"),
        "Rom.1.3",
        "Rom.1.999",
    );
    let json = cli(&[
        "validate",
        source.path().to_str().unwrap(),
        "--taxonomy",
        options().taxonomy.unwrap().to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert_eq!(json.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(report["report_version"], 1);
    assert_eq!(report["valid"], false);
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "SCRIPTURE_RANGE" && d["line"].is_u64() && d["column"].is_u64())
    );
}

#[test]
fn complete_translation_requires_each_declared_anchor_and_explicit_omissions() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("translation");
    assert!(
        cli(&[
            "init",
            source.to_str().unwrap(),
            "--id",
            "org.example.translation",
            "--publisher-id",
            "org.example",
            "--kind",
            "translation",
            "--start",
            "Rom.1.1"
        ])
        .status
        .success()
    );
    replace(
        &source.join("metadata.xml"),
        "mode=\"selected\"",
        "mode=\"complete\"",
    );
    replace(
        &source.join("metadata.xml"),
        "end=\"Rom.1.1\"",
        "end=\"Rom.1.2\"",
    );
    let missing = validate(&source, &ValidationOptions::default()).unwrap();
    assert!(
        missing
            .diagnostics
            .iter()
            .any(|d| d.code == "INCOMPLETE_COVERAGE")
    );
    replace(
        &source.join("content/001.xml"),
        "</document>",
        "<block id=\"omitted\" kind=\"verse\" status=\"omitted\" source-ref=\"Rom.1.2\"><annotations><scripture start=\"Rom.1.2\" relation=\"text-of\" basis=\"source\"/></annotations></block></document>",
    );
    assert!(
        validate(&source, &ValidationOptions::default())
            .unwrap()
            .valid
    );
}

#[test]
fn packs_validated_bytes_reproducibly_and_refuses_existing_output() {
    let source = fixture();
    let output = tempfile::tempdir().unwrap();
    let a = output
        .path()
        .join("a/org.example.romans-study--1.0.0.fontes-source.zip");
    let b = output
        .path()
        .join("b/org.example.romans-study--1.0.0.fontes-source.zip");
    let first = pack(source.path(), &a, &options()).unwrap();
    let second = pack(source.path(), &b, &options()).unwrap();
    assert_eq!(first.sha256, second.sha256);
    assert_eq!(fs::read(&a).unwrap(), fs::read(&b).unwrap());
    assert!(validate(&a, &options()).unwrap().valid);
    let bytes = fs::read(&a).unwrap();
    assert!(pack(source.path(), &a, &options()).is_err());
    assert_eq!(fs::read(&a).unwrap(), bytes);
    replace(
        &source.path().join("index.xml"),
        "content/rom/001.xml",
        "content/rom/missing.xml",
    );
    let invalid = output
        .path()
        .join("c/org.example.romans-study--1.0.0.fontes-source.zip");
    assert!(pack(source.path(), &invalid, &options()).is_err());
    assert!(!invalid.exists());
}

#[test]
fn rejects_zip_traversal_duplicate_case_paths_and_special_entries() {
    let temp = tempfile::tempdir().unwrap();
    for (index, names, expected) in [
        (0, vec!["../escaped.txt"], "UNSAFE_PATH"),
        (1, vec!["media/A.png", "media/a.png"], "DUPLICATE_PATH"),
        (2, vec!["content\\bad.xml"], "UNSAFE_PATH"),
    ] {
        let path = temp.path().join(format!("bad-{index}.zip"));
        let mut zip = zip::ZipWriter::new(fs::File::create(&path).unwrap());
        for name in names {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"test").unwrap();
        }
        zip.finish().unwrap();
        let report = validate(&path, &ValidationOptions::default()).unwrap();
        assert!(!report.valid);
        assert!(
            report.diagnostics.iter().any(|d| d.code == expected),
            "{}",
            report.text()
        );
    }
    assert!(!temp.path().join("escaped.txt").exists());
}

#[cfg(unix)]
#[test]
fn rejects_directory_symlinks_without_following_them() {
    let source = fixture();
    std::os::unix::fs::symlink("/etc/passwd", source.path().join("content/private.xml")).unwrap();
    let report = validate(source.path(), &options()).unwrap();
    assert!(report.diagnostics.iter().any(|d| d.code == "FILE_TYPE"));
}

#[test]
fn rejects_zip_symlinks_and_actual_expanded_xml_limit() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("symlink.zip");
    let mut zip = zip::ZipWriter::new(fs::File::create(&path).unwrap());
    zip.add_symlink(
        "content/private.xml",
        "/etc/passwd",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.finish().unwrap();
    assert!(
        validate(&path, &ValidationOptions::default())
            .unwrap()
            .diagnostics
            .iter()
            .any(|d| d.code == "ARCHIVE_ENTRY")
    );

    let path = temp.path().join("oversized.zip");
    let mut zip = zip::ZipWriter::new(fs::File::create(&path).unwrap());
    zip.start_file(
        "content/huge.xml",
        zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated),
    )
    .unwrap();
    zip.write_all(&vec![b'x'; 16 * 1024 * 1024 + 1]).unwrap();
    zip.finish().unwrap();
    assert!(fs::metadata(&path).unwrap().len() < 100_000);
    let report = validate(&path, &ValidationOptions::default()).unwrap();
    assert!(report.diagnostics.iter().any(|d| d.code == "SIZE_LIMIT"));
}

#[test]
fn checks_xml_depth_and_machine_id_whitespace() {
    let source = fixture();
    replace(
        &source.path().join("content/rom/001.xml"),
        "<text>This example",
        &format!(
            "<text>{}deep{}</text><text>This example",
            "<em>".repeat(33),
            "</em>".repeat(33)
        ),
    );
    assert!(
        validate(source.path(), &options())
            .unwrap()
            .diagnostics
            .iter()
            .any(|d| d.code == "XML_DEPTH")
    );
    let source = fixture();
    replace(
        &source.path().join("content/rom/001.xml"),
        "id=\"rom-001-heading\"",
        "id=\" rom-001-heading \"",
    );
    let report = validate(source.path(), &options()).unwrap();
    assert!(!report.valid);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "IDENTITY_WHITESPACE"),
        "{}",
        report.text()
    );
}

#[test]
fn accepts_referenced_media_and_union_of_declared_coverage_ranges() {
    let source = fixture();
    fs::create_dir_all(source.path().join("media")).unwrap();
    image::RgbImage::new(20, 20)
        .save(source.path().join("media/diagram.png"))
        .unwrap();
    replace(
        &source.path().join("content/rom/001.xml"),
        "</document>",
        "<block id=\"diagram-block\" kind=\"paragraph\"><text><image src=\"media/diagram.png\" alt=\"Diagram\"/></text></block></document>",
    );
    let report = validate(source.path(), &options()).unwrap();
    assert!(report.valid, "{}", report.text());

    let temp = tempfile::tempdir().unwrap();
    let translation = temp.path().join("translation");
    assert!(
        cli(&[
            "init",
            translation.to_str().unwrap(),
            "--id",
            "org.example.translation",
            "--publisher-id",
            "org.example",
            "--kind",
            "translation",
            "--start",
            "Rom.1.1",
            "--end",
            "Rom.1.2"
        ])
        .status
        .success()
    );
    replace(
        &translation.join("metadata.xml"),
        "<range start=\"Rom.1.1\" end=\"Rom.1.2\"/>",
        "<range start=\"Rom.1.1\"/><range start=\"Rom.1.2\"/>",
    );
    replace(
        &translation.join("metadata.xml"),
        "mode=\"selected\"",
        "mode=\"complete\"",
    );
    let report = validate(&translation, &ValidationOptions::default()).unwrap();
    assert!(report.valid, "{}", report.text());
}
