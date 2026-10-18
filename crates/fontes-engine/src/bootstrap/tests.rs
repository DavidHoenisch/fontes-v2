use super::*;
use ed25519_dalek::{Signer, SigningKey};
use std::{
    collections::BTreeMap,
    io::Cursor,
    sync::{Mutex, OnceLock},
};

struct Memory(Mutex<BTreeMap<String, Vec<u8>>>, Mutex<Vec<String>>);
impl Transport for Memory {
    async fn get(&self, url: &str, cancel: &AtomicBool) -> Result<Body> {
        check_cancel(cancel)?;
        self.1.lock().unwrap().push(url.into());
        Ok(Body::Memory(Cursor::new(
            self.0
                .lock()
                .unwrap()
                .get(url)
                .with_context(|| format!("Missing test response: {url}"))?
                .clone(),
        )))
    }
}
struct Fixture {
    transport: Memory,
    manifest: ReleaseManifest,
    trust: Trust,
    key: SigningKey,
}
fn sample() -> &'static (Vec<u8>, ReleaseManifest) {
    static SAMPLE: OnceLock<(Vec<u8>, ReleaseManifest)> = OnceLock::new();
    SAMPLE.get_or_init(|| {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.sqlite");
        let mut db = rusqlite::Connection::open(&path).unwrap();
        db.execute_batch(fontes_data_contracts::CONTENT_SCHEMA_SQL).unwrap();
        let mut manifest: ReleaseManifest = serde_json::from_str(include_str!("../../../../data/examples/release-manifest.json")).unwrap();
        manifest.sequence = 4;
        manifest.release_tag = "data-v1-stable-4".into();
        let tx = db.transaction().unwrap();
        tx.execute("INSERT INTO build_info VALUES(1,?1,'reading',1,1,?2,?3)", rusqlite::params![manifest.profiles[0].content_id, manifest.source_lock_sha256, manifest.builder_commit]).unwrap();
        let registry: serde_json::Value = serde_json::from_str(fontes_data_contracts::CANONICAL_REGISTRY_JSON).unwrap();
        for book in registry["books"].as_array().unwrap() {
            let id = book["id"].as_i64().unwrap();
            tx.execute("INSERT INTO books VALUES(?1,?2,?3,?4,?1)", rusqlite::params![id, book["osis"].as_str(), book["name"].as_str(), book["testament"].as_str()]).unwrap();
            for (chapter, count) in book["verse_counts"].as_array().unwrap().iter().enumerate() {
                for verse in 1..=count.as_i64().unwrap() {
                    tx.prepare_cached("INSERT INTO canonical_refs VALUES(?1,?2,?3,?4)").unwrap().execute(rusqlite::params![id*1_000_000+(chapter as i64+1)*1000+verse,id,chapter+1,verse]).unwrap();
                }
            }
        }
        tx.execute_batch("INSERT INTO source_snapshots VALUES('source','fixture','hash','https://example.org',NULL,'review','adapter');
            INSERT INTO text_witnesses VALUES('kjv','KJV','en','fixture','KJV','source'),('web','WEB','en','fixture','KJV','source');
            INSERT INTO translations VALUES('kjv','KJV','King James Version','en','kjv'),('web','WEB','World English Bible','en','web');
            INSERT INTO source_segments VALUES('kjv16','source','kjv','John.3.16','kjv16'),('web16','source','web','John.3.16','web16');
            INSERT INTO segment_refs VALUES('kjv16',43003016,'exact'),('web16',43003016,'exact');
            INSERT INTO verse_texts VALUES('kjv16','kjv','kjv16','For God so loved the world.',NULL),('web16','web','web16','For God so loved the world, WEB.',NULL);").unwrap();
        tx.commit().unwrap();
        drop(db);
        let sqlite = fs::read(path).unwrap();
        let compressed = zstd::stream::encode_all(sqlite.as_slice(), 1).unwrap();
        let p = &mut manifest.profiles[0];
        p.sqlite_bytes = sqlite.len() as u64;
        p.sqlite_sha256 = hash(&sqlite);
        p.compressed_bytes = compressed.len() as u64;
        p.compressed_sha256 = hash(&compressed);
        p.verse_text_count = 2;
        (compressed, manifest)
    })
}
impl Fixture {
    fn new() -> Self {
        let key = SigningKey::from_bytes(&[42; 32]);
        let mut fixture = Self {
            transport: Memory(Mutex::new(BTreeMap::new()), Mutex::new(Vec::new())),
            manifest: sample().1.clone(),
            trust: Trust {
                key_id: "data-release-1".into(),
                public_key_hex: hex::encode(key.verifying_key().to_bytes()),
            },
            key,
        };
        fixture.publish();
        fixture
    }
    fn publish(&mut self) {
        let tag = format!("data-v1-stable-{}", self.manifest.sequence);
        self.manifest.release_tag = tag.clone();
        let bytes = serde_json::to_vec(&self.manifest).unwrap();
        let signature = self.key.sign(&[DOMAIN, &bytes].concat());
        let base = format!("https://github.com/{REPOSITORY}/releases/download/{tag}");
        let mut map = self.transport.0.lock().unwrap();
        map.insert(
            format!("https://api.github.com/repos/{REPOSITORY}/releases?per_page=100&page=1"),
            serde_json::to_vec(&serde_json::json!([
                {"tag_name":"app-v0.1.0-dev.99.1","draft":false,"prerelease":true},
                {"tag_name":tag,"draft":false,"prerelease":false}
            ]))
            .unwrap(),
        );
        map.insert(format!("{base}/release-manifest.json"), bytes);
        map.insert(
            format!("{base}/release-manifest.json.sig"),
            signature.to_bytes().to_vec(),
        );
        map.insert(
            self.manifest.asset_url("reading").unwrap(),
            sample().0.clone(),
        );
    }
    fn install(
        &self,
        dir: &Path,
        cancel: &AtomicBool,
        progress: impl Fn(Progress),
    ) -> Result<PathBuf> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(install_with(
                dir,
                Profile::Reading,
                cancel,
                None,
                &progress,
                &self.transport,
                &self.trust,
            ))
    }
}
#[test]
fn signed_install_is_atomic_keeps_notes_and_restarts_offline() {
    let fixture = Fixture::new();
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("old.sqlite");
    fs::write(
        &old,
        zstd::stream::decode_all(sample().0.as_slice()).unwrap(),
    )
    .unwrap();
    let user = dir.path().join("user.sqlite");
    let mut engine = Engine::open(&old, &user).unwrap();
    engine
        .dispatch(crate::Command::SaveAnnotation {
            id: None,
            start_ref: 43003016,
            end_ref: 43003016,
            kind: crate::AnnotationKind::Note,
            color: None,
            content: Some("Keep my note".into()),
        })
        .unwrap();
    drop(engine);
    let original_notes = fs::read(&user).unwrap();
    let cancel = AtomicBool::new(false);
    let path = fixture.install(dir.path(), &cancel, |_| {}).unwrap();
    assert_eq!(fs::read(&user).unwrap(), original_notes);
    assert_eq!(
        Engine::open(&path, &user)
            .unwrap()
            .workspace()
            .unwrap()
            .annotations[0]
            .content
            .as_deref(),
        Some("Keep my note")
    );
    let receipt: Receipt =
        serde_json::from_slice(&fs::read(receipt_path(dir.path())).unwrap()).unwrap();
    assert!(
        verify(
            receipt.manifest.as_bytes(),
            &hex::decode(receipt.signature_hex).unwrap(),
            &fixture.trust
        )
        .is_ok()
    );
    assert!(
        installed_library(dir.path()).is_err(),
        "A receipt cannot provide its own trust root"
    );
    assert!(fs::read_dir(root(dir.path())).unwrap().all(|e| {
        !e.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".setup-")
    }));
}
#[test]
fn failed_download_and_cancel_preserve_active_generation() {
    let fixture = Fixture::new();
    let dir = tempfile::tempdir().unwrap();
    let cancel = AtomicBool::new(false);
    let path = fixture.install(dir.path(), &cancel, |_| {}).unwrap();
    let active = fs::read(receipt_path(dir.path())).unwrap();
    fixture.transport.0.lock().unwrap().insert(
        fixture.manifest.asset_url("reading").unwrap(),
        vec![1, 2, 3],
    );
    assert!(fixture.install(dir.path(), &cancel, |_| {}).is_err());
    assert_eq!(fs::read(receipt_path(dir.path())).unwrap(), active);
    verify_file(
        &path,
        fixture.manifest.profiles[0].sqlite_bytes,
        &fixture.manifest.profiles[0].sqlite_sha256,
        &cancel,
    )
    .unwrap();
    cancel.store(true, Ordering::Relaxed);
    assert!(fixture.install(dir.path(), &cancel, |_| {}).is_err());
    assert_eq!(fs::read(receipt_path(dir.path())).unwrap(), active);
}
#[test]
fn signatures_repository_and_limits_are_enforced() {
    let mut f = Fixture::new();
    let bytes = serde_json::to_vec(&f.manifest).unwrap();
    let sig = f.key.sign(&[DOMAIN, &bytes].concat()).to_bytes();
    assert!(verify(&bytes, &sig, &f.trust).is_ok());
    let mut changed = bytes.clone();
    changed.push(b' ');
    assert!(verify(&changed, &sig, &f.trust).is_err());
    for mutation in 0..4 {
        f.manifest = sample().1.clone();
        match mutation {
            0 => f.manifest.repository = "someone/else".into(),
            1 => f.manifest.profiles[0].sqlite_bytes = MAX_SQLITE + 1,
            2 => f.manifest.profiles[0].source_ids.clear(),
            _ => f.manifest.update_protocol_version = 99,
        }
        let bytes = serde_json::to_vec(&f.manifest).unwrap();
        let sig = f.key.sign(&[DOMAIN, &bytes].concat()).to_bytes();
        assert!(verify(&bytes, &sig, &f.trust).is_err());
    }
}
#[test]
fn stale_sequence_and_changed_same_sequence_are_refused() {
    let mut f = Fixture::new();
    let dir = tempfile::tempdir().unwrap();
    let cancel = AtomicBool::new(false);
    f.manifest.sequence = 5;
    f.publish();
    f.install(dir.path(), &cancel, |_| {}).unwrap();
    let active = fs::read(receipt_path(dir.path())).unwrap();
    f.manifest.sequence = 4;
    f.publish();
    assert!(
        f.install(dir.path(), &cancel, |_| {})
            .unwrap_err()
            .to_string()
            .contains("older")
    );
    f.manifest.sequence = 5;
    f.manifest.published_at = "changed".into();
    f.publish();
    assert!(
        f.install(dir.path(), &cancel, |_| {})
            .unwrap_err()
            .to_string()
            .contains("changed")
    );
    assert_eq!(fs::read(receipt_path(dir.path())).unwrap(), active);
}
#[test]
fn corrupt_expanded_data_never_activates() {
    for mode in 0..3 {
        let mut f = Fixture::new();
        let dir = tempfile::tempdir().unwrap();
        match mode {
            0 => f.manifest.profiles[0].sqlite_bytes = 4096,
            1 => f.manifest.profiles[0].sqlite_sha256 = "a".repeat(64),
            _ => f.manifest.profiles[0].resource_count = 99,
        }
        f.publish();
        assert!(
            f.install(dir.path(), &AtomicBool::new(false), |_| {})
                .is_err()
        );
        assert!(!receipt_path(dir.path()).exists());
    }
}
#[test]
fn cancellation_during_download_and_process_lock_are_recoverable() {
    let f = Fixture::new();
    let dir = tempfile::tempdir().unwrap();
    let cancel = AtomicBool::new(false);
    assert!(
        f.install(dir.path(), &cancel, |p| {
            if p.completed > 0 {
                cancel.store(true, Ordering::Relaxed);
            }
        })
        .is_err()
    );
    assert!(!receipt_path(dir.path()).exists());
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .open(root(dir.path()).join("setup.lock"))
        .unwrap();
    try_lock_setup(&lock).unwrap();
    cancel.store(false, Ordering::Relaxed);
    assert!(
        f.install(dir.path(), &cancel, |_| {})
            .unwrap_err()
            .to_string()
            .contains("already running")
    );
    drop(lock);
    assert!(f.install(dir.path(), &cancel, |_| {}).is_ok());
}
#[test]
fn lock_io_errors_are_not_reported_as_another_running_setup() {
    use std::io::{Error, ErrorKind};
    for kind in [
        ErrorKind::Unsupported,
        ErrorKind::PermissionDenied,
        ErrorKind::Other,
    ] {
        let error = setup_lock_error(Error::new(kind, "platform lock failure"));
        assert!(error.to_string().contains("Could not lock library storage"));
        assert!(!error.to_string().contains("already running"));
        assert!(format!("{error:#}").contains("platform lock failure"));
    }
    assert!(
        setup_lock_error(ErrorKind::WouldBlock.into())
            .to_string()
            .contains("already running")
    );
}
#[test]
fn setup_lock_survives_existing_files_and_releases_on_close() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("setup.lock");
    // A persistent lock file is harmless after an earlier process has exited.
    fs::write(&path, b"").unwrap();
    let open = || {
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap()
    };
    let first = open();
    try_lock_setup(&first).unwrap();
    let second = open();
    assert_eq!(
        try_lock_setup(&second).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    drop(first);
    try_lock_setup(&second).unwrap();
    drop(second);
    try_lock_setup(&open()).unwrap();
}
#[test]
fn actual_database_identity_is_checked_after_hashes() {
    let mut f = Fixture::new();
    f.manifest.profiles[0].content_id = "b".repeat(64);
    f.publish();
    let dir = tempfile::tempdir().unwrap();
    assert!(
        f.install(dir.path(), &AtomicBool::new(false), |_| {})
            .unwrap_err()
            .to_string()
            .contains("signed release")
    );
    assert!(!receipt_path(dir.path()).exists());
}

#[test]
fn gui_confirmation_precedes_payload_transfer() {
    let f = Fixture::new();
    let dir = tempfile::tempdir().unwrap();
    let cancel = AtomicBool::new(false);
    let approval = AtomicBool::new(false);
    let asset = f.manifest.asset_url("reading").unwrap();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(install_with(
            dir.path(),
            Profile::Reading,
            &cancel,
            Some(&approval),
            &|p| {
                if let Some(offer) = p.offer {
                    assert_eq!(
                        offer.download_bytes,
                        f.manifest.profiles[0].compressed_bytes
                    );
                    assert!(!f.transport.1.lock().unwrap().contains(&asset));
                    approval.store(true, Ordering::Relaxed);
                }
            },
            &f.transport,
            &f.trust,
        ))
        .unwrap();
    assert!(f.transport.1.lock().unwrap().contains(&asset));
}
#[test]
fn waiting_network_future_is_cancellable() {
    let flag = Arc::new(AtomicBool::new(false));
    let other = flag.clone();
    let start = std::time::Instant::now();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                other.store(true, Ordering::Relaxed);
            });
            assert!(
                cancellable(&flag, std::future::pending::<()>())
                    .await
                    .is_err()
            );
        });
    assert!(start.elapsed() < Duration::from_secs(2));
}
#[test]
fn unsigned_higher_tag_does_not_override_verified_release() {
    let f = Fixture::new();
    let list = format!("https://api.github.com/repos/{REPOSITORY}/releases?per_page=100&page=1");
    let mut map = f.transport.0.lock().unwrap();
    map.insert(
        list,
        serde_json::to_vec(&serde_json::json!([
            {"tag_name":"data-v1-stable-99","draft":false,"prerelease":false},
            {"tag_name":"data-v1-stable-4","draft":false,"prerelease":false}
        ]))
        .unwrap(),
    );
    let base = format!("https://github.com/{REPOSITORY}/releases/download/data-v1-stable-99");
    map.insert(
        format!("{base}/release-manifest.json"),
        serde_json::to_vec(&f.manifest).unwrap(),
    );
    map.insert(format!("{base}/release-manifest.json.sig"), vec![0; 64]);
    drop(map);
    let dir = tempfile::tempdir().unwrap();
    assert!(
        f.install(dir.path(), &AtomicBool::new(false), |_| {})
            .is_ok()
    );
}

fn check_fixture(f: &Fixture, dir: &Path) -> Result<UpdateInfo> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(check_with(
            dir,
            Profile::Reading,
            &AtomicBool::new(false),
            &f.transport,
            &f.trust,
        ))
}
fn advance_fixture(f: &mut Fixture) {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("new.sqlite");
    fs::write(
        &path,
        zstd::stream::decode_all(sample().0.as_slice()).unwrap(),
    )
    .unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute(
        "UPDATE verse_texts SET text='Updated source text.' WHERE id='kjv16'",
        [],
    )
    .unwrap();
    drop(db);
    let sqlite = fs::read(path).unwrap();
    let compressed = zstd::stream::encode_all(sqlite.as_slice(), 1).unwrap();
    f.manifest.sequence += 1;
    let p = &mut f.manifest.profiles[0];
    p.sqlite_bytes = sqlite.len() as u64;
    p.sqlite_sha256 = hash(&sqlite);
    p.compressed_bytes = compressed.len() as u64;
    p.compressed_sha256 = hash(&compressed);
    f.publish();
    f.transport
        .0
        .lock()
        .unwrap()
        .insert(f.manifest.asset_url("reading").unwrap(), compressed);
}
#[test]
fn update_check_is_metadata_only_and_pinned_update_preserves_user_data() {
    let mut f = Fixture::new();
    let dir = tempfile::tempdir().unwrap();
    let cancel = AtomicBool::new(false);
    let old = f.install(dir.path(), &cancel, |_| {}).unwrap();
    let user = dir.path().join("user.sqlite");
    let mut engine = Engine::open(&old, &user).unwrap();
    engine
        .dispatch(crate::Command::SaveAnnotation {
            id: None,
            start_ref: 43003016,
            end_ref: 43003016,
            kind: crate::AnnotationKind::Note,
            color: None,
            content: Some("My private note".into()),
        })
        .unwrap();
    engine
        .dispatch(crate::Command::SaveAnnotation {
            id: None,
            start_ref: 43003016,
            end_ref: 43003016,
            kind: crate::AnnotationKind::Highlight,
            color: Some("amber".into()),
            content: None,
        })
        .unwrap();
    drop(engine);
    let notes = fs::read(&user).unwrap();
    let active = fs::read(receipt_path(dir.path())).unwrap();
    assert!(!check_fixture(&f, dir.path()).unwrap().update_available);
    advance_fixture(&mut f);
    f.transport.1.lock().unwrap().clear();
    let offer = check_fixture(&f, dir.path()).unwrap();
    assert!(offer.update_available);
    assert_eq!(offer.installed_sequence, Some(4));
    assert_eq!(offer.available_sequence, 5);
    assert!(
        !f.transport
            .1
            .lock()
            .unwrap()
            .contains(&f.manifest.asset_url("reading").unwrap())
    );
    assert_eq!(fs::read(receipt_path(dir.path())).unwrap(), active);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    // A stale reviewed offer is refused before any artifact transfer.
    assert!(
        runtime
            .block_on(install_with_expected(
                dir.path(),
                Profile::Reading,
                &cancel,
                None,
                &|_| {},
                &f.transport,
                &f.trust,
                Some(&"0".repeat(64))
            ))
            .is_err()
    );
    assert_eq!(fs::read(receipt_path(dir.path())).unwrap(), active);
    let new = runtime
        .block_on(install_with_expected(
            dir.path(),
            Profile::Reading,
            &cancel,
            None,
            &|_| {},
            &f.transport,
            &f.trust,
            Some(&offer.manifest_sha256),
        ))
        .unwrap();
    assert_ne!(old, new);
    assert!(old.is_file());
    assert_eq!(
        fs::read(root(dir.path()).join("previous.json")).unwrap(),
        active
    );
    assert_eq!(fs::read(&user).unwrap(), notes);
    assert_eq!(
        Engine::open(&new, &user)
            .unwrap()
            .workspace()
            .unwrap()
            .annotations
            .len(),
        2
    );
    assert!(!check_fixture(&f, dir.path()).unwrap().update_available);
    f.manifest.sequence = 4;
    f.publish();
    assert!(check_fixture(&f, dir.path()).is_err());
}
