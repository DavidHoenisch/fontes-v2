//! Verified, cancellable first-run snapshot download. No compiler or UI dependency.
use crate::Engine;
use anyhow::{Context, Result, bail, ensure};
use ed25519_dalek::{Signature, VerifyingKey};
use fontes_data_contracts::{Channel, ProfileArtifact, ReleaseManifest};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    future::Future,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

pub const SETUP_OPERATION: &str = "library.setup";
pub const REPOSITORY: &str = "DavidHoenisch/fontes-v2";
const MIN_SEQUENCE: u64 = 4;
const MAX_COMPRESSED: u64 = 256 * 1024 * 1024;
const MAX_SQLITE: u64 = 1024 * 1024 * 1024;
const TRUST: &str = include_str!("../../../data/trust/data-release-1.json");
const DOMAIN: &[u8] = b"FONTES-DATA-MANIFEST-V1\n";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    Reading,
    #[default]
    Study,
}
impl Profile {
    pub fn id(self) -> &'static str {
        match self {
            Self::Reading => "reading",
            Self::Study => "study",
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Progress {
    pub message: String,
    pub completed: u64,
    pub total: Option<u64>,
    pub offer: Option<Offer>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Offer {
    pub download_bytes: u64,
    pub installed_bytes: u64,
    pub temporary_bytes: u64,
}
pub enum Event {
    Progress(Progress),
    Finished(std::result::Result<PathBuf, String>),
}
pub struct Job {
    pub events: mpsc::Receiver<Event>,
    cancel: Arc<AtomicBool>,
    approval: Arc<AtomicBool>,
}
impl Job {
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
    pub fn approve(&self) {
        self.approval.store(true, Ordering::Relaxed);
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel();
    }
}
pub fn start(data_dir: PathBuf, profile: Profile, wake: impl Fn() + Send + 'static) -> Job {
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    let approval = Arc::new(AtomicBool::new(false));
    let approve = approval.clone();
    let (tx, rx) = mpsc::sync_channel(16);
    std::thread::spawn(move || {
        let result = run_install(&data_dir, profile, &flag, Some(&approve), |p| {
            let critical = p.offer.is_some();
            let event = Event::Progress(p);
            if critical {
                if tx.send(event).is_ok() {
                    wake();
                }
            } else if tx.try_send(event).is_ok() {
                wake();
            }
        })
        .map_err(|e| format!("{e:#}"));
        if tx.send(Event::Finished(result)).is_ok() {
            wake();
        }
    });
    Job {
        events: rx,
        cancel,
        approval,
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Trust {
    key_id: String,
    public_key_hex: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    format_version: u32,
    manifest: String,
    signature_hex: String,
    profile: Profile,
    generation: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HighWater {
    format_version: u32,
    repository: String,
    channel: Channel,
    schema: u32,
    protocol: u32,
    sequence: u64,
    manifest_sha256: String,
}
#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
}
fn hash(bytes: &[u8]) -> String {
    hex::encode(ring::digest::digest(&ring::digest::SHA256, bytes).as_ref())
}
fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    ensure!(
        !cancel.load(Ordering::Relaxed),
        "Download cancelled. You can start again whenever you’re ready."
    );
    Ok(())
}
fn report(progress: &impl Fn(Progress), message: &str, completed: u64, total: Option<u64>) {
    progress(Progress {
        message: message.into(),
        completed,
        total,
        offer: None,
    });
}
fn verify(bytes: &[u8], signature: &[u8], trust: &Trust) -> Result<ReleaseManifest> {
    ensure!(
        bytes.len() <= 65536 && signature.len() == 64,
        "Invalid release metadata size."
    );
    let public: [u8; 32] = hex::decode(&trust.public_key_hex)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid app trust key."))?;
    VerifyingKey::from_bytes(&public)?
        .verify_strict(
            &[DOMAIN, bytes].concat(),
            &Signature::from_slice(signature)?,
        )
        .context("The library release signature could not be verified. Please retry later.")?;
    let m: ReleaseManifest = serde_json::from_slice(bytes)?;
    m.validate().map_err(anyhow::Error::msg)?;
    ensure!(
        m.repository == REPOSITORY
            && m.channel == Channel::Stable
            && m.signing_key_id == trust.key_id
            && m.sequence >= MIN_SEQUENCE,
        "This release is not trusted by this version of Fontes."
    );
    for p in &m.profiles {
        ensure!(
            fontes_data_contracts::supported_profile(p),
            "Unexpected library sources or capabilities. Update Fontes to continue."
        );
        ensure!(
            p.compressed_bytes <= MAX_COMPRESSED && p.sqlite_bytes <= MAX_SQLITE,
            "Library exceeds this app’s supported size."
        );
    }
    Ok(m)
}
fn read_limited(path: &Path, max: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?.take(max + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= max,
        "Setup metadata exceeds its size limit."
    );
    Ok(bytes)
}
fn root(data_dir: &Path) -> PathBuf {
    data_dir.join("snapshots-v1")
}
fn receipt_path(data_dir: &Path) -> PathBuf {
    root(data_dir).join("active.json")
}
fn artifact(m: &ReleaseManifest, profile: Profile) -> Result<&ProfileArtifact> {
    m.profiles
        .iter()
        .find(|p| p.profile == profile.id())
        .context("This release does not contain the selected library.")
}
fn parse_receipt(data_dir: &Path) -> Result<(Receipt, ReleaseManifest, PathBuf)> {
    let trust: Trust = serde_json::from_str(TRUST)?;
    parse_receipt_with(data_dir, &trust)
}
fn parse_receipt_with(
    data_dir: &Path,
    trust: &Trust,
) -> Result<(Receipt, ReleaseManifest, PathBuf)> {
    let receipt: Receipt =
        serde_json::from_slice(&read_limited(&receipt_path(data_dir), 150_000)?)?;
    ensure!(
        receipt.format_version == 1,
        "Unsupported local setup receipt."
    );
    let manifest = verify(
        receipt.manifest.as_bytes(),
        &hex::decode(&receipt.signature_hex)?,
        trust,
    )?;
    let p = artifact(&manifest, receipt.profile)?;
    let nonce = receipt
        .generation
        .strip_prefix(&format!("{}-", p.sqlite_sha256))
        .context("Invalid library generation.")?;
    ensure!(
        nonce.len() == 32 && nonce.bytes().all(|b| b.is_ascii_hexdigit()),
        "Invalid library generation."
    );
    let path = root(data_dir)
        .join("generations")
        .join(&receipt.generation)
        .join("library.sqlite");
    ensure!(
        path.is_file() && path.metadata()?.len() == p.sqlite_bytes,
        "Your downloaded library is incomplete. Download it again to repair it."
    );
    Ok((receipt, manifest, path))
}
/// Fast startup lookup. Full hash/SQLite checks run on the engine worker before use.
pub fn installed_library(data_dir: &Path) -> Result<Option<PathBuf>> {
    if !receipt_path(data_dir).exists() {
        return Ok(None);
    }
    Ok(Some(parse_receipt(data_dir)?.2))
}
pub fn is_managed_path(data_dir: &Path, path: &Path) -> bool {
    path.starts_with(root(data_dir).join("generations"))
}
/// Authenticate managed content each time it is opened; never modifies user notes.
pub fn verify_installed(data_dir: &Path) -> Result<PathBuf> {
    let (receipt, m, path) = parse_receipt(data_dir)?;
    let p = artifact(&m, receipt.profile)?;
    verify_file(
        &path,
        p.sqlite_bytes,
        &p.sqlite_sha256,
        &AtomicBool::new(false),
    )?;
    // These exact signed bytes were fully checked before activation. The engine
    // checks schema/build/registry again when opening; repeating a full integrity
    // scan on every phone launch adds no protection after the hash matches.
    Ok(path)
}
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().context("Missing setup directory.")?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    tmp.persist(path).map_err(|e| e.error)?;
    sync_dir(parent)?;
    Ok(())
}
fn sync_dir(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        File::open(path)?.sync_all()?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}
fn try_lock_setup(file: &File) -> std::io::Result<()> {
    // Rust 1.93's std::fs locks return Unsupported on Android. Use the
    // Android-supported flock syscall through rustix's safe Rust API instead.
    // The descriptor owns the lock until close, including on process death.
    #[cfg(target_os = "android")]
    {
        rustix::fs::flock(file, rustix::fs::FlockOperation::NonBlockingLockExclusive)
            .map_err(std::io::Error::from)
    }
    #[cfg(not(target_os = "android"))]
    {
        file.try_lock().map_err(std::io::Error::from)
    }
}
fn setup_lock_error(error: std::io::Error) -> anyhow::Error {
    let message = if error.kind() == std::io::ErrorKind::WouldBlock {
        "Library setup is already running. Wait for it to finish, then retry."
    } else {
        "Could not lock library storage. Check app storage access, then retry."
    };
    anyhow::Error::new(error).context(message)
}
fn verify_file(path: &Path, length: u64, expected: &str, cancel: &AtomicBool) -> Result<()> {
    ensure!(
        path.metadata()?.len() == length,
        "The downloaded library has an incorrect size."
    );
    let mut file = File::open(path)?;
    let mut sha = ring::digest::Context::new(&ring::digest::SHA256);
    let mut buffer = [0; 65536];
    let mut count = 0_u64;
    loop {
        check_cancel(cancel)?;
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        count += n as u64;
        ensure!(count <= length, "The library grew beyond its signed size.");
        sha.update(&buffer[..n]);
    }
    ensure!(
        hex::encode(sha.finish().as_ref()) == expected,
        "The downloaded library checksum did not match. Please retry."
    );
    Ok(())
}
fn validate_database(path: &Path, manifest: &ReleaseManifest, p: &ProfileArtifact) -> Result<()> {
    use rusqlite::{Connection, OpenFlags};
    let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    db.execute_batch("PRAGMA query_only=ON; PRAGMA trusted_schema=OFF;")?;
    let integrity: String = db.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    ensure!(integrity == "ok", "The library failed its integrity check.");
    ensure!(
        !db.prepare("PRAGMA foreign_key_check")?.exists([])?,
        "The library has broken references."
    );
    let (content, profile, lock, commit): (String, String, String, String) = db.query_row("SELECT content_id,profile,source_lock_sha256,builder_commit FROM build_info WHERE singleton=1", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?;
    ensure!(
        content == p.content_id
            && profile == p.profile
            && lock == manifest.source_lock_sha256
            && commit == manifest.builder_commit,
        "The library does not match its signed release."
    );
    for (table, expected) in [
        ("canonical_refs", u64::from(p.canonical_ref_count)),
        ("verse_texts", p.verse_text_count),
        ("resources", p.resource_count),
    ] {
        let count: u64 =
            db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))?;
        ensure!(
            count == expected,
            "The library’s {table} count does not match its release."
        );
    }
    drop(db);
    // Reuse the engine's full embedded canonical identity check with a temporary user store.
    let scratch = tempfile::tempdir_in(path.parent().context("Missing library directory.")?)?;
    let engine = Engine::open(path, &scratch.path().join("validation-user.sqlite"))?;
    engine.workspace()?;
    Ok(())
}
async fn cancellable<T>(cancel: &AtomicBool, future: impl Future<Output = T>) -> Result<T> {
    let mut future = std::pin::pin!(future);
    loop {
        tokio::select! {
            value = &mut future => return Ok(value),
            () = tokio::time::sleep(Duration::from_millis(100)) => check_cancel(cancel)?,
        }
    }
}
trait Transport {
    async fn get(&self, url: &str, cancel: &AtomicBool) -> Result<Body>;
}
enum Body {
    Http(reqwest::Response),
    #[cfg(test)]
    Memory(std::io::Cursor<Vec<u8>>),
}
impl Body {
    async fn chunk(&mut self, cancel: &AtomicBool) -> Result<Option<Vec<u8>>> {
        check_cancel(cancel)?;
        match self {
            Self::Http(response) => Ok(cancellable(cancel, response.chunk())
                .await??
                .map(|b| b.to_vec())),
            #[cfg(test)]
            Self::Memory(cursor) => {
                let mut bytes = vec![0; 65536];
                let n = cursor.read(&mut bytes)?;
                bytes.truncate(n);
                Ok((n > 0).then_some(bytes))
            }
        }
    }
}
struct Http(reqwest::Client);
impl Http {
    fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .https_only(true)
            .user_agent(concat!(
                "Fontes/",
                env!("CARGO_PKG_VERSION"),
                " library-setup"
            ))
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(20))
            .timeout(Duration::from_secs(20 * 60))
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                let allowed = matches!(
                    attempt.url().host_str(),
                    Some(
                        "github.com"
                            | "api.github.com"
                            | "release-assets.githubusercontent.com"
                            | "objects.githubusercontent.com"
                    )
                );
                if allowed
                    && attempt.url().scheme() == "https"
                    && attempt.url().port_or_known_default() == Some(443)
                    && attempt.url().username().is_empty()
                    && attempt.url().password().is_none()
                    && attempt.previous().len() < 5
                {
                    attempt.follow()
                } else {
                    attempt.error("Untrusted library download redirect.")
                }
            }))
            .build()?;
        Ok(Self(client))
    }
}
impl Transport for Http {
    async fn get(&self, url: &str, cancel: &AtomicBool) -> Result<Body> {
        let response = cancellable(
            cancel,
            self.0
                .get(url)
                .timeout(Duration::from_secs(if url.ends_with(".sqlite.zst") {
                    20 * 60
                } else {
                    30
                }))
                .send(),
        )
        .await?
        .context("Could not reach the library server. Check your connection and retry.")?
        .error_for_status()
        .context("The library server is unavailable or rate-limited. Please retry later.")?;
        Ok(Body::Http(response))
    }
}
async fn bytes(
    transport: &impl Transport,
    url: &str,
    max: usize,
    cancel: &AtomicBool,
) -> Result<Vec<u8>> {
    let mut body = transport.get(url, cancel).await?;
    let mut bytes = Vec::new();
    while let Some(chunk) = body.chunk(cancel).await? {
        ensure!(
            bytes.len() + chunk.len() <= max,
            "Release metadata is too large."
        );
        bytes.extend(chunk);
    }
    Ok(bytes)
}
struct Verified {
    bytes: Vec<u8>,
    signature: Vec<u8>,
    manifest: ReleaseManifest,
}
async fn discover(
    transport: &impl Transport,
    trust: &Trust,
    cancel: &AtomicBool,
) -> Result<Verified> {
    let mut candidates = BTreeSet::new();
    let mut complete = false;
    for page in 1..=10 {
        let url =
            format!("https://api.github.com/repos/{REPOSITORY}/releases?per_page=100&page={page}");
        let releases: Vec<Release> =
            serde_json::from_slice(&bytes(transport, &url, 2_000_000, cancel).await?)?;
        ensure!(releases.len() <= 100, "Invalid release listing.");
        for release in &releases {
            if !release.draft
                && !release.prerelease
                && let Some(sequence) = release
                    .tag_name
                    .strip_prefix("data-v1-stable-")
                    .and_then(|s| s.parse::<u64>().ok())
                && release.tag_name == format!("data-v1-stable-{sequence}")
            {
                candidates.insert(sequence);
            }
        }
        if releases.len() < 100 {
            complete = true;
            break;
        }
    }
    ensure!(
        complete,
        "Library discovery is incomplete. Please retry later."
    );
    for sequence in candidates.into_iter().rev().take(20) {
        check_cancel(cancel)?;
        let base =
            format!("https://github.com/{REPOSITORY}/releases/download/data-v1-stable-{sequence}");
        let candidate = async {
            let data = bytes(
                transport,
                &format!("{base}/release-manifest.json"),
                65536,
                cancel,
            )
            .await?;
            let signature = bytes(
                transport,
                &format!("{base}/release-manifest.json.sig"),
                64,
                cancel,
            )
            .await?;
            let manifest = verify(&data, &signature, trust)?;
            ensure!(
                manifest.sequence == sequence,
                "Release discovery identity mismatch."
            );
            Ok::<_, anyhow::Error>(Verified {
                bytes: data,
                signature,
                manifest,
            })
        }
        .await;
        if let Ok(candidate) = candidate {
            return Ok(candidate);
        }
    }
    check_cancel(cancel)?;
    bail!(
        "No compatible, verified library release is available. Check your connection or update Fontes, then retry."
    )
}
/// Verified metadata only: checking never downloads the database or changes active content.
#[derive(Clone, Debug, Serialize)]
pub struct UpdateInfo {
    pub installed_sequence: Option<u64>,
    pub available_sequence: u64,
    pub update_available: bool,
    pub download_bytes: u64,
    pub installed_bytes: u64,
    pub temporary_bytes: u64,
    pub manifest_sha256: String,
    pub release_url: String,
}
fn check_high_water(data_dir: &Path, verified: &Verified) -> Result<()> {
    let water_path = root(data_dir).join("high-water.json");
    if water_path.exists() {
        let previous: HighWater = serde_json::from_slice(&read_limited(&water_path, 4096)?)?;
        ensure!(
            previous.format_version == 1
                && previous.repository == REPOSITORY
                && previous.channel == Channel::Stable
                && previous.schema == 1
                && previous.protocol == 1
                && verified.manifest.sequence >= previous.sequence,
            "The server offered an older library release. Please retry later."
        );
        ensure!(
            verified.manifest.sequence != previous.sequence
                || previous.manifest_sha256 == hash(&verified.bytes),
            "This release changed after it was published."
        );
    }
    Ok(())
}
async fn check_with(
    data_dir: &Path,
    profile: Profile,
    cancel: &AtomicBool,
    transport: &impl Transport,
    trust: &Trust,
) -> Result<UpdateInfo> {
    let verified = discover(transport, trust, cancel).await?;
    check_high_water(data_dir, &verified)?;
    let p = artifact(&verified.manifest, profile)?;
    let installed = if receipt_path(data_dir).exists() {
        Some(parse_receipt_with(data_dir, trust)?)
    } else {
        None
    };
    if let Some((_, manifest, _)) = &installed {
        ensure!(
            verified.manifest.sequence >= manifest.sequence,
            "The server offered an older library release. Please retry later."
        );
    }
    let current = installed.as_ref().map(|(r, m, _)| (r, m));
    let same_content = current.is_some_and(|(r, m)| {
        artifact(m, r.profile).is_ok_and(|old| old.sqlite_sha256 == p.sqlite_sha256)
    });
    Ok(UpdateInfo {
        installed_sequence: current.map(|(_, m)| m.sequence),
        available_sequence: verified.manifest.sequence,
        update_available: !same_content,
        download_bytes: p.compressed_bytes,
        installed_bytes: p.sqlite_bytes,
        temporary_bytes: p.compressed_bytes + p.sqlite_bytes + 32 * 1024 * 1024,
        manifest_sha256: hash(&verified.bytes),
        release_url: format!(
            "https://github.com/{REPOSITORY}/releases/tag/{}",
            verified.manifest.release_tag
        ),
    })
}
pub fn check_update(data_dir: &Path, cancel: &AtomicBool) -> Result<UpdateInfo> {
    let http = Http::new()?;
    let trust: Trust = serde_json::from_str(TRUST)?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(check_with(
            data_dir,
            Profile::default(),
            cancel,
            &http,
            &trust,
        ))
}
/// Install only the exact signed release whose size the user reviewed.
pub fn install_update(
    data_dir: &Path,
    manifest_sha256: &str,
    cancel: &AtomicBool,
    progress: impl Fn(Progress),
) -> Result<PathBuf> {
    let http = Http::new()?;
    let trust: Trust = serde_json::from_str(TRUST)?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(install_with_expected(
            data_dir,
            Profile::default(),
            cancel,
            None,
            &progress,
            &http,
            &trust,
            Some(manifest_sha256),
        ))
}
/// Same setup service used by the GUI and headless CLI. Downloads only on request.
pub fn install(
    data_dir: &Path,
    profile: Profile,
    cancel: &AtomicBool,
    progress: impl Fn(Progress),
) -> Result<PathBuf> {
    run_install(data_dir, profile, cancel, None, progress)
}
fn run_install(
    data_dir: &Path,
    profile: Profile,
    cancel: &AtomicBool,
    approval: Option<&AtomicBool>,
    progress: impl Fn(Progress),
) -> Result<PathBuf> {
    let data_dir = std::path::absolute(data_dir)?;
    let http = Http::new()?;
    let trust: Trust = serde_json::from_str(TRUST)?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(install_with(
            &data_dir, profile, cancel, approval, &progress, &http, &trust,
        ))
}
async fn install_with(
    data_dir: &Path,
    profile: Profile,
    cancel: &AtomicBool,
    approval: Option<&AtomicBool>,
    progress: &impl Fn(Progress),
    transport: &impl Transport,
    trust: &Trust,
) -> Result<PathBuf> {
    install_with_expected(
        data_dir, profile, cancel, approval, progress, transport, trust, None,
    )
    .await
}
#[allow(clippy::too_many_arguments)]
async fn install_with_expected(
    data_dir: &Path,
    profile: Profile,
    cancel: &AtomicBool,
    approval: Option<&AtomicBool>,
    progress: &impl Fn(Progress),
    transport: &impl Transport,
    trust: &Trust,
    expected_manifest: Option<&str>,
) -> Result<PathBuf> {
    check_cancel(cancel)?;
    let root = root(data_dir);
    fs::create_dir_all(root.join("generations"))?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join("setup.lock"))
        .context("Could not open the library setup lock. Check app storage access, then retry.")?;
    try_lock_setup(&lock).map_err(setup_lock_error)?;
    // Only remove our abandoned staging directories while holding the process lock.
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        if entry.file_name().to_string_lossy().starts_with(".setup-") && entry.file_type()?.is_dir()
        {
            fs::remove_dir_all(entry.path())?;
        }
    }
    report(progress, "Finding your library…", 0, None);
    let verified = discover(transport, trust, cancel).await?;
    let manifest = &verified.manifest;
    let p = artifact(manifest, profile)?;
    check_high_water(data_dir, &verified)?;
    if let Some(expected) = expected_manifest {
        ensure!(
            hash(&verified.bytes) == expected,
            "A newer release is available. Check for updates again before downloading."
        );
    }
    let water_path = root.join("high-water.json");
    atomic_write(
        &water_path,
        &serde_json::to_vec(&HighWater {
            format_version: 1,
            repository: REPOSITORY.into(),
            channel: Channel::Stable,
            schema: 1,
            protocol: 1,
            sequence: manifest.sequence,
            manifest_sha256: hash(&verified.bytes),
        })?,
    )?;
    if let Some(approval) = approval {
        progress(Progress {
            message: "Ready to download".into(),
            completed: 0,
            total: None,
            offer: Some(Offer {
                download_bytes: p.compressed_bytes,
                installed_bytes: p.sqlite_bytes,
                temporary_bytes: p.compressed_bytes + p.sqlite_bytes + 32 * 1024 * 1024,
            }),
        });
        while !approval.load(Ordering::Relaxed) {
            check_cancel(cancel)?;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    let stage = tempfile::Builder::new()
        .prefix(".setup-")
        .tempdir_in(&root)?;
    let compressed = stage.path().join("download.part");
    let mut body = transport
        .get(
            &manifest
                .asset_url(profile.id())
                .context("Missing library URL.")?,
            cancel,
        )
        .await?;
    let mut file = File::create(&compressed)?;
    let mut count = 0_u64;
    report(
        progress,
        "Downloading your library…",
        0,
        Some(p.compressed_bytes),
    );
    while let Some(chunk) = body.chunk(cancel).await? {
        count += chunk.len() as u64;
        ensure!(
            count <= p.compressed_bytes,
            "The library download exceeded its signed size."
        );
        file.write_all(&chunk)
            .context("Could not save the download. Check available storage and retry.")?;
        report(
            progress,
            "Downloading your library…",
            count,
            Some(p.compressed_bytes),
        );
    }
    file.sync_all()?;
    drop(file);
    verify_file(
        &compressed,
        p.compressed_bytes,
        &p.compressed_sha256,
        cancel,
    )?;
    report(
        progress,
        "Preparing your offline library…",
        0,
        Some(p.sqlite_bytes),
    );
    let candidate = stage.path().join("generation");
    fs::create_dir(&candidate)?;
    let sqlite = candidate.join("library.sqlite");
    let mut output = File::create(&sqlite)?;
    let mut decoder = zstd::stream::read::Decoder::new(File::open(&compressed)?)?;
    decoder.window_log_max(27)?;
    let mut buffer = [0; 65536];
    count = 0;
    loop {
        check_cancel(cancel)?;
        let n = decoder.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        count += n as u64;
        ensure!(
            count <= p.sqlite_bytes,
            "The library expanded beyond its signed size."
        );
        output
            .write_all(&buffer[..n])
            .context("Could not prepare your library. Check available storage and retry.")?;
        report(
            progress,
            "Preparing your offline library…",
            count,
            Some(p.sqlite_bytes),
        );
    }
    output.sync_all()?;
    drop(output);
    verify_file(&sqlite, p.sqlite_bytes, &p.sqlite_sha256, cancel)?;
    report(progress, "Checking your library…", 0, None);
    validate_database(&sqlite, manifest, p)?;
    check_cancel(cancel)?;
    let generation = format!("{}-{}", p.sqlite_sha256, uuid::Uuid::new_v4().simple());
    let destination = root.join("generations").join(&generation);
    let receipt = Receipt {
        format_version: 1,
        manifest: String::from_utf8(verified.bytes)?,
        signature_hex: hex::encode(verified.signature),
        profile,
        generation,
    };
    if let Ok((old_receipt, old_manifest, old_path)) = parse_receipt_with(data_dir, trust) {
        let previous = artifact(&old_manifest, old_receipt.profile)?;
        if verify_file(
            &old_path,
            previous.sqlite_bytes,
            &previous.sqlite_sha256,
            cancel,
        )
        .is_ok()
        {
            atomic_write(
                &root.join("previous.json"),
                &read_limited(&receipt_path(data_dir), 150_000)?,
            )?;
        }
    }
    // This short activation section is the commit point. Cancellation before it
    // leaves the previous pointer intact and the staging directory is discarded.
    check_cancel(cancel)?;
    sync_dir(&candidate)?;
    fs::rename(&candidate, &destination)?;
    sync_dir(&root.join("generations"))?;
    atomic_write(&receipt_path(data_dir), &serde_json::to_vec(&receipt)?)?;
    Ok(destination.join("library.sqlite"))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[test]
fn default_install_is_the_full_free_resource_bundle() {
    assert_eq!(
        Profile::default().id(),
        fontes_data_contracts::DEFAULT_RESOURCE_PROFILE
    );
}
