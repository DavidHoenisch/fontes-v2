pub mod release;
mod source;
pub mod study;

use anyhow::{Context, Result, bail, ensure};
use fontes_data_contracts::{
    CANONICAL_REGISTRY_JSON, CONTENT_SCHEMA_SQL, Capability, LockedSource, ProfileArtifact,
    SQLITE_APPLICATION_ID, SourceLock, SourceSnapshot, canonical_ref_id,
};
use rusqlite::{Connection, OpenFlags, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{BufWriter, Read, Write},
    path::Path,
};

pub fn profile_sources(profile: &str) -> Result<&'static [&'static str]> {
    fontes_data_contracts::profile_sources(profile).context("unsupported profile")
}
pub fn profile_capabilities(profile: &str) -> Result<&'static [Capability]> {
    fontes_data_contracts::profile_capabilities(profile).context("unsupported profile")
}

pub fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

pub fn file_digest(path: &Path) -> Result<String> {
    let mut reader = File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(hex::encode(hash.finalize()))
}

pub fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    fs::write(path, bytes)?;
    Ok(())
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Book {
    pub id: u16,
    pub osis: String,
    pub usfm: String,
    pub name: String,
    pub testament: String,
    pub verse_counts: Vec<u16>,
}

#[derive(Deserialize)]
pub struct Registry {
    pub registry_version: u32,
    pub books: Vec<Book>,
}

impl Registry {
    pub fn load() -> Result<Self> {
        let registry: Self = serde_json::from_str(CANONICAL_REGISTRY_JSON)?;
        ensure!(
            registry.registry_version == 1 && registry.books.len() == 66,
            "invalid embedded registry"
        );
        Ok(registry)
    }

    pub fn references(&self) -> Result<BTreeSet<i64>> {
        let mut refs = BTreeSet::new();
        for book in &self.books {
            for (chapter, count) in book.verse_counts.iter().enumerate() {
                for verse in 1..=*count {
                    ensure!(
                        refs.insert(
                            canonical_ref_id(book.id, chapter as u16 + 1, verse)
                                .map_err(anyhow::Error::msg)?
                        ),
                        "duplicate registry anchor"
                    );
                }
            }
        }
        ensure!(refs.len() == 31102, "invalid registry coverage");
        Ok(refs)
    }
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mapping {
    pub reference_overrides: BTreeMap<String, String>,
    pub empty_verses: BTreeSet<String>,
    pub ignored_placeholders: BTreeSet<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Mappings {
    mapping_version: u32,
    sources: BTreeMap<String, Mapping>,
    review: String,
}

#[derive(Deserialize)]
struct ProfileConfig {
    profile_config_version: u32,
    profiles: Vec<Profile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    id: String,
    required_sources: Vec<String>,
    capabilities: Vec<Capability>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Verse {
    pub canonical_ref_id: i64,
    pub source_ref: String,
    pub text: String,
    pub source_omission: bool,
    pub mapping_relation: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportReport {
    pub source_id: String,
    pub verse_count: usize,
    pub empty_verses: Vec<String>,
    pub ignored_placeholders: Vec<String>,
    pub reference_overrides: BTreeMap<String, String>,
    pub excluded_books: BTreeMap<String, usize>,
}

pub struct TranslationInfo {
    pub id: &'static str,
    pub witness: &'static str,
    pub abbr: &'static str,
    pub name: &'static str,
    pub edition: &'static str,
    pub xml_member: &'static str,
}

pub fn translation(source: &str) -> Result<TranslationInfo> {
    match source {
        "kjv-base" => Ok(TranslationInfo {
            id: "translation:kjv",
            witness: "witness:kjv-1769",
            abbr: "KJV",
            name: "King James Version",
            edition: "1769 eBible base text",
            xml_member: "eng-kjv_vpl.xml",
        }),
        "web-classic" => Ok(TranslationInfo {
            id: "translation:web",
            witness: "witness:web-classic",
            abbr: "WEB",
            name: "World English Bible Classic",
            edition: "2020 stable text, locked eBible edition",
            xml_member: "eng-web_vpl.xml",
        }),
        _ => bail!("reading adapter does not support source {source}"),
    }
}

pub fn parse_xml(
    xml: &str,
    source_id: &str,
    registry: &Registry,
    mapping: &Mapping,
) -> Result<(Vec<Verse>, ImportReport)> {
    let document = roxmltree::Document::parse_with_options(
        xml.trim_start_matches('\u{feff}'),
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: 1_000_000,
            ..Default::default()
        },
    )?;
    let root = document.root_element();
    ensure!(
        root.tag_name().name() == "verseFile" && root.tag_name().namespace().is_none(),
        "unexpected verse XML root"
    );
    let books: BTreeMap<_, _> = registry
        .books
        .iter()
        .map(|b| (b.usfm.as_str(), b))
        .collect();
    let expected = registry.references()?;
    let mut verses = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut omissions = BTreeSet::new();
    let mut ignored = BTreeSet::new();
    let mut overrides = BTreeSet::new();
    let mut excluded = BTreeMap::new();
    for node in root.children().filter(|n| n.is_element()) {
        ensure!(
            node.tag_name().name() == "v" && node.attributes().len() == 3,
            "unexpected verse element"
        );
        ensure!(
            node.children().all(|n| n.is_text()),
            "verse contains unsupported nested markup"
        );
        let b = node.attribute("b").context("missing book")?;
        let chapter = node.attribute("c").context("missing chapter")?;
        let verse = node.attribute("v").context("missing verse")?;
        let source_ref = format!("{b}.{chapter}.{verse}");
        if !books.contains_key(b) {
            *excluded.entry(b.to_string()).or_insert(0) += 1;
            continue;
        }
        ensure!(
            seen.insert(source_ref.clone()),
            "duplicate source verse {source_ref}"
        );
        let text: String = node.children().filter_map(|n| n.text()).collect();
        let text = text.trim().to_owned();
        if mapping.ignored_placeholders.contains(&source_ref) {
            ensure!(
                text.is_empty(),
                "ignored placeholder {source_ref} unexpectedly contains text"
            );
            ignored.insert(source_ref);
            continue;
        }
        let target = mapping
            .reference_overrides
            .get(&source_ref)
            .unwrap_or(&source_ref);
        if target != &source_ref {
            overrides.insert(source_ref.clone());
        }
        let parts: Vec<_> = target.split('.').collect();
        ensure!(parts.len() == 3, "invalid mapping {target}");
        let book = books
            .get(parts[0])
            .context("mapping targets unknown book")?;
        let id = canonical_ref_id(book.id, parts[1].parse()?, parts[2].parse()?)
            .map_err(anyhow::Error::msg)?;
        ensure!(
            expected.contains(&id),
            "unmapped source verse {source_ref} -> {target}"
        );
        let omission = text.is_empty();
        ensure!(
            omission == mapping.empty_verses.contains(&source_ref),
            "unreviewed or changed omission at {source_ref}"
        );
        if omission {
            omissions.insert(source_ref.clone());
        }
        let row = Verse {
            canonical_ref_id: id,
            source_ref,
            text,
            source_omission: omission,
            mapping_relation: "exact".into(),
        };
        ensure!(
            verses.insert(id, row).is_none(),
            "multiple verses map to canonical anchor {id}"
        );
    }
    ensure!(
        omissions == mapping.empty_verses
            && ignored == mapping.ignored_placeholders
            && overrides == mapping.reference_overrides.keys().cloned().collect(),
        "reviewed mapping rules were not all exercised"
    );
    ensure!(
        verses.keys().copied().collect::<BTreeSet<_>>() == expected,
        "{source_id} does not cover all 31,102 canonical references"
    );
    let report = ImportReport {
        source_id: source_id.into(),
        verse_count: verses.len(),
        empty_verses: omissions.into_iter().collect(),
        ignored_placeholders: ignored.into_iter().collect(),
        reference_overrides: mapping.reference_overrides.clone(),
        excluded_books: excluded,
    };
    Ok((verses.into_values().collect(), report))
}

pub fn parse_archive(
    path: &Path,
    source_id: &str,
    registry: &Registry,
    mapping: &Mapping,
) -> Result<(Vec<Verse>, ImportReport)> {
    let mut zip = zip::ZipArchive::new(File::open(path)?)?;
    ensure!(zip.len() <= 128, "too many archive entries");
    let entry = zip.by_name(translation(source_id)?.xml_member)?;
    const LIMIT: u64 = 64 * 1024 * 1024;
    ensure!(entry.size() <= LIMIT, "XML exceeds decompression limit");
    let mut bytes = Vec::new();
    entry.take(LIMIT + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= LIMIT,
        "XML exceeds decompression limit"
    );
    parse_xml(std::str::from_utf8(&bytes)?, source_id, registry, mapping)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseReport {
    #[serde(default)]
    pub study: study::StudyCounts,
    pub canonical_ref_count: u32,
    pub verse_text_count: u64,
    pub resource_count: u64,
    pub translations: BTreeMap<String, u64>,
    pub source_omission_count: u64,
    pub content_id: String,
    pub source_lock_sha256: String,
    pub builder_commit: String,
    pub profile: String,
}

pub fn validate_database(path: &Path) -> Result<DatabaseReport> {
    let db = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    let app: u32 = db.pragma_query_value(None, "application_id", |r| r.get(0))?;
    let version: u32 = db.pragma_query_value(None, "user_version", |r| r.get(0))?;
    ensure!(
        app == SQLITE_APPLICATION_ID && version == 1,
        "not a compatible Fontes content database"
    );
    let integrity: String = db.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    ensure!(integrity == "ok", "SQLite integrity failed: {integrity}");
    ensure!(
        db.prepare("PRAGMA foreign_key_check")?
            .query([])?
            .next()?
            .is_none(),
        "foreign key validation failed"
    );
    let registry = Registry::load()?;
    let actual: BTreeSet<i64> = db
        .prepare("SELECT id FROM canonical_refs")?
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    ensure!(
        actual == registry.references()?,
        "canonical registry mismatch"
    );
    for book in &registry.books {
        let row: (String, String, u16) = db.query_row(
            "SELECT osis, testament, sort_order FROM books WHERE id = ?1",
            [book.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        ensure!(
            row == (book.osis.clone(), book.testament.clone(), book.id),
            "book identity mismatch"
        );
    }
    let book_count: u32 = db.query_row("SELECT count(*) FROM books", [], |r| r.get(0))?;
    ensure!(book_count == 66, "unexpected books");
    let translations: BTreeMap<String, u64> = db
        .prepare("SELECT translation_id, count(*) FROM verse_texts GROUP BY translation_id")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    ensure!(
        translations.len() == 2
            && translations.get("translation:kjv") == Some(&31102)
            && translations.get("translation:web") == Some(&31102),
        "reading translation coverage mismatch"
    );
    let mapping_count: u64 =
        db.query_row("SELECT count(*) FROM verses_by_reference", [], |r| r.get(0))?;
    let distinct_mappings: u64 = db.query_row("SELECT count(*) FROM (SELECT DISTINCT translation_id, canonical_ref_id FROM verses_by_reference)", [], |r| r.get(0))?;
    ensure!(
        mapping_count == 62204 && distinct_mappings == 62204,
        "incomplete or duplicated canonical verse mapping"
    );
    let invalid_witnesses: u64 = db.query_row("SELECT count(*) FROM verse_texts v JOIN source_segments s ON s.id=v.segment_id JOIN translations t ON t.id=v.translation_id WHERE s.witness_id IS NOT t.witness_id", [], |r| r.get(0))?;
    ensure!(invalid_witnesses == 0, "cross-witness verse mapping");
    let empty: u64 = db.query_row("SELECT count(*) FROM verse_texts WHERE text = '' AND display_json = '{\"type\":\"source_omission\"}' AND translation_id = 'translation:web'", [], |r| r.get(0))?;
    let all_empty: u64 = db.query_row(
        "SELECT count(*) FROM verse_texts WHERE text = ''",
        [],
        |r| r.get(0),
    )?;
    ensure!(
        empty == 4 && all_empty == 4,
        "incorrect source omission handling"
    );
    let documents: u64 = db.query_row("SELECT count(*) FROM search_documents", [], |r| r.get(0))?;
    let resource_documents: u64 =
        db.query_row("SELECT count(*) FROM resources", [], |r| r.get(0))?;
    ensure!(
        documents == 62204 + resource_documents,
        "search document coverage mismatch"
    );
    let bad_documents: u64 = db.query_row("SELECT count(*) FROM resources r WHERE (SELECT count(*) FROM search_documents d WHERE d.resource_id=r.id)!=1",[],|r|r.get(0))?;
    ensure!(bad_documents == 0, "resource search coverage mismatch");
    let hits: u64 = db.query_row("SELECT count(*) FROM search_fts f JOIN search_documents d ON d.id = f.rowid JOIN verses_by_reference v ON v.id = d.verse_text_id WHERE search_fts MATCH 'God AND world' AND v.canonical_ref_id = 43003016", [], |r| r.get(0))?;
    ensure!(hits == 2, "John 3:16 search smoke query failed");
    let doxology: String = db.query_row("SELECT text FROM verses_by_reference WHERE translation_id = 'translation:web' AND canonical_ref_id = 45016025", [], |r| r.get(0))?;
    ensure!(
        doxology.starts_with("Now to him"),
        "WEB Romans doxology not mapped"
    );
    let resources: u64 = db.query_row("SELECT count(*) FROM resources", [], |r| r.get(0))?;
    let (content_id, lock, commit, profile): (String,String,String,String) = db.query_row("SELECT content_id, source_lock_sha256, builder_commit, profile FROM build_info WHERE singleton = 1", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?;
    let source_ids: BTreeSet<String> = db
        .prepare("SELECT source_id FROM source_snapshots")?
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    ensure!(
        source_ids
            == profile_sources(&profile)?
                .iter()
                .map(|s| s.to_string())
                .collect(),
        "database source membership mismatch"
    );
    let study = study::StudyCounts::read(&db)?;
    if profile == "study" {
        study::validate_study(&db, &study)?;
    } else {
        ensure!(
            profile == "reading" && study == study::StudyCounts::default(),
            "unsupported profile or unexpected study rows"
        );
    }
    Ok(DatabaseReport {
        study,
        canonical_ref_count: actual.len() as u32,
        verse_text_count: 62204,
        resource_count: resources,
        translations,
        source_omission_count: empty,
        content_id,
        source_lock_sha256: lock,
        builder_commit: commit,
        profile,
    })
}

#[derive(Serialize, Deserialize)]
pub struct BuildReport {
    #[serde(default)]
    pub study_imports: Vec<study::StudyImport>,
    pub compiler_version: String,
    pub sqlite_version: String,
    pub builder_commit: String,
    pub source_lock_sha256: String,
    pub sources: Vec<SourceSnapshot>,
    pub imports: Vec<ImportReport>,
    pub database: DatabaseReport,
    pub profile_artifact: ProfileArtifact,
}

pub fn source_asset(source: &LockedSource) -> String {
    format!(
        "source-{}-{}.{}",
        source.source_id,
        source.sha256,
        if source.adapter.starts_with("strongs-") {
            "xml"
        } else {
            "zip"
        }
    )
}

pub fn build(
    root: &Path,
    profile: &str,
    cache: &Path,
    output: &Path,
    commit: &str,
    offline: bool,
    mirror_repository: Option<&str>,
) -> Result<BuildReport> {
    ensure!(
        matches!(profile, "reading" | "study"),
        "unsupported profile"
    );
    ensure!(
        commit.len() == 40 && commit.bytes().all(|c| c.is_ascii_hexdigit()),
        "builder commit must be a full Git SHA"
    );
    ensure!(
        !output.exists() || fs::read_dir(output)?.next().is_none(),
        "output directory must be new or empty"
    );
    let lock_bytes = fs::read(root.join("data/sources.lock.json"))?;
    let lock: SourceLock = serde_json::from_slice(&lock_bytes)?;
    lock.validate().map_err(anyhow::Error::msg)?;
    let profile_bytes = fs::read(root.join("data/profiles.json"))?;
    let config: ProfileConfig = serde_json::from_slice(&profile_bytes)?;
    ensure!(
        config.profile_config_version == 1,
        "unsupported profiles file"
    );
    let config = config
        .profiles
        .into_iter()
        .find(|p| p.id == profile)
        .context("profile not defined")?;
    ensure!(
        config.required_sources == profile_sources(profile)?
            && config.capabilities == profile_capabilities(profile)?,
        "profile membership changed without adapter support"
    );
    let mappings_bytes = fs::read(root.join("data/mappings/reading-v1.json"))?;
    let mappings: Mappings = serde_json::from_slice(&mappings_bytes)?;
    ensure!(
        mappings.mapping_version == 1 && !mappings.review.is_empty(),
        "invalid mapping review"
    );
    ensure!(
        fs::read(root.join("data/canonical/registry-v1.json"))?
            == CANONICAL_REGISTRY_JSON.as_bytes(),
        "on-disk registry differs from compiled registry"
    );
    let registry = Registry::load()?;
    let mut acquired = Vec::new();
    let mut study_acquired = Vec::new();
    if profile == "study" {
        study::check_exporter()?;
    }
    for id in &config.required_sources {
        let source = lock
            .sources
            .iter()
            .find(|s| &s.source_id == id)
            .context("required source not locked")?;
        let path = source::acquire(source, cache, offline, mirror_repository)?;
        if !matches!(id.as_str(), "kjv-base" | "web-classic") {
            let expected = match id.as_str() {
                "strongs-greek" => "strongs-greek-xml-v1",
                "strongs-hebrew" => "strongs-hebrew-osis-v1",
                "robinson-tr" => "robinson-utr-v1",
                "nave" => "sword-dictionary-v1",
                "matthew-henry" | "tsk" => "sword-commentary-v1",
                _ => bail!("unknown study source"),
            };
            ensure!(source.adapter == expected, "unsupported study adapter");
            study_acquired.push((source, path));
            continue;
        }
        ensure!(
            source.adapter == "ebible-vpl-v1",
            "unsupported reading adapter"
        );
        let (verses, report) = parse_archive(
            &path,
            id,
            &registry,
            mappings.sources.get(id).context("missing source mapping")?,
        )
        .with_context(|| format!("importing {id}"))?;
        acquired.push((source, path, verses, report));
    }
    let lock_sha = digest(&lock_bytes);
    let content_id = digest(&serde_json::to_vec(&serde_json::json!({
        "study_mapping": if profile=="study" {Some(digest(&fs::read(root.join("data/mappings/study-v1.json"))?))} else {None}, "profile": profile, "sword_exporter": (profile=="study").then_some(study::SWORD_VERSION), "lock": lock_sha, "mappings": digest(&mappings_bytes),
        "profile_config": digest(&profile_bytes), "registry": digest(CANONICAL_REGISTRY_JSON.as_bytes()),
        "schema": digest(CONTENT_SCHEMA_SQL.as_bytes()), "cargo_lock": digest(&fs::read(root.join("Cargo.lock"))?),
        "toolchain": digest(&fs::read(root.join("rust-toolchain.toml"))?),
        "builder_commit": commit, "compiler": env!("CARGO_PKG_VERSION"), "sqlite": rusqlite::version()
    }))?);
    fs::create_dir_all(output.join("normalized"))?;
    let database = output.join("library.sqlite");
    let mut db = Connection::open(&database)?;
    db.execute_batch(
        "PRAGMA page_size=4096; PRAGMA journal_mode=DELETE; PRAGMA temp_store=MEMORY;",
    )?;
    db.execute_batch(CONTENT_SCHEMA_SQL)?;
    let tx = db.transaction()?;
    tx.execute(
        "INSERT INTO build_info VALUES (1, ?1, ?2, 1, 1, ?3, ?4)",
        params![content_id, profile, lock_sha, commit],
    )?;
    for book in &registry.books {
        tx.execute(
            "INSERT INTO books VALUES (?1, ?2, ?3, ?4, ?1)",
            params![book.id, book.osis, book.name, book.testament],
        )?;
        for (chapter, count) in book.verse_counts.iter().enumerate() {
            for verse in 1..=*count {
                tx.prepare_cached("INSERT INTO canonical_refs VALUES (?1, ?2, ?3, ?4)")?
                    .execute(params![
                        canonical_ref_id(book.id, chapter as u16 + 1, verse)
                            .map_err(anyhow::Error::msg)?,
                        book.id,
                        chapter + 1,
                        verse
                    ])?;
            }
        }
    }
    let mut documents = 0_i64;
    let mut snapshots = Vec::new();
    let mut reports = Vec::new();
    let mut notice = String::from(
        "Fontes content data\n\nEnglish base verse text only; no modern KJV word annotations.\nCanonical registry: fontes-protestant-66-v1.\n\n",
    );
    for (source, path, verses, report) in acquired {
        let info = translation(&source.source_id)?;
        let snapshot = format!("source:{}@{}", source.source_id, source.sha256);
        tx.execute(
            "INSERT INTO source_snapshots VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                snapshot,
                source.source_id,
                source.sha256,
                source.url,
                source.upstream_revision,
                source.rights_review_id,
                source.adapter
            ],
        )?;
        tx.execute(
            "INSERT INTO text_witnesses VALUES (?1, ?2, 'en', ?3, ?4, ?5)",
            params![
                info.witness,
                info.name,
                info.edition,
                if source.source_id == "web-classic" {
                    "WEB Classic (Romans doxology mapped to KJV)"
                } else {
                    "KJV"
                },
                snapshot
            ],
        )?;
        tx.execute(
            "INSERT INTO translations VALUES (?1, ?2, ?3, 'en', ?4)",
            params![info.id, info.abbr, info.name, info.witness],
        )?;
        let mut normalized = BufWriter::new(File::create(
            output
                .join("normalized")
                .join(format!("{}.jsonl", source.source_id)),
        )?);
        for verse in verses {
            serde_json::to_writer(&mut normalized, &verse)?;
            normalized.write_all(b"\n")?;
            let segment = format!("segment:{}:{}", source.source_id, verse.source_ref);
            let verse_id = format!("{}:{}", info.id, verse.source_ref);
            tx.prepare_cached("INSERT INTO source_segments VALUES (?1, ?2, ?3, ?4, ?4)")?
                .execute(params![segment, snapshot, info.witness, verse.source_ref])?;
            tx.prepare_cached("INSERT INTO segment_refs VALUES (?1, ?2, ?3)")?
                .execute(params![
                    segment,
                    verse.canonical_ref_id,
                    verse.mapping_relation
                ])?;
            let display = verse
                .source_omission
                .then_some("{\"type\":\"source_omission\"}");
            tx.prepare_cached("INSERT INTO verse_texts VALUES (?1, ?2, ?3, ?4, ?5)")?
                .execute(params![verse_id, info.id, segment, verse.text, display])?;
            documents += 1;
            tx.prepare_cached("INSERT INTO search_documents VALUES (?1, ?2, NULL, ?3)")?
                .execute(params![documents, verse_id, verse.text])?;
        }
        normalized.flush()?;
        fs::copy(path, output.join(source_asset(source)))?;
        notice.push_str(&format!(
            "{}\nSource: {}\nRights evidence: {}\nReview: {}\nSHA-256: {}\n\n",
            info.name,
            source.url,
            source.rights_evidence_url,
            source.rights_review_id,
            source.sha256
        ));
        snapshots.push(SourceSnapshot {
            source_id: source.source_id.clone(),
            sha256: source.sha256.clone(),
            adapter_version: source.adapter.clone(),
            rights_review_id: source.rights_review_id.clone(),
        });
        reports.push(report);
    }
    let mut study_imports = Vec::new();
    for (source, path) in study_acquired {
        let snapshot = format!("source:{}@{}", source.source_id, source.sha256);
        tx.execute(
            "INSERT INTO source_snapshots VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                snapshot,
                source.source_id,
                source.sha256,
                source.url,
                source.upstream_revision,
                source.rights_review_id,
                source.adapter
            ],
        )?;
        let normalized = output
            .join("normalized")
            .join(format!("{}.jsonl", source.source_id));
        let report = match source.source_id.as_str() {
            "strongs-greek" | "strongs-hebrew" => {
                study::import_strongs(&tx, &source.source_id, &snapshot, &path)?
            }
            "robinson-tr" => study::import_greek(&tx, &snapshot, &path, &registry, &normalized)?,
            _ => study::import_sword(
                &tx,
                &source.source_id,
                &snapshot,
                &path,
                &registry,
                &normalized,
            )?,
        };
        println!(
            "Imported {}: {} source records, {} resources, {} linked aliases, {} unresolved citations",
            source.source_id,
            report.source_records,
            report.resources,
            report.linked_aliases,
            report.unmapped_citations.len()
        );
        notice.push_str(&format!("{}\nSource: {}\nRights evidence: {}\nReview: {}\nSHA-256: {}\nUnmapped citations retained for review: {}\n\n",source.source_id,source.url,source.rights_evidence_url,source.rights_review_id,source.sha256,report.unmapped_citations.len()));
        fs::copy(path, output.join(source_asset(source)))?;
        snapshots.push(SourceSnapshot {
            source_id: source.source_id.clone(),
            sha256: source.sha256.clone(),
            adapter_version: source.adapter.clone(),
            rights_review_id: source.rights_review_id.clone(),
        });
        study_imports.push(report);
    }
    if profile == "study" {
        study::check_citation_policy(
            &root.join("data/mappings/study-v1.json"),
            &lock,
            &study_imports,
        )?;
        study::write_audit(&output.join("study-imports.json"), &study_imports)?;
        notice.push_str("Greek: Robinson's public-domain UTR, both source reading streams; unaccented Unicode surfaces, alternative analyses and numeric parsing IDs retained. Source subscriptions and alternate numbering remain in structured apparatus. No English word alignment or Hebrew morphology is inferred. Strong's Hebrew headwords retain supplied Unicode and transliteration; TWOT article text is not imported.\nSWORD 1.9.0 is a build-time extraction dependency only. Scripture links are registry checked; see study-imports.json for source citation anomalies.\n");
    }
    tx.execute("INSERT INTO search_fts(search_fts) VALUES ('rebuild')", [])?;
    tx.execute(
        "INSERT INTO search_fts(search_fts, rank) VALUES ('integrity-check', 1)",
        [],
    )?;
    tx.commit()?;
    db.execute_batch("VACUUM;")?;
    db.close().map_err(|(_, e)| e)?;
    let validation = validate_database(&database)?;
    let asset = format!("fontes-{profile}.sqlite.zst");
    let compressed = output.join(&asset);
    let mut encoder = zstd::stream::Encoder::new(File::create(&compressed)?, 9)?;
    std::io::copy(&mut File::open(&database)?, &mut encoder)?;
    encoder.finish()?.sync_all()?;
    let artifact = ProfileArtifact {
        profile: profile.into(),
        capabilities: config.capabilities,
        asset,
        compression: "zstd".into(),
        compressed_bytes: compressed.metadata()?.len(),
        compressed_sha256: file_digest(&compressed)?,
        sqlite_bytes: database.metadata()?.len(),
        sqlite_sha256: file_digest(&database)?,
        content_id,
        source_ids: snapshots.iter().map(|s| s.source_id.clone()).collect(),
        canonical_ref_count: validation.canonical_ref_count,
        verse_text_count: validation.verse_text_count,
        resource_count: validation.resource_count,
    };
    notice.push_str("KJV: public domain outside the United Kingdom; Crown rights apply in the UK as described by the provider.\nWEB: dedicated to the public domain. World English Bible is a trademark of eBible.org; the underlying verse text is preserved apart from surrounding whitespace.\nFour WEB omissions are represented explicitly; the Romans doxology has reviewed canonical mappings.\nThese verse-per-line sources do not preserve the full paragraph, poetry, and footnote structure of USFM.\n");
    fs::write(output.join("NOTICE.txt"), notice)?;
    fs::write(output.join("sources.lock.json"), lock_bytes)?;
    let report = BuildReport {
        study_imports,
        compiler_version: env!("CARGO_PKG_VERSION").into(),
        sqlite_version: rusqlite::version().into(),
        builder_commit: commit.into(),
        source_lock_sha256: lock_sha,
        sources: snapshots,
        imports: reports,
        database: validation,
        profile_artifact: artifact,
    };
    write_json(&output.join("validation-report.json"), &report)?;
    println!(
        "Built {} profile: {} verse rows, {} resources, {} bytes SQLite, {} bytes zstd",
        profile,
        report.database.verse_text_count,
        report.database.resource_count,
        report.profile_artifact.sqlite_bytes,
        report.profile_artifact.compressed_bytes
    );
    Ok(report)
}
