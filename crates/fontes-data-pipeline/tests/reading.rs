use ed25519_dalek::{Signer, SigningKey};
use fontes_data_contracts::{Channel, ReleaseManifest, SourceLock};
use fontes_data_pipeline::{
    Mapping, Registry, build, digest, parse_xml,
    release::{TrustKey, verify_manifest, verify_package},
    write_json,
};
use std::{fs, io::Write, path::Path};

const DOMAIN: &[u8] = b"FONTES-DATA-MANIFEST-V1\n";
const COMMIT: &str = "1234567890123456789012345678901234567890";

fn mapping(source: &str) -> Mapping {
    let value: serde_json::Value =
        serde_json::from_str(include_str!("../../../data/mappings/reading-v1.json")).unwrap();
    serde_json::from_value(value["sources"][source].clone()).unwrap()
}

fn xml(web: bool) -> String {
    let registry = Registry::load().unwrap();
    let omissions = ["LUK.17.36", "ACT.8.37", "ACT.15.34", "ACT.24.7"];
    let mut xml = String::from("<?xml version=\"1.0\"?><verseFile>");
    for book in registry.books {
        for (chapter, count) in book.verse_counts.iter().enumerate() {
            for verse in 1..=*count {
                let mut reference = format!("{}.{}.{verse}", book.usfm, chapter + 1);
                if web && book.usfm == "ROM" && chapter == 15 && verse >= 25 {
                    reference = format!("ROM.14.{}", verse - 1);
                }
                let parts: Vec<_> = reference.split('.').collect();
                let text = if web && omissions.contains(&reference.as_str()) {
                    String::new()
                } else if book.usfm == "ROM" && chapter == 15 && verse == 25 {
                    "Now to him who is able to establish you".into()
                } else if book.usfm == "JHN" && chapter == 2 && verse == 16 {
                    "For God so loved the world".into()
                } else {
                    format!("Fixture {} {} {verse} &amp; light.", book.usfm, chapter + 1)
                };
                xml.push_str(&format!(
                    "<v b=\"{}\" c=\"{}\" v=\"{}\"> {text} </v>",
                    parts[0], parts[1], parts[2]
                ));
            }
        }
    }
    if web {
        xml.push_str("<v b=\"ROM\" c=\"16\" v=\"25\"/>");
    }
    xml.push_str("</verseFile>");
    xml
}

#[test]
fn complete_versification_mapping_preserves_omissions_and_decodes_entities() {
    let (rows, report) = parse_xml(
        &xml(true),
        "web-classic",
        &Registry::load().unwrap(),
        &mapping("web-classic"),
    )
    .unwrap();
    assert_eq!(rows.len(), 31102);
    assert_eq!(report.empty_verses.len(), 4);
    assert_eq!(report.ignored_placeholders, ["ROM.16.25"]);
    assert_eq!(report.reference_overrides.len(), 3);
    assert_eq!(
        rows.iter()
            .find(|r| r.canonical_ref_id == 45016025)
            .unwrap()
            .source_ref,
        "ROM.14.24"
    );
    assert!(rows[0].text.contains("& light"));
    assert_eq!(rows.iter().filter(|r| r.source_omission).count(), 4);
}

#[test]
fn importer_rejects_duplicates_missing_verses_unreviewed_blanks_and_dtds() {
    let registry = Registry::load().unwrap();
    let original = xml(false);
    let duplicate = original.replace(
        "</verseFile>",
        "<v b=\"GEN\" c=\"1\" v=\"1\">duplicate</v></verseFile>",
    );
    assert!(parse_xml(&duplicate, "kjv-base", &registry, &Mapping::default()).is_err());
    let missing = original.replace("b=\"GEN\" c=\"1\" v=\"1\"", "b=\"GEN\" c=\"1\" v=\"999\"");
    assert!(parse_xml(&missing, "kjv-base", &registry, &Mapping::default()).is_err());
    let blank = original.replacen("Fixture GEN 1 1 &amp; light.", "", 1);
    assert!(parse_xml(&blank, "kjv-base", &registry, &Mapping::default()).is_err());
    assert!(
        parse_xml(
            "<!DOCTYPE verseFile [<!ENTITY bad 'bad'>]><verseFile/>",
            "kjv-base",
            &registry,
            &Mapping::default()
        )
        .is_err()
    );
}

fn zip_xml(dest: &Path, name: &str, xml: &str) {
    let mut writer = zip::ZipWriter::new(fs::File::create(dest).unwrap());
    writer
        .start_file(
            name,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated),
        )
        .unwrap();
    writer.write_all(xml.as_bytes()).unwrap();
    writer.finish().unwrap();
}

fn fixture(root: &Path) {
    fs::create_dir_all(root.join("data/mappings")).unwrap();
    fs::create_dir_all(root.join("data/canonical")).unwrap();
    fs::create_dir_all(root.join("cache")).unwrap();
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for name in [
        "Cargo.lock",
        "rust-toolchain.toml",
        "data/profiles.json",
        "data/mappings/reading-v1.json",
        "data/canonical/registry-v1.json",
    ] {
        fs::copy(repo.join(name), root.join(name)).unwrap();
    }
    let mut lock: SourceLock =
        serde_json::from_str(include_str!("../../../data/sources.lock.json")).unwrap();
    lock.sources
        .retain(|s| ["kjv-base", "web-classic"].contains(&s.source_id.as_str()));
    for source in &mut lock.sources {
        let tmp = root.join(format!("{}.zip", source.source_id));
        let web = source.source_id == "web-classic";
        zip_xml(
            &tmp,
            if web {
                "eng-web_vpl.xml"
            } else {
                "eng-kjv_vpl.xml"
            },
            &xml(web),
        );
        let bytes = fs::read(&tmp).unwrap();
        source.sha256 = digest(&bytes);
        source.bytes = bytes.len() as u64;
        fs::rename(
            tmp,
            root.join("cache").join(format!("{}.zip", source.sha256)),
        )
        .unwrap();
    }
    write_json(&root.join("data/sources.lock.json"), &lock).unwrap();
}

#[test]
fn full_reading_build_is_reproducible_and_signed_package_detects_tampering() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    let output = root.join("first");
    let a = build(
        root,
        "reading",
        &root.join("cache"),
        &output,
        COMMIT,
        true,
        None,
    )
    .unwrap();
    let b = build(
        root,
        "reading",
        &root.join("cache"),
        &root.join("second"),
        COMMIT,
        true,
        None,
    )
    .unwrap();
    assert_eq!(a.database.verse_text_count, 62204);
    assert_eq!(
        a.profile_artifact.sqlite_sha256,
        b.profile_artifact.sqlite_sha256
    );
    assert_eq!(
        a.profile_artifact.compressed_sha256,
        b.profile_artifact.compressed_sha256
    );
    let key = SigningKey::from_bytes(&[7; 32]);
    let trust = TrustKey {
        key_id: "fixture-key".into(),
        public_key_hex: hex::encode(key.verifying_key().to_bytes()),
    };
    let trust_path = root.join("trust.json");
    write_json(&trust_path, &trust).unwrap();
    let release = ReleaseManifest {
        manifest_version: 1,
        update_protocol_version: 1,
        content_schema_version: 1,
        canonical_registry_version: 1,
        repository: "example/fontes".into(),
        release_tag: "data-v1-stable-1".into(),
        channel: Channel::Stable,
        sequence: 1,
        published_at: "2026-10-16T00:00:00Z".into(),
        signing_key_id: "fixture-key".into(),
        builder_commit: COMMIT.into(),
        source_lock_sha256: a.source_lock_sha256,
        profiles: vec![a.profile_artifact],
        sources: a.sources,
    };
    write_json(&output.join("release-manifest.json"), &release).unwrap();
    let bytes = fs::read(output.join("release-manifest.json")).unwrap();
    let sig = key.sign(&[DOMAIN, &bytes].concat()).to_bytes();
    fs::write(output.join("release-manifest.json.sig"), sig).unwrap();
    verify_package(&output, &trust_path, "example/fontes").unwrap();
    assert!(verify_manifest(&bytes, &sig, &trust, "another/repo").is_err());
    let mut tampered = bytes.clone();
    tampered[0] ^= 1;
    assert!(verify_manifest(&tampered, &sig, &trust, "example/fontes").is_err());
    let mut wrong_key = trust;
    wrong_key.public_key_hex =
        hex::encode(SigningKey::from_bytes(&[8; 32]).verifying_key().to_bytes());
    assert!(verify_manifest(&bytes, &sig, &wrong_key, "example/fontes").is_err());
    let artifact = output.join(&release.profiles[0].asset);
    let mut damaged = fs::read(&artifact).unwrap();
    damaged[0] ^= 1;
    fs::write(artifact, damaged).unwrap();
    assert!(verify_package(&output, &trust_path, "example/fontes").is_err());
    let cache = root.join("cache");
    let entry = fs::read_dir(&cache)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::write(entry, b"bad cache").unwrap();
    assert!(
        build(
            root,
            "reading",
            &cache,
            &root.join("corrupt"),
            COMMIT,
            true,
            None
        )
        .is_err()
    );
    assert!(
        build(
            root,
            "study",
            &cache,
            &root.join("unsupported"),
            COMMIT,
            true,
            None
        )
        .is_err()
    );
}

#[test]
fn signed_manifest_supports_both_profiles_and_checks_each_profiles_claims() {
    let mut manifest: ReleaseManifest =
        serde_json::from_str(include_str!("../../../data/examples/release-manifest.json")).unwrap();
    let lock: SourceLock =
        serde_json::from_str(include_str!("../../../data/sources.lock.json")).unwrap();
    manifest.sources = lock
        .sources
        .iter()
        .map(|s| fontes_data_contracts::SourceSnapshot {
            source_id: s.source_id.clone(),
            sha256: s.sha256.clone(),
            adapter_version: s.adapter.clone(),
            rights_review_id: s.rights_review_id.clone(),
        })
        .collect();
    let mut study = manifest.profiles[0].clone();
    study.profile = "study".into();
    study.asset = "fontes-study.sqlite.zst".into();
    study.capabilities = fontes_data_pipeline::profile_capabilities("study")
        .unwrap()
        .to_vec();
    study.source_ids = fontes_data_pipeline::profile_sources("study")
        .unwrap()
        .iter()
        .map(|s| s.to_string())
        .collect();
    manifest.profiles.push(study);
    let key = SigningKey::from_bytes(&[9; 32]);
    let trust = TrustKey {
        key_id: manifest.signing_key_id.clone(),
        public_key_hex: hex::encode(key.verifying_key().to_bytes()),
    };
    let check = |manifest: &ReleaseManifest| {
        let bytes = serde_json::to_vec(manifest).unwrap();
        let signed = [DOMAIN, bytes.as_slice()].concat();
        verify_manifest(
            &bytes,
            &key.sign(&signed).to_bytes(),
            &trust,
            &manifest.repository,
        )
    };
    assert_eq!(check(&manifest).unwrap().profiles.len(), 2);
    manifest.profiles[1].capabilities.pop();
    assert!(check(&manifest).is_err());
    manifest.profiles[1].capabilities = fontes_data_pipeline::profile_capabilities("study")
        .unwrap()
        .to_vec();
    manifest.profiles[1]
        .source_ids
        .retain(|s| s != "strongs-hebrew");
    assert!(check(&manifest).is_err());
}
