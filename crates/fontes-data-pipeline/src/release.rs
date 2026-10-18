use crate::{
    BuildReport, DatabaseReport, digest, file_digest, source::valid_repository, source_asset,
    validate_database, write_json,
};
use anyhow::{Context, Result, bail, ensure};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use fontes_data_contracts::{Channel, ProfileArtifact, ReleaseManifest, SourceLock};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
    process::Command,
};

const DOMAIN: &[u8] = b"FONTES-DATA-MANIFEST-V1\n";
const MAX_SQLITE: u64 = 1024 * 1024 * 1024;
const MAX_COMPRESSED: u64 = 256 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustKey {
    pub key_id: String,
    pub public_key_hex: String,
}

fn read_limited(path: &Path, limit: u64) -> Result<Vec<u8>> {
    ensure!(
        path.metadata()?.len() <= limit,
        "{} exceeds size limit",
        path.display()
    );
    let mut bytes = Vec::new();
    File::open(path)?.take(limit + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= limit, "file grew beyond limit");
    Ok(bytes)
}

fn signing_message(bytes: &[u8]) -> Vec<u8> {
    [DOMAIN, bytes].concat()
}

pub fn keygen(secret: &Path, public: &Path, key_id: &str) -> Result<()> {
    ensure!(
        !key_id.is_empty()
            && key_id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
        "invalid key ID"
    );
    ensure!(
        !public.exists() && !secret.exists(),
        "key file already exists"
    );
    if let Some(p) = secret.parent() {
        fs::create_dir_all(p)?;
    }
    if let Some(p) = public.parent() {
        fs::create_dir_all(p)?;
    }
    let key = SigningKey::generate(&mut OsRng);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(secret)?;
    writeln!(file, "{}", hex::encode(key.to_bytes()))?;
    file.sync_all()?;
    write_json(
        public,
        &TrustKey {
            key_id: key_id.into(),
            public_key_hex: hex::encode(key.verifying_key().to_bytes()),
        },
    )?;
    println!(
        "Created {key_id}; private seed written only to {}",
        secret.display()
    );
    Ok(())
}

pub fn verify_manifest(
    bytes: &[u8],
    signature: &[u8],
    trust: &TrustKey,
    repository: &str,
) -> Result<ReleaseManifest> {
    ensure!(
        bytes.len() <= 65536 && valid_repository(repository),
        "invalid manifest size or repository"
    );
    let public: [u8; 32] = hex::decode(&trust.public_key_hex)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("public key must be 32 bytes"))?;
    let key = VerifyingKey::from_bytes(&public)?;
    key.verify_strict(&signing_message(bytes), &Signature::from_slice(signature)?)
        .context("manifest signature verification failed")?;
    let manifest: ReleaseManifest = serde_json::from_slice(bytes)?;
    manifest.validate().map_err(anyhow::Error::msg)?;
    ensure!(
        manifest.repository == repository
            && manifest.channel == Channel::Stable
            && manifest.signing_key_id == trust.key_id,
        "manifest trust identity mismatch"
    );
    ensure!(
        manifest
            .profiles
            .iter()
            .all(|p| matches!(p.profile.as_str(), "reading" | "study")),
        "unsupported published profile"
    );
    for artifact in &manifest.profiles {
        ensure!(
            fontes_data_contracts::supported_profile(artifact),
            "profile source/capability mismatch"
        );
    }
    Ok(manifest)
}

fn decompress_checked(
    dir: &Path,
    manifest: &ReleaseManifest,
    artifact: &ProfileArtifact,
    dest: &Path,
) -> Result<DatabaseReport> {
    ensure!(
        artifact.sqlite_bytes <= MAX_SQLITE && artifact.compressed_bytes <= MAX_COMPRESSED,
        "artifact exceeds client size limit"
    );
    let compressed = dir.join(&artifact.asset);
    ensure!(
        compressed.metadata()?.len() == artifact.compressed_bytes
            && file_digest(&compressed)? == artifact.compressed_sha256,
        "compressed artifact digest/length mismatch"
    );
    let decoder = zstd::stream::read::Decoder::new(File::open(compressed)?)?;
    let mut output = File::create(dest)?;
    let bytes = std::io::copy(&mut decoder.take(artifact.sqlite_bytes + 1), &mut output)?;
    output.sync_all()?;
    ensure!(
        bytes == artifact.sqlite_bytes && file_digest(dest)? == artifact.sqlite_sha256,
        "SQLite artifact digest/length mismatch"
    );
    let db = validate_database(dest)?;
    ensure!(
        db.content_id == artifact.content_id
            && db.profile == artifact.profile
            && db.source_lock_sha256 == manifest.source_lock_sha256
            && db.builder_commit == manifest.builder_commit,
        "database build identity mismatch"
    );
    ensure!(
        db.canonical_ref_count == artifact.canonical_ref_count
            && db.verse_text_count == artifact.verse_text_count
            && db.resource_count == artifact.resource_count,
        "manifest counts do not match database"
    );
    Ok(db)
}

pub fn verify_package(dir: &Path, trust_path: &Path, repository: &str) -> Result<ReleaseManifest> {
    let trust: TrustKey = serde_json::from_slice(&read_limited(trust_path, 4096)?)?;
    let bytes = read_limited(&dir.join("release-manifest.json"), 65536)?;
    let signature = read_limited(&dir.join("release-manifest.json.sig"), 64)?;
    let manifest = verify_manifest(&bytes, &signature, &trust, repository)?;
    let lock_bytes = read_limited(&dir.join("sources.lock.json"), 1024 * 1024)?;
    ensure!(
        digest(&lock_bytes) == manifest.source_lock_sha256,
        "source lock digest mismatch"
    );
    let lock: SourceLock = serde_json::from_slice(&lock_bytes)?;
    lock.validate().map_err(anyhow::Error::msg)?;
    for snapshot in &manifest.sources {
        let source = lock
            .sources
            .iter()
            .find(|s| s.source_id == snapshot.source_id)
            .context("source missing from lock")?;
        ensure!(
            snapshot.sha256 == source.sha256
                && snapshot.adapter_version == source.adapter
                && snapshot.rights_review_id == source.rights_review_id,
            "source snapshot provenance mismatch"
        );
        let file = dir.join(source_asset(source));
        ensure!(
            file.metadata()?.len() == source.bytes && file_digest(&file)? == source.sha256,
            "retained source digest mismatch"
        );
    }
    if manifest
        .sources
        .iter()
        .any(|s| s.source_id == "kjv-strongs")
    {
        ensure!(
            fs::read(dir.join("GPL-3.0.txt"))?
                == include_bytes!("../../../data/rights/GPL-3.0.txt"),
            "KJV annotation GPL text missing or changed"
        );
        ensure!(
            fs::read(dir.join("KJV-ANNOTATIONS-NOTICE.md"))?
                == include_bytes!("../../../data/rights/kjv-strongs-2026-10-16.md"),
            "KJV annotation provenance notice missing or changed"
        );
    }
    let tmp = tempfile::tempdir()?;
    for artifact in &manifest.profiles {
        decompress_checked(
            dir,
            &manifest,
            artifact,
            &tmp.path().join(format!("{}.sqlite", artifact.profile)),
        )?;
    }
    Ok(manifest)
}

fn gh(args: &[&str]) -> Result<String> {
    let output = Command::new("gh")
        .args(args)
        .output()
        .context("GitHub CLI is required for publication")?;
    if !output.status.success() {
        bail!(
            "gh {} failed: {}",
            args.first().unwrap_or(&""),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn download_assets(repository: &str, tag: &str, dir: &Path, patterns: &[&str]) -> Result<()> {
    let mut args = vec![
        "release",
        "download",
        tag,
        "--repo",
        repository,
        "--dir",
        dir.to_str().context("non-UTF8 path")?,
    ];
    for pattern in patterns {
        args.extend(["--pattern", pattern]);
    }
    gh(&args)?;
    Ok(())
}

fn texts(path: &Path) -> Result<BTreeMap<(String, i64), String>> {
    let db =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut stmt =
        db.prepare("SELECT translation_id, canonical_ref_id, text FROM verses_by_reference")?;
    Ok(stmt
        .query_map([], |r| Ok(((r.get(0)?, r.get(1)?), r.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?)
}

fn resource_texts(path: &Path) -> Result<BTreeMap<String, String>> {
    let db =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    Ok(db.prepare("SELECT id,title || char(10) || content || char(10) || coalesce(display_json,'') FROM resources")?.query_map([],|r|Ok((r.get(0)?,r.get(1)?)))?.collect::<rusqlite::Result<_>>()?)
}

/// Only the deliberate Reading -> complete-default consolidation may retire a profile.
fn check_profile_transition(
    previous: &[ProfileArtifact],
    next: &[ProfileArtifact],
) -> Result<Vec<String>> {
    let complete_default = next.len() == 1
        && next[0].profile == fontes_data_contracts::DEFAULT_RESOURCE_PROFILE
        && next[0]
            .source_ids
            .iter()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>()
            == fontes_data_contracts::profile_sources("study")
                .unwrap()
                .iter()
                .copied()
                .collect()
        && fontes_data_contracts::supported_profile(&next[0]);
    let mut retired = Vec::new();
    for old in previous {
        if next.iter().any(|p| p.profile == old.profile) {
            continue;
        }
        ensure!(
            complete_default && old.profile == "reading",
            "published profile would disappear without a complete default replacement"
        );
        retired.push(old.profile.clone());
    }
    Ok(retired)
}

pub fn publish(
    dir: &Path,
    trust_path: &Path,
    repository: &str,
    additional: Option<&Path>,
) -> Result<()> {
    ensure!(valid_repository(repository), "invalid release repository");
    let trust: TrustKey = serde_json::from_slice(&read_limited(trust_path, 4096)?)?;
    let report: BuildReport = serde_json::from_slice(&read_limited(
        &dir.join("validation-report.json"),
        1024 * 1024,
    )?)?;
    let actual = validate_database(&dir.join("library.sqlite"))?;
    ensure!(
        serde_json::to_value(&actual)? == serde_json::to_value(&report.database)?,
        "build report does not match SQLite"
    );
    ensure!(
        actual.builder_commit == report.builder_commit
            && actual.source_lock_sha256 == report.source_lock_sha256,
        "report provenance mismatch"
    );
    let mut profiles = vec![report.profile_artifact.clone()];
    let mut snapshots: BTreeMap<_, _> = report
        .sources
        .iter()
        .cloned()
        .map(|s| (s.source_id.clone(), s))
        .collect();
    if let Some(other) = additional {
        let study: BuildReport = serde_json::from_slice(&read_limited(
            &other.join("validation-report.json"),
            1024 * 1024,
        )?)?;
        let checked = validate_database(&other.join("library.sqlite"))?;
        ensure!(
            serde_json::to_value(&checked)? == serde_json::to_value(&study.database)?,
            "additional report does not match database"
        );
        ensure!(
            report.profile_artifact.profile == "reading"
                && study.profile_artifact.profile == "study"
                && study.builder_commit == report.builder_commit
                && study.source_lock_sha256 == report.source_lock_sha256,
            "incompatible profile bundle"
        );
        ensure!(
            fs::read(dir.join("sources.lock.json"))? == fs::read(other.join("sources.lock.json"))?,
            "different source locks in profile bundle"
        );
        profiles.push(study.profile_artifact.clone());
        let lock: SourceLock = serde_json::from_slice(&fs::read(dir.join("sources.lock.json"))?)?;
        for snapshot in &study.sources {
            if let Some(existing) = snapshots.get(&snapshot.source_id) {
                ensure!(
                    serde_json::to_value(existing)? == serde_json::to_value(snapshot)?,
                    "conflicting profile source"
                );
            }
            snapshots.insert(snapshot.source_id.clone(), snapshot.clone());
            let source = lock
                .sources
                .iter()
                .find(|s| s.source_id == snapshot.source_id)
                .context("bundle source not locked")?;
            let asset = source_asset(source);
            fs::copy(other.join(&asset), dir.join(&asset))?;
        }
        fs::copy(
            other.join(&study.profile_artifact.asset),
            dir.join(&study.profile_artifact.asset),
        )?;
        fs::copy(
            other.join("validation-report.json"),
            dir.join("validation-report-study.json"),
        )?;
        fs::copy(
            other.join("study-imports.json"),
            dir.join("study-imports.json"),
        )?;
        if study.sources.iter().any(|s| s.source_id == "kjv-strongs") {
            for name in ["GPL-3.0.txt", "KJV-ANNOTATIONS-NOTICE.md"] {
                fs::copy(other.join(name), dir.join(name))?;
            }
        }
        let mut notices = fs::read_to_string(dir.join("NOTICE.txt"))?;
        notices.push_str("\n=== STUDY PROFILE ===\n");
        notices.push_str(&fs::read_to_string(other.join("NOTICE.txt"))?);
        fs::write(dir.join("NOTICE.txt"), notices)?;
    }
    let snapshots: Vec<_> = snapshots.into_values().collect();
    let releases: Vec<Value> = serde_json::from_str(&gh(&[
        "release",
        "list",
        "--repo",
        repository,
        "--limit",
        "10000",
        "--json",
        "tagName,isDraft",
    ])?)?;
    let sequence_for = |value: &Value| -> Option<u64> {
        value["tagName"]
            .as_str()?
            .strip_prefix("data-v1-stable-")?
            .parse()
            .ok()
    };
    let sequence = releases
        .iter()
        .filter_map(sequence_for)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .context("release sequence overflow")?;
    let previous = releases
        .iter()
        .filter(|r| r["isDraft"] == false)
        .filter_map(|r| sequence_for(r).map(|seq| (seq, r)))
        .max_by_key(|(seq, _)| *seq)
        .map(|(_, r)| r["tagName"].as_str().unwrap().to_owned());
    let old = tempfile::tempdir()?;
    let mut changes = serde_json::json!({"previous_release":previous,"profiles":{},"source_changes":[],"retired_profiles":[],"default_profile":fontes_data_contracts::DEFAULT_RESOURCE_PROFILE});
    if let Some(tag) = &previous {
        download_assets(
            repository,
            tag,
            old.path(),
            &["release-manifest.json", "release-manifest.json.sig"],
        )?;
        let prior = verify_manifest(
            &read_limited(&old.path().join("release-manifest.json"), 65536)?,
            &read_limited(&old.path().join("release-manifest.json.sig"), 64)?,
            &trust,
            repository,
        )?;
        ensure!(
            prior.release_tag == *tag,
            "previous release identity mismatch"
        );
        changes["retired_profiles"] =
            serde_json::to_value(check_profile_transition(&prior.profiles, &profiles)?)?;
        changes["default_profile"] = fontes_data_contracts::DEFAULT_RESOURCE_PROFILE.into();
        if profiles.len() == prior.profiles.len()
            && profiles.iter().all(|p| {
                prior
                    .profiles
                    .iter()
                    .any(|old| old.profile == p.profile && old.content_id == p.content_id)
            })
        {
            println!(
                "No content changes; existing release https://github.com/{repository}/releases/tag/{tag}"
            );
            return Ok(());
        }
        for profile in &profiles {
            let current_dir = if profile.profile == report.profile_artifact.profile {
                dir
            } else {
                additional.context("profile directory missing")?
            };
            if let Some(prior_profile) =
                prior.profiles.iter().find(|p| p.profile == profile.profile)
            {
                download_assets(repository, tag, old.path(), &[&prior_profile.asset])?;
                let old_database = old.path().join(format!("{}.sqlite", profile.profile));
                decompress_checked(old.path(), &prior, prior_profile, &old_database)?;
                let before = texts(&old_database)?;
                let after = texts(&current_dir.join("library.sqlite"))?;
                let added = after
                    .keys()
                    .filter(|key| !before.contains_key(*key))
                    .count();
                let removed = before
                    .keys()
                    .filter(|key| !after.contains_key(*key))
                    .count();
                let changed = after
                    .iter()
                    .filter(|(key, text)| before.get(*key).is_some_and(|old| old != *text))
                    .count();
                ensure!(
                    added == 0 && removed == 0,
                    "unexpected reading coverage regression"
                );
                let before = resource_texts(&old_database)?;
                let after = resource_texts(&current_dir.join("library.sqlite"))?;
                changes["profiles"][&profile.profile] = serde_json::json!({"added_verse_rows":added,"changed_verse_rows":changed,"removed_verse_rows":removed,"added_resources":after.keys().filter(|key|!before.contains_key(*key)).count(),"removed_resources":before.keys().filter(|key|!after.contains_key(*key)).count(),"changed_resources":after.iter().filter(|(key,text)|before.get(*key).is_some_and(|old|old!=*text)).count()});
            } else {
                changes["profiles"][&profile.profile] = serde_json::json!({"new_profile":true,"added_verse_rows":profile.verse_text_count,"added_resources":profile.resource_count});
            }
        }
        changes["source_changes"]=serde_json::to_value(snapshots.iter().filter_map(|new| {
            let old=prior.sources.iter().find(|s|s.source_id==new.source_id);
            (old.map(|s|&s.sha256)!=Some(&new.sha256)).then(||serde_json::json!({"source_id":new.source_id,"before":old.map(|s|&s.sha256),"after":new.sha256}))
        }).collect::<Vec<_>>())?;
    }
    let manifest = ReleaseManifest {
        manifest_version: 1,
        update_protocol_version: 1,
        content_schema_version: 1,
        canonical_registry_version: 1,
        repository: repository.into(),
        release_tag: format!("data-v1-stable-{sequence}"),
        channel: Channel::Stable,
        sequence,
        published_at: time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)?,
        signing_key_id: trust.key_id.clone(),
        builder_commit: report.builder_commit.clone(),
        source_lock_sha256: report.source_lock_sha256.clone(),
        profiles,
        sources: snapshots,
    };
    manifest.validate().map_err(anyhow::Error::msg)?;
    write_json(&dir.join("changes.json"), &changes)?;
    write_json(&dir.join("release-manifest.json"), &manifest)?;
    let seed = std::env::var("FONTES_DATA_SIGNING_KEY")
        .context("FONTES_DATA_SIGNING_KEY is required for publication")?;
    let seed: [u8; 32] = hex::decode(seed.trim())
        .context("invalid signing seed encoding")?
        .try_into()
        .map_err(|_| anyhow::anyhow!("signing seed must be 32 bytes"))?;
    let signing_key = SigningKey::from_bytes(&seed);
    ensure!(
        hex::encode(signing_key.verifying_key().to_bytes()) == trust.public_key_hex,
        "signing secret does not match trusted public key"
    );
    let bytes = fs::read(dir.join("release-manifest.json"))?;
    fs::write(
        dir.join("release-manifest.json.sig"),
        signing_key.sign(&signing_message(&bytes)).to_bytes(),
    )?;
    verify_package(dir, trust_path, repository)?;
    let lock: SourceLock = serde_json::from_slice(&fs::read(dir.join("sources.lock.json"))?)?;
    let mut assets = vec![
        "release-manifest.json".to_owned(),
        "release-manifest.json.sig".into(),
        "sources.lock.json".into(),
        "validation-report.json".into(),
        "changes.json".into(),
        "NOTICE.txt".into(),
    ];
    for profile in &manifest.profiles {
        assets.push(profile.asset.clone());
    }
    if additional.is_some() {
        assets.push("validation-report-study.json".into());
    }
    if manifest.profiles.iter().any(|p| p.profile == "study") {
        assets.push("study-imports.json".into());
    }
    if manifest
        .sources
        .iter()
        .any(|s| s.source_id == "kjv-strongs")
    {
        assets.extend(["GPL-3.0.txt".into(), "KJV-ANNOTATIONS-NOTICE.md".into()]);
    }
    for snapshot in &manifest.sources {
        assets.push(source_asset(
            lock.sources
                .iter()
                .find(|s| s.source_id == snapshot.source_id)
                .context("missing source")?,
        ));
    }
    let notes = format!(
        "Fontes data {}\n\n{}\nCompiler commit: {}\nManifest: Ed25519 signed with {}\n\nThe complete free-resource package contains KJV and WEB Classic; Strong's Greek/Hebrew dictionaries; Abbott-Smith's Greek Lexicon (1922); Easton's Bible Dictionary (1897); Robinson TR Greek morphology; OSHB Hebrew words/morphology (CC BY 4.0); edition-bound KJV dictionary tags (GPL); Matthew Henry; Nave's; and TSK. The internal study identifier is retained for compatibility. New releases no longer provide a separate Reading download; historical releases remain unchanged. See NOTICE.txt, source locks, validation reports, study-imports.json, and changes.json. Verify with the public key distributed with the application.\n",
        manifest.release_tag,
        manifest
            .profiles
            .iter()
            .map(|p| format!(
                "- {}: {} resources, {} compressed bytes",
                p.profile, p.resource_count, p.compressed_bytes
            ))
            .collect::<Vec<_>>()
            .join("\n"),
        manifest.builder_commit,
        trust.key_id
    );
    let notes_path = dir.join("release-notes.md");
    fs::write(&notes_path, notes)?;
    let paths: Vec<_> = assets.iter().map(|name| dir.join(name)).collect();
    let mut args = vec![
        "release",
        "create",
        &manifest.release_tag,
        "--repo",
        repository,
        "--draft",
        "--latest=false",
        "--target",
        &manifest.builder_commit,
        "--title",
        &manifest.release_tag,
        "--notes-file",
        notes_path.to_str().context("non-UTF8 notes path")?,
    ];
    for path in &paths {
        args.push(path.to_str().context("non-UTF8 asset path")?);
    }
    gh(&args)?;
    for publish in [false, true] {
        if publish {
            gh(&[
                "release",
                "edit",
                &manifest.release_tag,
                "--repo",
                repository,
                "--draft=false",
                "--latest=false",
            ])?;
        }
        let downloaded = tempfile::tempdir()?;
        download_assets(repository, &manifest.release_tag, downloaded.path(), &["*"])?;
        for name in &assets {
            ensure!(
                file_digest(&downloaded.path().join(name))? == file_digest(&dir.join(name))?,
                "uploaded release asset changed: {name}"
            );
        }
        verify_package(downloaded.path(), trust_path, repository)?;
    }
    println!(
        "Published and verified https://github.com/{repository}/releases/tag/{}",
        manifest.release_tag
    );
    Ok(())
}

#[cfg(test)]
mod default_bundle_tests {
    use super::*;
    fn profiles() -> (ProfileArtifact, ProfileArtifact) {
        let manifest: ReleaseManifest =
            serde_json::from_str(include_str!("../../../data/examples/release-manifest.json"))
                .unwrap();
        let reading = manifest.profiles[0].clone();
        let mut complete = reading.clone();
        complete.profile = "study".into();
        complete.asset = "fontes-study.sqlite.zst".into();
        complete.source_ids = fontes_data_contracts::profile_sources("study")
            .unwrap()
            .iter()
            .map(|s| s.to_string())
            .collect();
        complete.capabilities = fontes_data_contracts::profile_capabilities("study")
            .unwrap()
            .to_vec();
        (reading, complete)
    }
    #[test]
    fn sole_complete_default_can_retire_reading_but_not_other_resources() {
        let (reading, complete) = profiles();
        let mut historical = complete.clone();
        historical
            .source_ids
            .retain(|s| !matches!(s.as_str(), "abbott-smith" | "easton"));
        assert_eq!(
            check_profile_transition(
                &[reading.clone(), historical.clone()],
                std::slice::from_ref(&complete)
            )
            .unwrap(),
            ["reading"]
        );
        assert!(
            check_profile_transition(&[historical.clone()], std::slice::from_ref(&reading))
                .is_err()
        );
        assert!(
            check_profile_transition(
                &[reading.clone(), historical.clone()],
                &[historical.clone()]
            )
            .is_err()
        );
        let mut partial = complete.clone();
        partial.source_ids.retain(|s| s != "easton");
        assert!(
            check_profile_transition(&[reading.clone(), historical.clone()], &[partial]).is_err()
        );
        assert!(
            check_profile_transition(&[reading.clone(), historical.clone()], &[reading, complete])
                .unwrap()
                .is_empty()
        );
        let mut unknown = historical;
        unknown.profile = "optional-resource".into();
        assert!(check_profile_transition(&[unknown], std::slice::from_ref(&profiles().1)).is_err());
    }
}
