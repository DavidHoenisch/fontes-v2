//! Submission/source publication only; these releases are not app delivery packages.
use crate::{ValidationOptions, input::safe_path, pack, validate};
use anyhow::{Context, Result, bail, ensure};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
};

const DOMAIN: &[u8] = b"FONTES-RESOURCE-SOURCE-RELEASE-V1\n";
const MAX_DOWNLOAD: u64 = 256 * 1024 * 1024;
const MANIFEST: &str = "resource-source-manifest.json";
const SIGNATURE: &str = "resource-source-manifest.json.sig";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Distribution {
    PublicFree,
    PrivateRestricted,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub policy_version: u32,
    pub distribution: Distribution,
    pub publishers: BTreeSet<String>,
    pub source_hosts: BTreeSet<String>,
    pub taxonomy: String,
    pub trust: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Source {
    Path {
        path: String,
    },
    Https {
        url: String,
        sha256: String,
        bytes: u64,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Submission {
    pub submission_version: u32,
    pub resource_id: String,
    pub publisher_id: String,
    pub version: String,
    pub build_revision: u32,
    pub source: Source,
    pub release_notes: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Approval {
    pub approval_version: u32,
    pub resource_id: String,
    pub version: String,
    pub source_sha256: String,
    pub distribution: Distribution,
    pub rights_review_id: String,
    pub publisher_review_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Trust {
    pub key_id: String,
    pub public_key_hex: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceManifest {
    pub source_release_protocol: u32,
    pub artifact_kind: String,
    pub repository: String,
    pub source_repository: String,
    pub source_commit: String,
    pub release_commit: String,
    pub tooling_commit: String,
    pub release_tag: String,
    pub resource_id: String,
    pub publisher_id: String,
    pub version: String,
    pub build_revision: u32,
    pub distribution: Distribution,
    pub signing_key_id: String,
    pub source_archive: String,
    pub source_sha256: String,
    pub taxonomy_sha256: String,
    pub policy_sha256: String,
    pub approved: bool,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Serialize)]
pub struct SubmissionResult {
    pub resource_id: String,
    pub status: String,
    pub approved: bool,
    pub message: String,
}
#[derive(Debug, Serialize)]
pub struct BatchReport {
    pub report_version: u32,
    pub valid: bool,
    pub submissions: Vec<SubmissionResult>,
}

pub struct BuildOptions<'a> {
    pub root: &'a Path,
    pub policy_root: &'a Path,
    pub output: &'a Path,
    pub repository: &'a str,
    pub source_repository: &'a str,
    pub source_commit: &'a str,
    pub tooling_commit: &'a str,
    pub approved_only: bool,
}

pub fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
fn file_digest(path: &Path) -> Result<String> {
    let mut hash = Sha256::new();
    std::io::copy(&mut fs::File::open(path)?, &mut hash)?;
    Ok(hex::encode(hash.finalize()))
}
fn read(path: &Path, limit: u64) -> Result<Vec<u8>> {
    ensure!(
        fs::symlink_metadata(path)?.file_type().is_file(),
        "expected a regular file: {}",
        path.display()
    );
    let mut bytes = vec![];
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= limit,
        "file exceeds size limit: {}",
        path.display()
    );
    Ok(bytes)
}
fn json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    serde_json::from_slice(&read(path, 1024 * 1024)?)
        .with_context(|| format!("invalid JSON contract: {}", path.display()))
}
fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}
fn hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn commit(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn namespace(value: &str, minimum: usize) -> bool {
    value.len() <= 160
        && value.split('.').count() >= minimum
        && value.split('.').all(|part| {
            part.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}
fn version(value: &str) -> bool {
    value.len() <= 40
        && value.split('.').count() == 3
        && value.split('.').all(|v| {
            !v.is_empty()
                && (v == "0" || !v.starts_with('0'))
                && v.bytes().all(|b| b.is_ascii_digit())
        })
}
fn repository(value: &str) -> bool {
    value.split('/').count() == 2
        && value.split('/').all(|p| {
            !p.is_empty()
                && p != "."
                && p != ".."
                && p.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
}
fn inside(root: &Path, relative: &str) -> Result<PathBuf> {
    ensure!(
        safe_path(relative),
        "unsafe configured source/policy path: {relative}"
    );
    let root = fs::canonicalize(root)?;
    let path = fs::canonicalize(root.join(relative))?;
    ensure!(
        path.starts_with(&root),
        "path escapes its configured root: {relative}"
    );
    Ok(path)
}
fn policy(root: &Path) -> Result<(Policy, Trust)> {
    let policy: Policy = json(&root.join("policy.json"))?;
    ensure!(
        policy.policy_version == 1
            && !policy.publishers.is_empty()
            && policy.publishers.iter().all(|p| namespace(p, 2)),
        "unsupported/invalid publishing policy"
    );
    inside(root, &policy.taxonomy)?;
    let trust: Trust = json(&inside(root, &policy.trust)?)?;
    ensure!(
        !trust.key_id.is_empty() && hash(&trust.public_key_hex),
        "invalid source release trust key"
    );
    Ok((policy, trust))
}
pub fn tag(id: &str, version: &str, revision: u32) -> String {
    format!("resource-source-{id}--{version}--b{revision}")
}

fn acquire(source: &Source, root: &Path, policy: &Policy, scratch: &Path) -> Result<PathBuf> {
    match source {
        Source::Path { path } => inside(root, path),
        Source::Https { url, sha256, bytes } => {
            ensure!(
                policy.distribution == Distribution::PublicFree,
                "restricted sources must remain in the private submission repository"
            );
            ensure!(
                hash(sha256) && *bytes > 0 && *bytes <= MAX_DOWNLOAD,
                "invalid source download digest/size"
            );
            let client = reqwest::blocking::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(120))
                .build()?;
            let mut url = reqwest::Url::parse(url)?;
            for _ in 0..6 {
                ensure!(
                    url.scheme() == "https"
                        && url.username().is_empty()
                        && url.password().is_none()
                        && url.fragment().is_none()
                        && url.port_or_known_default() == Some(443)
                        && url
                            .host_str()
                            .is_some_and(|h| policy.source_hosts.contains(h)),
                    "source URL/redirect is outside the trusted HTTPS host allowlist"
                );
                let response = client
                    .get(url.clone())
                    .header("User-Agent", "Fontes-Resource-Submission/1")
                    .send()?;
                if response.status().is_redirection() {
                    url = url.join(
                        response
                            .headers()
                            .get(reqwest::header::LOCATION)
                            .context("redirect lacks Location")?
                            .to_str()?,
                    )?;
                    continue;
                }
                let response = response.error_for_status()?;
                if let Some(length) = response.content_length() {
                    ensure!(length == *bytes, "source HTTP size differs from descriptor");
                }
                let path = scratch.join("submitted.zip");
                let mut file = fs::File::create(&path)?;
                let length = std::io::copy(&mut response.take(bytes + 1), &mut file)?;
                file.sync_all()?;
                ensure!(
                    length == *bytes && file_digest(&path)? == *sha256,
                    "submitted source hash/length mismatch"
                );
                return Ok(path);
            }
            bail!("source redirect limit exceeded")
        }
    }
}

pub fn build(options: &BuildOptions<'_>) -> Result<BatchReport> {
    ensure!(
        !options.output.exists(),
        "output already exists; choose a new build directory"
    );
    ensure!(
        repository(options.repository)
            && repository(options.source_repository)
            && commit(options.source_commit)
            && commit(options.tooling_commit),
        "invalid repository/commit provenance"
    );
    let (policy, trust) = policy(options.policy_root)?;
    let mut descriptors = vec![];
    for entry in fs::read_dir(options.root.join("submissions"))? {
        ensure!(descriptors.len() < 100, "at most 100 submissions per run");
        let entry = entry?;
        ensure!(
            entry.file_type()?.is_dir(),
            "submission entries must be real directories"
        );
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| anyhow::anyhow!("submission name is not UTF-8"))?;
        ensure!(namespace(&name, 3), "invalid submission directory name");
        descriptors.push((name, entry.path().join("submission.json")));
    }
    descriptors.sort();
    fs::create_dir_all(options.output)?;
    let mut batch = BatchReport {
        report_version: 1,
        valid: true,
        submissions: vec![],
    };
    for (id, path) in descriptors {
        let result = (|| -> Result<SubmissionResult> {
            let submission: Submission = json(&path)?;
            ensure!(
                submission.submission_version == 1
                    && submission.resource_id == id
                    && namespace(&submission.publisher_id, 2)
                    && id.starts_with(&format!("{}.", submission.publisher_id))
                    && version(&submission.version)
                    && submission.build_revision > 0
                    && !submission.release_notes.trim().is_empty()
                    && submission.release_notes.len() <= 4000,
                "invalid submission identity/version/notes"
            );
            let approval_path = options
                .policy_root
                .join("approvals")
                .join(format!("{id}--{}.json", submission.version));
            if options.approved_only && !approval_path.exists() {
                return Ok(SubmissionResult {
                    resource_id: id.clone(),
                    status: "awaiting_approval".into(),
                    approved: false,
                    message: "No maintainer approval for this version; no release prepared.".into(),
                });
            }
            let target = options.output.join(&id);
            fs::create_dir(&target)?;
            let temp = tempfile::tempdir()?;
            let source = acquire(&submission.source, options.root, &policy, temp.path())?;
            let checks = ValidationOptions {
                taxonomy: Some(inside(options.policy_root, &policy.taxonomy)?),
                deny_warnings: true,
            };
            let report = validate(&source, &checks)?;
            write_json(&target.join("validation-report.json"), &report)?;
            ensure!(report.valid, "{}", report.text());
            ensure!(
                report.resource_id.as_deref() == Some(&id)
                    && report.resource_version.as_deref() == Some(&submission.version),
                "submission descriptor differs from source metadata identity/version"
            );
            let source_name = format!("{id}--{}.fontes-source.zip", submission.version);
            let archive = pack(&source, &target.join(&source_name), &checks)?;
            ensure!(
                archive.bytes <= MAX_DOWNLOAD,
                "normalized source archive exceeds 256 MiB release limit"
            );
            // Revalidate the canonical frozen artifact, not a second mutable source read.
            let frozen = validate(&target.join(&source_name), &checks)?;
            ensure!(frozen.valid, "canonical archive failed revalidation");
            write_json(&target.join("validation-report.json"), &frozen)?;
            let approval = if approval_path.exists() {
                let approval: Approval = json(&approval_path)?;
                ensure!(
                    approval.approval_version == 1
                        && approval.resource_id == id
                        && approval.version == submission.version
                        && approval.source_sha256 == archive.sha256
                        && approval.distribution == policy.distribution
                        && !approval.rights_review_id.trim().is_empty()
                        && !approval.publisher_review_id.trim().is_empty()
                        && policy.publishers.contains(&submission.publisher_id),
                    "maintainer approval does not match exact source hash, publisher, version, or distribution; use a new resource version for changed bytes"
                );
                write_json(&target.join("approval.json"), &approval)?;
                Some(approval)
            } else {
                None
            };
            let mut zip = zip::ZipArchive::new(fs::File::open(target.join(&source_name))?)?;
            let mut metadata = String::new();
            zip.by_name("metadata.xml")?.read_to_string(&mut metadata)?;
            let doc = roxmltree::Document::parse(&metadata)?;
            ensure!(
                doc.descendants()
                    .find(|n| n.has_tag_name("publisher"))
                    .and_then(|n| n.attribute("id"))
                    == Some(submission.publisher_id.as_str()),
                "publisher differs from source metadata"
            );
            let cover = doc
                .descendants()
                .find(|n| n.has_tag_name("cover"))
                .context("missing cover")?;
            let cover_path = cover.attribute("href").context("missing cover path")?;
            let cover_name = if cover_path.ends_with(".jpg") {
                "cover.jpg"
            } else {
                "cover.png"
            };
            let mut bytes = vec![];
            zip.by_name(cover_path)?.read_to_end(&mut bytes)?;
            fs::write(target.join(cover_name), &bytes)?;
            let image = image::load_from_memory(&bytes)?;
            for (name, width, height) in
                [("cover-card.png", 200, 300), ("cover-detail.png", 600, 900)]
            {
                image
                    .resize_exact(width, height, image::imageops::FilterType::Triangle)
                    .save(target.join(name))?;
            }
            fs::write(target.join("metadata.xml"), &metadata)?;
            let mut notice = String::from(
                "Fontes source-package release. Not an app-installable delivery payload.\n\n",
            );
            for path in ["rights/LICENSE.txt", "rights/ATTRIBUTION.txt"] {
                zip.by_name(path)?.read_to_string(&mut notice)?;
                notice.push('\n');
            }
            notice.push_str(&format!(
                "\nCover license: {}\nCover rights holder: {}\nCover attribution: {}\n",
                cover.attribute("license-id").unwrap_or(""),
                cover.attribute("rights-holder").unwrap_or(""),
                cover.attribute("attribution").unwrap_or("")
            ));
            fs::write(target.join("NOTICE.txt"), notice)?;
            write_json(&target.join("submission.json"), &submission)?;
            let mut entries = vec![];
            for entry in fs::read_dir(&target)? {
                let path = entry?.path();
                entries.push(Entry {
                    path: path
                        .file_name()
                        .unwrap()
                        .to_str()
                        .context("asset name")?
                        .into(),
                    bytes: path.metadata()?.len(),
                    sha256: file_digest(&path)?,
                });
            }
            entries.sort_by(|a, b| a.path.cmp(&b.path));
            write_json(
                &target.join(MANIFEST),
                &SourceManifest {
                    source_release_protocol: 1,
                    artifact_kind: "validated_source_package".into(),
                    repository: options.repository.into(),
                    source_repository: options.source_repository.into(),
                    source_commit: options.source_commit.into(),
                    release_commit: options.source_commit.into(),
                    tooling_commit: options.tooling_commit.into(),
                    release_tag: tag(&id, &submission.version, submission.build_revision),
                    resource_id: id.clone(),
                    publisher_id: submission.publisher_id,
                    version: submission.version,
                    build_revision: submission.build_revision,
                    distribution: policy.distribution.clone(),
                    signing_key_id: trust.key_id.clone(),
                    source_archive: source_name,
                    source_sha256: archive.sha256,
                    taxonomy_sha256: file_digest(&inside(options.policy_root, &policy.taxonomy)?)?,
                    policy_sha256: file_digest(&options.policy_root.join("policy.json"))?,
                    approved: approval.is_some(),
                    entries,
                },
            )?;
            Ok(SubmissionResult {
                resource_id: id.clone(),
                status: if approval.is_some() {
                    "ready"
                } else {
                    "awaiting_approval"
                }
                .into(),
                approved: approval.is_some(),
                message: if approval.is_some() {
                    "Validated, hash-approved, and prepared."
                } else {
                    "Validated preview; maintainer approval required before publication."
                }
                .into(),
            })
        })();
        match result {
            Ok(result) => batch.submissions.push(result),
            Err(error) => {
                batch.valid = false;
                batch.submissions.push(SubmissionResult {
                    resource_id: id,
                    status: "invalid".into(),
                    approved: false,
                    message: format!("{error:#}"),
                });
            }
        }
    }
    write_json(&options.output.join("submission-report.json"), &batch)?;
    Ok(batch)
}

fn verify_bytes(
    bytes: &[u8],
    signature: &[u8],
    trust: &Trust,
    expected_repository: &str,
) -> Result<SourceManifest> {
    ensure!(bytes.len() <= 1024 * 1024, "manifest exceeds size limit");
    let public: [u8; 32] = hex::decode(&trust.public_key_hex)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("public key must be 32 bytes"))?;
    VerifyingKey::from_bytes(&public)?
        .verify_strict(
            &[DOMAIN, bytes].concat(),
            &Signature::from_slice(signature)?,
        )
        .context("source-release signature verification failed")?;
    let manifest: SourceManifest = serde_json::from_slice(bytes)?;
    ensure!(
        manifest.source_release_protocol == 1
            && manifest.artifact_kind == "validated_source_package"
            && manifest.repository == expected_repository
            && repository(expected_repository)
            && repository(&manifest.source_repository)
            && manifest.signing_key_id == trust.key_id
            && manifest.approved
            && namespace(&manifest.resource_id, 3)
            && namespace(&manifest.publisher_id, 2)
            && manifest
                .resource_id
                .starts_with(&format!("{}.", manifest.publisher_id))
            && version(&manifest.version)
            && manifest.build_revision > 0
            && commit(&manifest.source_commit)
            && commit(&manifest.release_commit)
            && commit(&manifest.tooling_commit)
            && hash(&manifest.source_sha256)
            && hash(&manifest.taxonomy_sha256)
            && hash(&manifest.policy_sha256)
            && manifest.release_tag
                == tag(
                    &manifest.resource_id,
                    &manifest.version,
                    manifest.build_revision
                ),
        "source-release identity/protocol/approval mismatch"
    );
    ensure!(
        manifest.source_archive
            == format!(
                "{}--{}.fontes-source.zip",
                manifest.resource_id, manifest.version
            ),
        "invalid source archive identity"
    );
    let mut seen = BTreeSet::new();
    for entry in &manifest.entries {
        ensure!(
            safe_path(&entry.path)
                && !entry.path.contains('/')
                && !matches!(entry.path.as_str(), MANIFEST | SIGNATURE)
                && seen.insert(entry.path.to_ascii_lowercase())
                && hash(&entry.sha256)
                && entry.bytes > 0
                && entry.bytes <= MAX_DOWNLOAD,
            "invalid manifest payload entry"
        );
    }
    for required in [
        manifest.source_archive.as_str(),
        "metadata.xml",
        "validation-report.json",
        "submission.json",
        "approval.json",
        "cover-card.png",
        "cover-detail.png",
        "NOTICE.txt",
    ] {
        ensure!(
            seen.contains(&required.to_ascii_lowercase()),
            "missing required manifest asset: {required}"
        );
    }
    ensure!(
        seen.contains("cover.png") ^ seen.contains("cover.jpg"),
        "exactly one master cover is required"
    );
    ensure!(seen.len() == 9, "unknown source release assets");
    let archive = manifest
        .entries
        .iter()
        .find(|entry| entry.path == manifest.source_archive)
        .unwrap();
    ensure!(
        archive.sha256 == manifest.source_sha256,
        "source hash disagrees with archive entry"
    );
    Ok(manifest)
}

pub fn verify(dir: &Path, trust_path: &Path, expected_repository: &str) -> Result<SourceManifest> {
    let trust: Trust = json(trust_path)?;
    let manifest = verify_bytes(
        &read(&dir.join(MANIFEST), 1024 * 1024)?,
        &read(&dir.join(SIGNATURE), 64)?,
        &trust,
        expected_repository,
    )?;
    let expected: BTreeSet<_> = manifest
        .entries
        .iter()
        .map(|entry| entry.path.as_str())
        .chain([MANIFEST, SIGNATURE])
        .collect();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        ensure!(
            entry.file_type()?.is_file()
                && expected.contains(
                    entry
                        .file_name()
                        .to_str()
                        .context("invalid release asset name")?
                ),
            "unexpected/special release asset"
        );
    }
    for entry in &manifest.entries {
        let path = dir.join(&entry.path);
        ensure!(
            path.metadata()?.len() == entry.bytes && file_digest(&path)? == entry.sha256,
            "release asset hash/length mismatch: {}",
            entry.path
        );
    }
    Ok(manifest)
}

fn gh(args: &[&str]) -> Result<String> {
    let output = Command::new("gh")
        .args(args)
        .output()
        .context("GitHub CLI is required")?;
    ensure!(
        output.status.success(),
        "GitHub {} failed: {}",
        args.first().unwrap_or(&"operation"),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?)
}
type Host = dyn Fn(&[&str]) -> Result<String>;

fn download(repository: &str, tag: &str, dir: &Path, all: bool, gh: &Host) -> Result<()> {
    let mut args = vec![
        "release",
        "download",
        tag,
        "--repo",
        repository,
        "--dir",
        dir.to_str().context("download path")?,
        "--pattern",
    ];
    if all {
        args.push("*");
    } else {
        args.push(MANIFEST);
        args.extend(["--pattern", SIGNATURE]);
    }
    gh(&args)?;
    Ok(())
}

/// Sign only approved trusted-build outputs, then verify draft/public downloads.
pub fn publish(dir: &Path, policy_root: &Path, expected_repository: &str) -> Result<Vec<String>> {
    let batch: serde_json::Value = json(&dir.join("submission-report.json"))?;
    ensure!(batch["valid"] == true, "refusing a failed submission build");
    if batch["submissions"]
        .as_array()
        .context("missing submission results")?
        .iter()
        .all(|s| s["approved"] != true)
    {
        return Ok(vec![]);
    }
    let seed = std::env::var("FONTES_RESOURCE_SIGNING_KEY")
        .context("FONTES_RESOURCE_SIGNING_KEY is required")?;
    let seed: [u8; 32] = hex::decode(seed.trim())
        .context("invalid signing seed encoding")?
        .try_into()
        .map_err(|_| anyhow::anyhow!("signing seed must be 32 bytes"))?;
    let key = SigningKey::from_bytes(&seed);
    publish_with(dir, policy_root, expected_repository, &key, &gh)
}

#[cfg(test)]
mod tests;

fn publish_with(
    dir: &Path,
    policy_root: &Path,
    expected_repository: &str,
    key: &SigningKey,
    gh: &Host,
) -> Result<Vec<String>> {
    let (policy, trust) = policy(policy_root)?;
    let batch: serde_json::Value = json(&dir.join("submission-report.json"))?;
    ensure!(batch["valid"] == true, "refusing a failed submission build");
    ensure!(
        repository(expected_repository),
        "invalid destination repository"
    );
    let destination: serde_json::Value = serde_json::from_str(&gh(&[
        "repo",
        "view",
        expected_repository,
        "--json",
        "isPrivate",
    ])?)?;
    if policy.distribution == Distribution::PrivateRestricted {
        ensure!(
            destination["isPrivate"] == true,
            "restricted resources cannot be published to a public repository"
        );
    }
    ensure!(
        hex::encode(key.verifying_key().to_bytes()) == trust.public_key_hex,
        "signing secret differs from configured public key"
    );
    let releases: Vec<serde_json::Value> = serde_json::from_str(&gh(&[
        "release",
        "list",
        "--repo",
        expected_repository,
        "--limit",
        "10000",
        "--json",
        "tagName,isDraft",
    ])?)?;
    ensure!(
        releases.len() < 10000,
        "release enumeration bound reached; refusing incomplete identity checks"
    );
    let mut urls = vec![];
    for resource in batch["submissions"]
        .as_array()
        .context("missing submission results")?
    {
        if resource["approved"] != true {
            continue;
        }
        let id = resource["resource_id"]
            .as_str()
            .context("missing resource id")?;
        ensure!(namespace(id, 3), "invalid result identity");
        let build = dir.join(id);
        let mut prepared: SourceManifest = json(&build.join(MANIFEST))?;
        prepared.release_commit = if prepared.source_repository == expected_repository {
            prepared.source_commit.clone()
        } else {
            gh(&[
                "api",
                &format!("repos/{expected_repository}/commits/HEAD"),
                "--jq",
                ".sha",
            ])?
            .trim()
            .to_owned()
        };
        ensure!(
            commit(&prepared.release_commit),
            "destination returned an invalid target commit"
        );
        write_json(&build.join(MANIFEST), &prepared)?;
        let bytes = read(&build.join(MANIFEST), 1024 * 1024)?;
        let signature = key.sign(&[DOMAIN, &bytes].concat()).to_bytes();
        fs::write(build.join(SIGNATURE), signature)?;
        let manifest = verify(
            &build,
            &inside(policy_root, &policy.trust)?,
            expected_repository,
        )?;
        ensure!(
            manifest.resource_id == id
                && manifest.distribution == policy.distribution
                && manifest.policy_sha256 == file_digest(&policy_root.join("policy.json"))?
                && manifest.taxonomy_sha256
                    == file_digest(&inside(policy_root, &policy.taxonomy)?)?
                && policy.publishers.contains(&manifest.publisher_id),
            "build does not match trusted publication policy"
        );
        let approval: Approval = json(&inside(
            policy_root,
            &format!("approvals/{id}--{}.json", manifest.version),
        )?)?;
        ensure!(
            serde_json::to_value(&approval)?
                == serde_json::from_slice::<serde_json::Value>(&read(
                    &build.join("approval.json"),
                    1024 * 1024
                )?)?
                && approval.source_sha256 == manifest.source_sha256
                && approval.distribution == manifest.distribution,
            "build does not match current hash approval"
        );
        if policy.distribution == Distribution::PrivateRestricted {
            let source: serde_json::Value = serde_json::from_str(&gh(&[
                "repo",
                "view",
                &manifest.source_repository,
                "--json",
                "isPrivate",
            ])?)?;
            ensure!(
                source["isPrivate"] == true,
                "restricted submissions must originate from a private repository"
            );
        }
        let prefix = format!("resource-source-{id}--{}--b", manifest.version);
        let mut existing = false;
        for release in releases.iter().filter(|r| {
            r["tagName"]
                .as_str()
                .is_some_and(|t| t.starts_with(&prefix))
        }) {
            let previous_tag = release["tagName"].as_str().unwrap();
            let previous = tempfile::tempdir()?;
            download(expected_repository,previous_tag,previous.path(),false,gh).context("existing draft may be incomplete; inspect it and retry using a new build_revision, never overwrite published assets")?;
            let old = verify_bytes(
                &read(&previous.path().join(MANIFEST), 1024 * 1024)?,
                &read(&previous.path().join(SIGNATURE), 64)?,
                &trust,
                expected_repository,
            )?;
            ensure!(
                old.source_sha256 == manifest.source_sha256
                    && old.resource_id == id
                    && old.version == manifest.version
                    && old.release_tag == previous_tag,
                "resource version was already reserved/published with different source bytes; increment resource version"
            );
            if previous_tag == manifest.release_tag {
                ensure!(
                    old.distribution == manifest.distribution
                        && old.taxonomy_sha256 == manifest.taxonomy_sha256
                        && old.policy_sha256 == manifest.policy_sha256
                        && old.entries == manifest.entries,
                    "existing build identity has different payload/policy; increment build_revision"
                );
                let all = tempfile::tempdir()?;
                download(expected_repository, previous_tag, all.path(), true, gh)?;
                verify(
                    all.path(),
                    &inside(policy_root, &policy.trust)?,
                    expected_repository,
                )?;
                if release["isDraft"] == true {
                    gh(&[
                        "release",
                        "edit",
                        previous_tag,
                        "--repo",
                        expected_repository,
                        "--draft=false",
                        "--latest=false",
                    ])?;
                    let public = tempfile::tempdir()?;
                    download(expected_repository, previous_tag, public.path(), true, gh)?;
                    verify(
                        public.path(),
                        &inside(policy_root, &policy.trust)?,
                        expected_repository,
                    )?;
                }
                existing = true;
            }
        }
        if !existing {
            let notes = tempfile::NamedTempFile::new()?;
            let submission: Submission = json(&build.join("submission.json"))?;
            fs::write(
                notes.path(),
                format!(
                    "Validated publisher source package (preview), not an app-installable SQLite delivery package.\n\n{}\n\nSource: {} @ {}\nTooling: {}\nRights review: {}\nVerify the detached source-release manifest using the configured public key.\n",
                    submission.release_notes,
                    manifest.source_repository,
                    manifest.source_commit,
                    manifest.tooling_commit,
                    approval.rights_review_id
                ),
            )?;
            let mut args = vec![
                "release",
                "create",
                &manifest.release_tag,
                "--repo",
                expected_repository,
                "--target",
                &manifest.release_commit,
                "--draft",
                "--prerelease",
                "--latest=false",
                "--title",
                &manifest.release_tag,
                "--notes-file",
                notes.path().to_str().context("notes path")?,
            ];
            let mut paths: Vec<_> = manifest
                .entries
                .iter()
                .map(|entry| build.join(&entry.path))
                .collect();
            paths.extend([build.join(MANIFEST), build.join(SIGNATURE)]);
            for path in &paths {
                args.push(path.to_str().context("asset path")?);
            }
            gh(&args)?;
            for public in [false, true] {
                if public {
                    gh(&[
                        "release",
                        "edit",
                        &manifest.release_tag,
                        "--repo",
                        expected_repository,
                        "--draft=false",
                        "--latest=false",
                    ])?;
                }
                let downloaded = tempfile::tempdir()?;
                download(
                    expected_repository,
                    &manifest.release_tag,
                    downloaded.path(),
                    true,
                    gh,
                )?;
                verify(
                    downloaded.path(),
                    &inside(policy_root, &policy.trust)?,
                    expected_repository,
                )?;
            }
        }
        urls.push(format!(
            "https://github.com/{expected_repository}/releases/tag/{}",
            manifest.release_tag
        ));
    }
    Ok(urls)
}
