use super::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

fn copy(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if entry.path().is_dir() {
            copy(&entry.path(), &destination.join(entry.file_name()));
        } else {
            fs::copy(entry.path(), destination.join(entry.file_name())).unwrap();
        }
    }
}
fn setup() -> (tempfile::TempDir, SigningKey) {
    let temp = tempfile::tempdir().unwrap();
    let contracts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../specs/resource-packages/v1");
    let library = temp.path().join("library");
    copy(
        &contracts.join("examples/romans-study"),
        &library.join("submissions/org.example.romans-study/source"),
    );
    fs::create_dir_all(library.join("trust")).unwrap();
    fs::create_dir_all(library.join("approvals")).unwrap();
    fs::copy(
        contracts.join("taxonomy.example.xml"),
        library.join("taxonomy.xml"),
    )
    .unwrap();
    write_json(
        &library.join("policy.json"),
        &Policy {
            policy_version: 1,
            distribution: Distribution::PublicFree,
            publishers: ["org.example".into()].into_iter().collect(),
            source_hosts: ["github.com".into()].into_iter().collect(),
            taxonomy: "taxonomy.xml".into(),
            trust: "trust/source-release-1.json".into(),
        },
    )
    .unwrap();
    write_json(
        &library.join("submissions/org.example.romans-study/submission.json"),
        &Submission {
            submission_version: 1,
            resource_id: "org.example.romans-study".into(),
            publisher_id: "org.example".into(),
            version: "1.0.0".into(),
            build_revision: 1,
            source: Source::Path {
                path: "submissions/org.example.romans-study/source".into(),
            },
            release_notes: "Isolated original CC0 test example".into(),
        },
    )
    .unwrap();
    let key = SigningKey::from_bytes(&[19; 32]);
    write_json(
        &temp.path().join("library/trust/source-release-1.json"),
        &Trust {
            key_id: "test-source-key".into(),
            public_key_hex: hex::encode(key.verifying_key().to_bytes()),
        },
    )
    .unwrap();
    let archive = pack(
        &library.join("submissions/org.example.romans-study/source"),
        &temp
            .path()
            .join("org.example.romans-study--1.0.0.fontes-source.zip"),
        &ValidationOptions {
            taxonomy: Some(library.join("taxonomy.xml")),
            deny_warnings: true,
        },
    )
    .unwrap();
    write_json(
        &library.join("approvals/org.example.romans-study--1.0.0.json"),
        &Approval {
            approval_version: 1,
            resource_id: "org.example.romans-study".into(),
            version: "1.0.0".into(),
            source_sha256: archive.sha256,
            distribution: Distribution::PublicFree,
            rights_review_id: "test-cc0-rights".into(),
            publisher_review_id: "test-publisher".into(),
        },
    )
    .unwrap();
    (temp, key)
}
fn build_at(temp: &tempfile::TempDir, name: &str, approved_only: bool) -> BatchReport {
    build(&BuildOptions {
        root: &temp.path().join("library"),
        policy_root: &temp.path().join("library"),
        output: &temp.path().join(name),
        repository: "test/resources",
        source_repository: "test/resources",
        source_commit: &"a".repeat(40),
        tooling_commit: &"b".repeat(40),
        approved_only,
    })
    .unwrap()
}

#[derive(Default)]
struct State {
    releases: BTreeMap<String, bool>,
    creates: usize,
    downloads: usize,
    private: bool,
    tamper: bool,
    fail_publish: bool,
}
fn host(
    state: Rc<RefCell<State>>,
    storage: Rc<tempfile::TempDir>,
) -> impl Fn(&[&str]) -> Result<String> {
    move |args| {
        let value = |key: &str| {
            args.iter()
                .position(|arg| *arg == key)
                .and_then(|i| args.get(i + 1))
                .copied()
                .context("missing mock argument")
        };
        match args {
            ["repo", "view", _, "--json", "isPrivate"] => {
                Ok(serde_json::json!({"isPrivate":state.borrow().private}).to_string())
            }
            ["api", _, "--jq", ".sha"] => Ok("c".repeat(40)),
            ["release", "list", ..] => Ok(serde_json::to_string(
                &state
                    .borrow()
                    .releases
                    .iter()
                    .map(|(tag, draft)| serde_json::json!({"tagName":tag,"isDraft":draft}))
                    .collect::<Vec<_>>(),
            )?),
            ["release", "create", tag, ..] => {
                let mut state = state.borrow_mut();
                ensure!(!state.releases.contains_key(*tag), "duplicate create");
                state.creates += 1;
                state.releases.insert(tag.to_string(), true);
                let destination = storage.path().join(tag);
                fs::create_dir(&destination)?;
                let start = args.iter().position(|arg| *arg == "--notes-file").unwrap() + 2;
                for asset in &args[start..] {
                    let path = Path::new(asset);
                    fs::copy(path, destination.join(path.file_name().unwrap()))?;
                }
                let manifest: SourceManifest = json(&destination.join(MANIFEST))?;
                ensure!(
                    value("--target")? == manifest.release_commit,
                    "wrong release tag target"
                );
                Ok(String::new())
            }
            ["release", "download", tag, ..] => {
                let mut state = state.borrow_mut();
                state.downloads += 1;
                let destination = Path::new(value("--dir")?);
                let patterns: Vec<_> = args
                    .windows(2)
                    .filter(|p| p[0] == "--pattern")
                    .map(|p| p[1])
                    .collect();
                for entry in fs::read_dir(storage.path().join(tag))? {
                    let entry = entry?;
                    let name = entry.file_name();
                    if patterns.contains(&"*") || patterns.contains(&name.to_str().unwrap()) {
                        fs::copy(entry.path(), destination.join(name))?;
                    }
                }
                if state.tamper && destination.join("metadata.xml").exists() {
                    fs::write(destination.join("metadata.xml"), "tampered")?;
                }
                Ok(String::new())
            }
            ["release", "edit", tag, ..] => {
                let mut state = state.borrow_mut();
                ensure!(
                    !state.fail_publish,
                    "simulated interruption before publishing"
                );
                *state.releases.get_mut(*tag).context("unknown draft")? = false;
                Ok(String::new())
            }
            _ => bail!("unexpected mock GitHub arguments: {args:?}"),
        }
    }
}

#[test]
fn source_builds_are_reproducible_and_approved_hash_is_pinned() {
    let (temp, _) = setup();
    let a = build_at(&temp, "first", true);
    let b = build_at(&temp, "second", true);
    assert!(a.valid && b.valid);
    assert!(a.submissions[0].approved);
    let first = temp.path().join("first/org.example.romans-study");
    let second = temp.path().join("second/org.example.romans-study");
    for entry in fs::read_dir(&first).unwrap() {
        let entry = entry.unwrap();
        assert_eq!(
            fs::read(entry.path()).unwrap(),
            fs::read(second.join(entry.file_name())).unwrap()
        );
    }
    let path = temp
        .path()
        .join("library/submissions/org.example.romans-study/source/content/rom/001.xml");
    fs::write(
        &path,
        fs::read_to_string(&path)
            .unwrap()
            .replace("Identity and ancestry", "Changed heading"),
    )
    .unwrap();
    let invalid = build_at(&temp, "changed", true);
    assert!(!invalid.valid);
    assert!(invalid.submissions[0].message.contains("exact source hash"));
}

#[test]
fn proposed_approval_in_untrusted_content_does_not_authorize_release() {
    let (temp, _) = setup();
    let trusted = temp.path().join("trusted");
    copy(&temp.path().join("library"), &trusted);
    fs::remove_file(trusted.join("approvals/org.example.romans-study--1.0.0.json")).unwrap();
    let options = |output| BuildOptions {
        root: temp.path(),
        policy_root: temp.path(),
        output,
        repository: "test/resources",
        source_repository: "test/resources",
        source_commit: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        tooling_commit: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        approved_only: true,
    };
    let output = temp.path().join("build");
    let mut config = options(&output);
    let library = temp.path().join("library");
    config.root = &library;
    config.policy_root = &trusted;
    let result = build(&config).unwrap();
    assert!(result.valid);
    assert!(!result.submissions[0].approved);
    assert!(!output.join("org.example.romans-study").exists());
    config.approved_only = false;
    let output = temp.path().join("preview");
    config.output = &output;
    let preview = build(&config).unwrap();
    assert!(preview.valid && !preview.submissions[0].approved);
    assert!(
        output
            .join("org.example.romans-study/cover-card.png")
            .exists()
    );
}

#[test]
fn publishes_verifies_and_retries_without_overwriting() {
    let (temp, key) = setup();
    assert!(build_at(&temp, "build", true).valid);
    let state = Rc::new(RefCell::new(State::default()));
    let storage = Rc::new(tempfile::tempdir().unwrap());
    let api = host(state.clone(), storage.clone());
    let result = publish_with(
        &temp.path().join("build"),
        &temp.path().join("library"),
        "test/resources",
        &key,
        &api,
    )
    .unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(state.borrow().creates, 1);
    assert_eq!(state.borrow().downloads, 2);
    assert!(state.borrow().releases.values().all(|draft| !*draft));
    let tag = tag("org.example.romans-study", "1.0.0", 1);
    let manifest = verify(
        &storage.path().join(&tag),
        &temp.path().join("library/trust/source-release-1.json"),
        "test/resources",
    )
    .unwrap();
    assert_eq!(manifest.artifact_kind, "validated_source_package");
    assert!(
        publish_with(
            &temp.path().join("build"),
            &temp.path().join("library"),
            "test/resources",
            &key,
            &api
        )
        .is_ok()
    );
    assert_eq!(state.borrow().creates, 1);
}

#[test]
fn detects_uploaded_corruption_before_publishing_and_rejects_signature_tampering() {
    let (temp, key) = setup();
    assert!(build_at(&temp, "build", true).valid);
    let state = Rc::new(RefCell::new(State {
        tamper: true,
        ..Default::default()
    }));
    let storage = Rc::new(tempfile::tempdir().unwrap());
    let api = host(state.clone(), storage.clone());
    assert!(
        publish_with(
            &temp.path().join("build"),
            &temp.path().join("library"),
            "test/resources",
            &key,
            &api
        )
        .is_err()
    );
    assert!(state.borrow().releases.values().all(|draft| *draft));
    let release = storage
        .path()
        .join(tag("org.example.romans-study", "1.0.0", 1));
    fs::write(release.join(SIGNATURE), [0; 64]).unwrap();
    assert!(
        verify(
            &release,
            &temp.path().join("library/trust/source-release-1.json"),
            "test/resources"
        )
        .is_err()
    );
}

#[test]
fn resumes_a_complete_verified_draft_after_interruption() {
    let (temp, key) = setup();
    assert!(build_at(&temp, "build", true).valid);
    let state = Rc::new(RefCell::new(State {
        fail_publish: true,
        ..Default::default()
    }));
    let storage = Rc::new(tempfile::tempdir().unwrap());
    let api = host(state.clone(), storage);
    assert!(
        publish_with(
            &temp.path().join("build"),
            &temp.path().join("library"),
            "test/resources",
            &key,
            &api
        )
        .is_err()
    );
    state.borrow_mut().fail_publish = false;
    assert!(
        publish_with(
            &temp.path().join("build"),
            &temp.path().join("library"),
            "test/resources",
            &key,
            &api
        )
        .is_ok()
    );
    assert_eq!(state.borrow().creates, 1);
    assert!(state.borrow().releases.values().all(|draft| !*draft));
}

#[test]
fn refuses_private_content_in_public_storage_and_supports_separate_private_storage() {
    let (temp, key) = setup();
    for path in [
        temp.path().join("library/policy.json"),
        temp.path()
            .join("library/approvals/org.example.romans-study--1.0.0.json"),
    ] {
        let text = fs::read_to_string(&path)
            .unwrap()
            .replace("public_free", "private_restricted");
        fs::write(path, text).unwrap();
    }
    assert!(build_at(&temp, "build", true).valid);
    let state = Rc::new(RefCell::new(State::default()));
    let storage = Rc::new(tempfile::tempdir().unwrap());
    let api = host(state.clone(), storage.clone());
    assert!(
        publish_with(
            &temp.path().join("build"),
            &temp.path().join("library"),
            "test/resources",
            &key,
            &api
        )
        .unwrap_err()
        .to_string()
        .contains("public repository")
    );
    assert_eq!(state.borrow().creates, 0);
    state.borrow_mut().private = true;
    // Separate storage repository: the source commit remains original, the tag
    // points to storage's own resolved HEAD, and both identities are signed.
    let path = temp
        .path()
        .join("build/org.example.romans-study")
        .join(MANIFEST);
    let mut manifest: SourceManifest = json(&path).unwrap();
    manifest.repository = "test/private-storage".into();
    write_json(&path, &manifest).unwrap();
    assert!(
        publish_with(
            &temp.path().join("build"),
            &temp.path().join("library"),
            "test/private-storage",
            &key,
            &api
        )
        .is_ok()
    );
    let release = storage
        .path()
        .join(tag("org.example.romans-study", "1.0.0", 1));
    let manifest: SourceManifest = json(&release.join(MANIFEST)).unwrap();
    assert_eq!(manifest.release_commit, "c".repeat(40));
    assert_eq!(manifest.source_commit, "a".repeat(40));
}

#[test]
fn rejects_identity_protocol_policy_and_inventory_tampering() {
    let (temp, key) = setup();
    assert!(build_at(&temp, "build", true).valid);
    let state = Rc::new(RefCell::new(State::default()));
    let storage = Rc::new(tempfile::tempdir().unwrap());
    let api = host(state, storage.clone());
    publish_with(
        &temp.path().join("build"),
        &temp.path().join("library"),
        "test/resources",
        &key,
        &api,
    )
    .unwrap();
    let release = storage
        .path()
        .join(tag("org.example.romans-study", "1.0.0", 1));
    let trust = temp.path().join("library/trust/source-release-1.json");
    assert!(verify(&release, &trust, "other/repository").is_err());
    fs::write(release.join("extra.sql"), "unexpected").unwrap();
    assert!(verify(&release, &trust, "test/resources").is_err());
    fs::remove_file(release.join("extra.sql")).unwrap();
    let original = fs::read(release.join(MANIFEST)).unwrap();
    for field in [
        "source_release_protocol",
        "artifact_kind",
        "approved",
        "source_archive",
    ] {
        let mut value: serde_json::Value = serde_json::from_slice(&original).unwrap();
        value[field] = match field {
            "source_release_protocol" => serde_json::json!(2),
            "approved" => serde_json::json!(false),
            _ => serde_json::json!("unsupported"),
        };
        let bytes = serde_json::to_vec_pretty(&value).unwrap();
        fs::write(release.join(MANIFEST), &bytes).unwrap();
        fs::write(
            release.join(SIGNATURE),
            key.sign(&[DOMAIN, bytes.as_slice()].concat()).to_bytes(),
        )
        .unwrap();
        assert!(verify(&release, &trust, "test/resources").is_err());
    }
}

#[test]
fn rejects_unsafe_remote_hosts_before_network_access() {
    let (temp, _) = setup();
    let policy: Policy = json(&temp.path().join("library/policy.json")).unwrap();
    for url in [
        "http://github.com/resource.zip",
        "https://localhost/resource.zip",
        "https://github.com:444/resource.zip",
        "https://user:password@github.com/file.zip",
    ] {
        let result = acquire(
            &Source::Https {
                url: url.into(),
                sha256: "0".repeat(64),
                bytes: 1,
            },
            &temp.path().join("library"),
            &policy,
            temp.path(),
        );
        assert!(result.unwrap_err().to_string().contains("allowlist"));
    }
}

#[test]
fn refuses_revised_source_under_an_old_version_even_with_new_approval_and_build_revision() {
    let (temp, key) = setup();
    assert!(build_at(&temp, "first", true).valid);
    let state = Rc::new(RefCell::new(State::default()));
    let storage = Rc::new(tempfile::tempdir().unwrap());
    let api = host(state.clone(), storage);
    publish_with(
        &temp.path().join("first"),
        &temp.path().join("library"),
        "test/resources",
        &key,
        &api,
    )
    .unwrap();
    let source = temp
        .path()
        .join("library/submissions/org.example.romans-study/source/content/rom/001.xml");
    fs::write(
        &source,
        fs::read_to_string(&source)
            .unwrap()
            .replace("Identity and ancestry", "Revised source text"),
    )
    .unwrap();
    let descriptor = temp
        .path()
        .join("library/submissions/org.example.romans-study/submission.json");
    let mut submission: Submission = json(&descriptor).unwrap();
    submission.build_revision = 2;
    write_json(&descriptor, &submission).unwrap();
    let approval_path = temp
        .path()
        .join("library/approvals/org.example.romans-study--1.0.0.json");
    let mut approval: Approval = json(&approval_path).unwrap();
    fs::remove_file(&approval_path).unwrap();
    assert!(build_at(&temp, "preview", false).valid);
    let manifest: SourceManifest = json(
        &temp
            .path()
            .join("preview/org.example.romans-study")
            .join(MANIFEST),
    )
    .unwrap();
    approval.source_sha256 = manifest.source_sha256;
    write_json(&approval_path, &approval).unwrap();
    assert!(build_at(&temp, "second", true).valid);
    let error = publish_with(
        &temp.path().join("second"),
        &temp.path().join("library"),
        "test/resources",
        &key,
        &api,
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("increment resource version"),
        "{error:#}"
    );
    assert_eq!(state.borrow().creates, 1);
}

#[test]
fn adding_a_publisher_does_not_force_unchanged_resource_republication() {
    let (temp, key) = setup();
    assert!(build_at(&temp, "first", true).valid);
    let state = Rc::new(RefCell::new(State::default()));
    let storage = Rc::new(tempfile::tempdir().unwrap());
    let api = host(state.clone(), storage);
    publish_with(
        &temp.path().join("first"),
        &temp.path().join("library"),
        "test/resources",
        &key,
        &api,
    )
    .unwrap();
    let policy_path = temp.path().join("library/policy.json");
    let mut policy: Policy = json(&policy_path).unwrap();
    policy.publishers.insert("org.another".into());
    write_json(&policy_path, &policy).unwrap();
    assert!(build_at(&temp, "next", true).valid);
    publish_with(
        &temp.path().join("next"),
        &temp.path().join("library"),
        "test/resources",
        &key,
        &api,
    )
    .unwrap();
    assert_eq!(state.borrow().creates, 1);
}
