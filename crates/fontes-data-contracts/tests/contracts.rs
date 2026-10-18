use fontes_data_contracts::{
    CONTENT_SCHEMA_SQL, ReleaseManifest, SQLITE_APPLICATION_ID, SourceLock, USER_SCHEMA_SQL,
    canonical_ref_id,
};
use rusqlite::Connection;
use serde_json::Value;
use std::collections::BTreeSet;

fn manifest() -> ReleaseManifest {
    serde_json::from_str(include_str!("../../../data/examples/release-manifest.json")).unwrap()
}

#[test]
fn source_lock_uses_inspected_hashes_and_rejects_unapproved_rights() {
    let lock: SourceLock =
        serde_json::from_str(include_str!("../../../data/sources.lock.json")).unwrap();
    lock.validate().unwrap();
    let evidence: Value = serde_json::from_str(include_str!(
        "../../../docs/research/source-validation.json"
    ))
    .unwrap();
    for source in &lock.sources {
        let file = evidence["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["resource_id"] == source.source_id)
            .unwrap();
        assert_eq!(file["sha256"].as_str().unwrap(), source.sha256);
        assert_eq!(file["downloaded_bytes"].as_u64().unwrap(), source.bytes);
    }
    let mut value = serde_json::to_value(lock).unwrap();
    value["sources"][0]["rights_class"] = "annotation_scope_unresolved".into();
    assert!(serde_json::from_value::<SourceLock>(value).is_err());
}

fn content() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch(CONTENT_SCHEMA_SQL).unwrap();
    db.execute_batch(
        "INSERT INTO books VALUES (1, 'Gen', 'Genesis', 'OT', 1);
         INSERT INTO canonical_refs VALUES (1001001, 1, 1, 1);
         INSERT INTO canonical_refs VALUES (1001002, 1, 1, 2);
         INSERT INTO canonical_refs VALUES (1002001, 1, 2, 1);
         INSERT INTO source_snapshots VALUES (
             'kjv-fixture', 'kjv-base', 'fixture-sha', 'https://example.com', NULL, 'review-1', 'v1');
         INSERT INTO text_witnesses VALUES ('kjv', 'KJV', 'en', '1769', 'KJV', 'kjv-fixture');
         INSERT INTO translations VALUES ('kjv', 'KJV', 'King James Version', 'en', 'kjv');
         INSERT INTO source_segments VALUES ('seg-1', 'kjv-fixture', 'kjv', 'Gen.1.1', 'Gen.1.1');
         INSERT INTO segment_refs VALUES ('seg-1', 1001001, 'exact');
         INSERT INTO verse_texts VALUES ('kjv:Gen.1.1', 'kjv', 'seg-1', 'Fixture verse text', NULL);",
    )
    .unwrap();
    db
}

#[test]
fn permanent_registry_has_expected_coverage_and_unique_ids() {
    let registry: Value =
        serde_json::from_str(include_str!("../../../data/canonical/registry-v1.json")).unwrap();
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch(CONTENT_SCHEMA_SQL).unwrap();
    let mut ids = BTreeSet::new();
    let mut chapters = 0;
    let books = registry["books"].as_array().unwrap();
    for book in books {
        let id = book["id"].as_u64().unwrap() as u16;
        db.execute(
            "INSERT INTO books VALUES (?1, ?2, ?3, ?4, ?1)",
            rusqlite::params![
                id,
                book["osis"].as_str().unwrap(),
                book["name"].as_str().unwrap(),
                book["testament"].as_str().unwrap()
            ],
        )
        .unwrap();
        for (chapter, count) in book["verse_counts"].as_array().unwrap().iter().enumerate() {
            chapters += 1;
            for verse in 1..=count.as_u64().unwrap() as u16 {
                let ref_id = canonical_ref_id(id, chapter as u16 + 1, verse).unwrap();
                assert!(ids.insert(ref_id));
                db.execute(
                    "INSERT INTO canonical_refs VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![ref_id, id, chapter + 1, verse],
                )
                .unwrap();
            }
        }
    }
    assert_eq!((books.len(), chapters, ids.len()), (66, 1189, 31102));
    assert_eq!(canonical_ref_id(43, 3, 16).unwrap(), 43_003_016);
    assert!(canonical_ref_id(1, 0, 1).is_err());
    assert!(canonical_ref_id(1, 1, 1000).is_err());
}

#[test]
fn release_fixture_and_fixed_asset_location_are_valid() {
    let release = manifest();
    release.validate().unwrap();
    assert_eq!(
        release.asset_url("reading").unwrap(),
        "https://github.com/DavidHoenisch/fontes-v2/releases/download/data-v1-stable-1/fontes-reading.sqlite.zst"
    );
    assert!(release.asset_url("missing").is_none());
}

#[test]
fn release_rejects_path_traversal_unknown_sources_and_duplicate_profiles() {
    let mut release = manifest();
    release.profiles[0].asset = "../user.sqlite".into();
    assert!(release.validate().is_err());
    let mut release = manifest();
    release.profiles[0].source_ids.push("not-in-release".into());
    assert!(release.validate().is_err());
    let mut release = manifest();
    release.profiles.push(release.profiles[0].clone());
    assert!(release.validate().is_err());
}

#[test]
fn release_rejects_incompatible_protocol_and_misleading_sequence() {
    let mut release = manifest();
    release.update_protocol_version = 999;
    assert!(release.validate().is_err());
    let mut release = manifest();
    release.sequence = 2;
    assert!(release.validate().is_err());
    let mut release = manifest();
    release.profiles[0].sqlite_sha256 = "not-a-digest".into();
    assert!(release.validate().is_err());
    let mut value = serde_json::to_value(manifest()).unwrap();
    value["unexpected"] = true.into();
    assert!(serde_json::from_value::<ReleaseManifest>(value).is_err());
}

#[test]
fn schema_rejects_unstable_or_dangling_references() {
    let db = content();
    assert!(
        db.execute("INSERT INTO canonical_refs VALUES (99, 1, 1, 3)", [])
            .is_err()
    );
    assert!(
        db.execute("INSERT INTO canonical_refs VALUES (1001001, 1, 1, 1)", [])
            .is_err()
    );
    assert!(
        db.execute(
            "INSERT INTO segment_refs VALUES ('seg-1', 999999, 'exact')",
            []
        )
        .is_err()
    );
    assert!(
        db.execute("DELETE FROM canonical_refs WHERE id = 1001001", [])
            .is_err()
    );
    let application_id: u32 = db
        .pragma_query_value(None, "application_id", |row| row.get(0))
        .unwrap();
    assert_eq!(application_id, SQLITE_APPLICATION_ID);
}

#[test]
fn merged_source_verses_preserve_both_reference_links() {
    let db = content();
    db.execute(
        "INSERT INTO segment_refs VALUES ('seg-1', 1001002, 'merged')",
        [],
    )
    .unwrap();
    let count: u32 = db
        .query_row(
            "SELECT count(*) FROM verses_by_reference WHERE id = 'kjv:Gen.1.1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 2);
    let text_count: u32 = db
        .query_row("SELECT count(*) FROM verse_texts", [], |r| r.get(0))
        .unwrap();
    assert_eq!(text_count, 1);
}

#[test]
fn user_notes_survive_a_content_generation_replacement() {
    let old_content = content();
    let user = Connection::open_in_memory().unwrap();
    user.execute_batch(USER_SCHEMA_SQL).unwrap();
    user.execute_batch(
        "INSERT INTO canonical_refs VALUES (1001001, 'Gen', 1, 1);
         INSERT INTO canonical_refs VALUES (1002001, 'Gen', 2, 1);
         INSERT INTO reference_ranges VALUES ('my-range', 1001001, 1002001);
         INSERT INTO annotations VALUES (
             'my-note', 'my-range', 'note', NULL, 'Keep this note',
             '2026-10-16T00:00:00Z', '2026-10-16T00:00:00Z', NULL, NULL, NULL);",
    )
    .unwrap();
    drop(old_content);
    let next_content = content();
    next_content
        .execute("UPDATE verse_texts SET text = 'Corrected fixture text'", [])
        .unwrap();
    let note: String = user
        .query_row(
            "SELECT content FROM annotations WHERE id = 'my-note'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(note, "Keep this note");
    let exists: bool = next_content
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM canonical_refs WHERE id = 1002001)",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(exists);
    assert!(
        user.execute("DELETE FROM canonical_refs WHERE id = 1001001", [])
            .is_err()
    );
}

#[test]
fn fts_index_returns_canonical_evidence_and_resources_require_a_target() {
    let db = content();
    db.execute_batch(
        "INSERT INTO search_documents VALUES (1, 'kjv:Gen.1.1', NULL, 'Fixture verse text');
         INSERT INTO search_fts(search_fts) VALUES ('rebuild');
         INSERT INTO reference_ranges VALUES ('gen-1-1', 1001001, 1001001);
         INSERT INTO resources VALUES ('resource-1', 'kjv-fixture', 'entry-1', 'commentary', 'Title', NULL, 'Text', NULL);",
    ).unwrap();
    let reference: i64 = db
        .query_row(
            "SELECT v.canonical_ref_id FROM search_fts f
         JOIN search_documents d ON d.id = f.rowid
         JOIN verses_by_reference v ON v.id = d.verse_text_id
         WHERE search_fts MATCH 'Fixture'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(reference, 1001001);
    assert!(db.execute("INSERT INTO resource_links VALUES ('link-1', 'resource-1', NULL, NULL, NULL, 'commentary')", []).is_err());
    db.execute("INSERT INTO resource_links VALUES ('link-1', 'resource-1', 'gen-1-1', NULL, NULL, 'commentary')", []).unwrap();
}

#[test]
fn study_profiles_accept_exact_legacy_and_hebrew_sets_but_reject_mixed_claims() {
    let mut artifact = manifest().profiles.remove(0);
    artifact.profile = "study".into();
    artifact.source_ids = fontes_data_contracts::profile_sources("study")
        .unwrap()
        .iter()
        .map(|s| s.to_string())
        .collect();
    artifact.capabilities = fontes_data_contracts::profile_capabilities("study")
        .unwrap()
        .to_vec();
    assert!(fontes_data_contracts::supported_profile(&artifact));
    artifact.source_ids.retain(|s| s != "abbott-smith");
    assert!(
        !fontes_data_contracts::supported_profile(&artifact),
        "partial new dictionary bundle is not a historical profile"
    );
    artifact.source_ids.retain(|s| s != "easton");
    assert!(fontes_data_contracts::supported_profile(&artifact));
    artifact.source_ids.retain(|s| s != "kjv-strongs");
    assert!(fontes_data_contracts::supported_profile(&artifact));
    artifact.source_ids.retain(|s| s != "oshb-hebrew");
    assert!(
        !fontes_data_contracts::supported_profile(&artifact),
        "A Hebrew capability needs actual Hebrew data"
    );
    artifact
        .capabilities
        .retain(|c| *c != fontes_data_contracts::Capability::HebrewMorphology);
    assert!(
        fontes_data_contracts::supported_profile(&artifact),
        "Published legacy Study remains supported"
    );
    artifact.source_ids.push("unreviewed-extra-source".into());
    assert!(!fontes_data_contracts::supported_profile(&artifact));
}

#[test]
fn reading_tsk_capability_requires_tsk_and_preserves_old_signed_reading_profiles() {
    use fontes_data_contracts::{Capability, supported_profile};
    let mut artifact = manifest().profiles.remove(0);
    artifact.profile = "reading".into();
    artifact.source_ids = vec!["kjv-base".into(), "web-classic".into()];
    artifact.capabilities = vec![Capability::Reading];
    assert!(supported_profile(&artifact));
    artifact.capabilities.push(Capability::CrossReferences);
    assert!(!supported_profile(&artifact));
    artifact.source_ids.push("tsk".into());
    assert!(supported_profile(&artifact));
    artifact
        .capabilities
        .retain(|c| *c != Capability::CrossReferences);
    assert!(!supported_profile(&artifact));
}

#[test]
fn default_resource_bundle_is_the_complete_study_source_set() {
    let config: Value = serde_json::from_str(include_str!("../../../data/profiles.json")).unwrap();
    assert_eq!(
        config["default_profile"],
        fontes_data_contracts::DEFAULT_RESOURCE_PROFILE
    );
    let full = config["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == fontes_data_contracts::DEFAULT_RESOURCE_PROFILE)
        .unwrap();
    for source in [
        "kjv-base",
        "web-classic",
        "tsk",
        "matthew-henry",
        "strongs-hebrew",
        "strongs-greek",
        "nave",
        "oshb-hebrew",
        "robinson-tr",
        "kjv-strongs",
    ] {
        assert!(
            full["required_sources"]
                .as_array()
                .unwrap()
                .iter()
                .any(|s| s == source)
        );
    }
}
