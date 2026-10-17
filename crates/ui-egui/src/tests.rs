use super::*;
use egui_kittest::{Harness, kittest::Queryable};
use std::time::{Duration, Instant};
#[test]
fn first_run_offers_path_free_setup_with_reachable_actions() {
    for viewport in [[320.0, 640.0], [390.0, 844.0], [1440.0, 960.0]] {
        let dir = tempfile::tempdir().unwrap();
        let mut h = Harness::builder()
            .with_size(viewport)
            .build_eframe(|cc| FontesApp::new(cc, None, dir.path().to_owned()));
        assert!(h.query_by_label("Open library").is_none());
        assert!(h.get_by_label("Continue").rect().right() <= viewport[0]);
        h.get_by_label("Study").click();
        h.run_ok();
        assert_eq!(h.state().setup.profile, bootstrap::Profile::Study);
        h.state_mut().setup.offer = Some(bootstrap::Offer {
            download_bytes: 110_000_000,
            installed_bytes: 585_000_000,
            temporary_bytes: 730_000_000,
        });
        h.run_ok();
        let download = h.get_by_label("Download and open").rect();
        assert!(download.right() <= viewport[0] && download.bottom() <= viewport[1]);
        assert!(h.query_by_label("Choose another library").is_some());
        h.state_mut().setup.offer = None;
        h.state_mut().setup.error = Some("Connection interrupted".into());
        h.run_ok();
        assert!(h.get_by_label("Retry setup").rect().bottom() <= viewport[1]);
        assert!(!dir.path().join("user.sqlite").exists());
    }
}

fn wait(harness: &mut Harness<'_, FontesApp>) {
    let start = Instant::now();
    while harness.state().pending > 0 {
        assert!(
            start.elapsed() < Duration::from_secs(15),
            "Engine worker did not finish"
        );
        std::thread::sleep(Duration::from_millis(10));
        harness.step();
    }
    harness.run();
}

#[test]
fn setup_actions_respect_system_safe_areas() {
    for (size, margins) in [
        ([390.0, 844.0], [0.0, 30.0, 0.0, 24.0]),
        ([780.0, 390.0], [36.0, 0.0, 24.0, 0.0]),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut h = Harness::builder()
            .with_size(size)
            .build_eframe(|cc| FontesApp::new(cc, None, dir.path().to_owned()));
        let [left, top, right, bottom] = margins;
        h.input_mut().safe_area_insets = Some(egui::SafeAreaInsets(egui::epaint::MarginF32 {
            left,
            top,
            right,
            bottom,
        }));
        h.run_ok();
        let safe = egui::Rect::from_min_max(
            egui::pos2(left, top),
            egui::pos2(size[0] - right, size[1] - bottom),
        );
        assert!(
            safe.contains_rect(h.get_by_label("Continue").rect()),
            "size {size:?}, safe {safe:?}, action {:?}",
            h.get_by_label("Continue").rect()
        );
    }
}

#[test]
#[ignore = "requires FONTES_TEST_LIBRARY and a working wgpu adapter"]
fn real_library_safe_areas() {
    let library = PathBuf::from(std::env::var("FONTES_TEST_LIBRARY").unwrap());
    let out = PathBuf::from(std::env::var("FONTES_CAPTURE_DIR").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    for (name, size, margins) in [
        ("portrait", [390.0, 844.0], [0.0, 30.0, 0.0, 24.0]),
        ("landscape", [780.0, 390.0], [36.0, 0.0, 24.0, 0.0]),
        ("keyboard", [390.0, 844.0], [0.0, 30.0, 0.0, 300.0]),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut h = Harness::builder()
            .with_size(size)
            .build_eframe(|cc| FontesApp::new(cc, Some(library.clone()), dir.path().to_owned()));
        let [left, top, right, bottom] = margins;
        h.input_mut().safe_area_insets = Some(egui::SafeAreaInsets(egui::epaint::MarginF32 {
            left,
            top,
            right,
            bottom,
        }));
        wait(&mut h);
        let safe = egui::Rect::from_min_max(
            egui::pos2(left, top),
            egui::pos2(size[0] - right, size[1] - bottom),
        );
        let reference = h
            .get_by_role_and_label(egui::accesskit::Role::Button, "John 3:16")
            .rect();
        assert!(
            safe.contains_rect(reference),
            "{name}: toolbar {reference:?} must fit {safe:?}"
        );
        for title in ["Read", "Search", "Notes", "Study"] {
            assert!(
                safe.contains_rect(h.get_by_label(title).rect()),
                "{name}: navigation must avoid system UI"
            );
        }
        h.render()
            .unwrap()
            .save(out.join(format!("{name}-reading.png")))
            .unwrap();
        h.get_by_role_and_label(egui::accesskit::Role::Button, "John 3:16")
            .click();
        h.run_ok();
        assert!(safe.contains_rect(h.get_by_label("Open John 3:16").rect()));
        assert!(safe.contains_rect(h.get_by_label("Close").rect()));
        h.render()
            .unwrap()
            .save(out.join(format!("{name}-picker.png")))
            .unwrap();
        h.get_by_label("Close").click();
        h.run_ok();
        let verse = h
            .state()
            .state
            .as_ref()
            .unwrap()
            .passage
            .iter()
            .find(|v| v.refs.contains(&43003016))
            .unwrap()
            .clone();
        h.get_by_value(&verse.text).click_secondary();
        h.run_ok();
        assert!(h.state().verse_actions.is_some());
        assert!(safe.contains_rect(h.get_by_label("Close actions").rect()));
        h.get_by_label("Copy verse").scroll_to_me();
        h.run_ok();
        assert!(safe.contains_rect(h.get_by_label("Copy verse").rect()));
        h.render()
            .unwrap()
            .save(out.join(format!("{name}-actions.png")))
            .unwrap();
        h.get_by_label("Close actions").click();
        h.run_ok();
        h.get_by_label("Reading preferences").click();
        h.run_ok();
        assert!(safe.contains_rect(h.get_by_label("Done").rect()));
        h.get_by_label("Library settings").scroll_to_me();
        h.run_ok();
        assert!(safe.contains_rect(h.get_by_label("Library settings").rect()));
        h.get_by_label("Done").click();
        h.run_ok();
        assert!(!h.state().reading_settings);
        // Insets change independently of the viewport (system bars appear/disappear).
        h.input_mut().safe_area_insets = Some(egui::SafeAreaInsets(egui::epaint::MarginF32 {
            top: top + 12.0,
            ..Default::default()
        }));
        h.run_ok();
        assert!(
            h.get_by_role_and_label(egui::accesskit::Role::Button, "John 3:16")
                .rect()
                .top()
                >= top + 12.0
        );
    }
}
/// Real-corpus acceptance and actual wgpu rendering without a desktop/window requirement.
#[test]
#[ignore = "requires FONTES_TEST_LIBRARY and a working wgpu adapter"]
fn real_library_desktop_and_mobile() {
    let library =
        PathBuf::from(std::env::var("FONTES_TEST_LIBRARY").expect("Set FONTES_TEST_LIBRARY"));
    let out = PathBuf::from(std::env::var("FONTES_CAPTURE_DIR").expect("Set FONTES_CAPTURE_DIR"));
    std::fs::create_dir_all(&out).unwrap();
    for (name, size) in [
        ("desktop", [1440.0, 960.0]),
        ("mobile", [390.0, 844.0]),
        ("small-mobile", [320.0, 640.0]),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut h = Harness::builder()
            .with_size(size)
            .build_eframe(|cc| FontesApp::new(cc, Some(library.clone()), dir.path().to_owned()));
        wait(&mut h);
        assert!(h.state().error.is_none(), "{:?}", h.state().error);
        assert!(
            h.state()
                .state
                .as_ref()
                .unwrap()
                .passage
                .iter()
                .any(|v| v.text.contains("For God so loved"))
        );
        h.render()
            .unwrap()
            .save(out.join(format!("{name}-reading.png")))
            .unwrap();
        h.get_by_label("Translation").click();
        h.run_ok();
        h.get_by_label("WEB").click();
        h.run_ok();
        wait(&mut h);
        let workspace = h.state().state.as_ref().unwrap();
        assert_eq!(workspace.session.selected_ref, 43003016);
        assert_eq!(
            workspace
                .translations
                .iter()
                .find(|t| t.id == workspace.session.translation)
                .unwrap()
                .abbr,
            "WEB"
        );
        h.get_by_label("Translation").click();
        h.run_ok();
        h.get_by_label("KJV").click();
        h.run_ok();
        wait(&mut h);
        if name != "desktop" {
            h.get_by_label("Reading preferences").click();
            h.run_ok();
            assert!(h.state().reading_settings);
            assert!(h.state().destination == Destination::Read);
            assert!(h.get_by_label("Done").rect().bottom() <= size[1]);
            h.render()
                .unwrap()
                .save(out.join(format!("{name}-reading-style.png")))
                .unwrap();
            h.get_by_label("IBM Plex Sans").click();
            h.run_ok();
            wait(&mut h);
            assert!(
                !h.state()
                    .state
                    .as_ref()
                    .unwrap()
                    .session
                    .preferences
                    .scripture_serif
            );
            h.get_by_label("Lora").click();
            h.run_ok();
            wait(&mut h);
            assert!(
                h.state()
                    .state
                    .as_ref()
                    .unwrap()
                    .session
                    .preferences
                    .scripture_serif
            );
            h.get_by_label("Done").click();
            h.run_ok();
            assert!(!h.state().reading_settings);
            assert!(h.state().destination == Destination::Read);
            h.get_by_label("Reading preferences").click();
            h.run_ok();
            h.key_press(egui::Key::Escape);
            h.run_ok();
            assert!(!h.state().reading_settings);
            for (reference, heading, capture) in [
                ("Proverbs 16:1", "Proverbs 16", "chapter-heading"),
                (
                    "1 Thessalonians 1:1",
                    "1 Thessalonians 1",
                    "long-chapter-heading",
                ),
            ] {
                h.state_mut().dispatch(Command::Navigate {
                    reference: reference.into(),
                });
                wait(&mut h);
                h.get_by_label(heading).scroll_to_me();
                h.run_ok();
                let rect = h.get_by_label(heading).rect();
                assert!(
                    rect.left() >= 0.0 && rect.right() <= size[0],
                    "{name}: {heading} bounds {rect:?}, viewport {size:?}"
                );
                let translation = h.get_by_label("King James Version").rect();
                assert!(translation.top() > rect.bottom());
                let workspace = h.state().state.as_ref().unwrap();
                let passage = h
                    .get_by_role_and_label(
                        egui::accesskit::Role::Button,
                        &workspace.reference_label(),
                    )
                    .rect();
                let translation_control = h.get_by_label("Translation").rect();
                let settings = h.get_by_label("Reading preferences").rect();
                assert!(passage.left() >= 0.0 && settings.right() <= size[0]);
                assert!(passage.right() <= translation_control.left());
                assert_eq!(passage.height(), 44.0);
                assert_eq!(translation_control.height(), passage.height());
                assert_eq!(settings.height(), passage.height());
                assert_eq!(settings.center().y, passage.center().y);
                h.render()
                    .unwrap()
                    .save(out.join(format!("{name}-{capture}.png")))
                    .unwrap();
                if capture == "chapter-heading" {
                    let verse = h
                        .state()
                        .state
                        .as_ref()
                        .unwrap()
                        .passage
                        .iter()
                        .find(|v| v.refs.contains(&20016001))
                        .unwrap()
                        .clone();
                    let start = verse.text[..verse.text.find("LORD").unwrap()]
                        .chars()
                        .count();
                    h.state_mut().selected_text = Some(SelectedText {
                        verse_id: verse.id.clone(),
                        ref_id: 20016001,
                        range: start..start + 4,
                        text: "LORD".into(),
                    });
                    h.get_by_value(&verse.text).click_secondary();
                    h.run_ok();
                    assert!(h.state().verse_actions.is_some());
                    let lookup_title = if verse.strongs_links.iter().any(|l| l.strongs == "H3068") {
                        "Look up Strong’s H3068"
                    } else {
                        "Look up original word…"
                    };
                    for title in [
                        lookup_title,
                        "Copy selection",
                        "Study this verse",
                        "Add note",
                        "Highlight verse · amber",
                        "Copy verse",
                    ] {
                        let action = h.get_by_label(title);
                        action.scroll_to_me();
                        h.run_ok();
                        let rect = h.get_by_label(title).rect();
                        assert!(rect.height() >= 44.0 && rect.width() >= 44.0);
                        assert!(rect.left() >= 0.0 && rect.right() <= size[0]);
                        assert!(rect.top() >= 0.0 && rect.bottom() <= size[1]);
                    }
                    h.get_by_label(lookup_title).scroll_to_me();
                    h.run_ok();
                    h.render()
                        .unwrap()
                        .save(out.join(format!("{name}-word-actions.png")))
                        .unwrap();
                    h.get_by_label("Close actions").click();
                    h.run_ok();
                    assert!(h.state().verse_actions.is_none());
                    h.get_by_value(&verse.text).click_secondary();
                    h.run_ok();
                    h.key_press(egui::Key::Escape);
                    h.run_ok();
                    assert!(h.state().verse_actions.is_none());
                    h.get_by_value(&verse.text).click_secondary();
                    h.run_ok();
                    h.get_by_label("Add note").scroll_to_me();
                    h.run_ok();
                    h.get_by_label("Add note").click();
                    h.run_ok();
                    assert!(h.state().verse_actions.is_none());
                    assert_eq!(h.state().note.as_ref().unwrap().start, 20016001);
                    h.get_by_label("Cancel").click();
                    h.run_ok();
                    h.state_mut().selected_text = None;
                    h.state_mut().verse_actions = Some(verse.clone());
                    h.run_ok();
                    assert!(h.query_by_label("Copy selection").is_none());
                    h.get_by_label("Copy verse").click();
                    h.run_ok();
                    assert!(h.state().verse_actions.is_none());
                }
            }
            h.state_mut().dispatch(Command::Navigate {
                reference: "John 3:16".into(),
            });
            wait(&mut h);
        }
        h.get_by_role_and_label(egui::accesskit::Role::Button, "John 3:16")
            .click();
        h.run_ok();
        assert!(h.state().picker);
        assert_eq!(h.state().picker_stage, PickerStage::Book);
        h.render()
            .unwrap()
            .save(out.join(format!("{name}-picker-books.png")))
            .unwrap();
        h.get_by_label("Old Testament").click();
        h.run_ok();
        h.get_by_label("Prov").scroll_to_me();
        h.run_ok();
        h.get_by_label("Prov").click();
        h.run_ok();
        assert_eq!(h.state().picker_stage, PickerStage::Chapter);
        assert!(h.query_by_label("Old Testament").is_none());
        let first = h
            .get_by_role_and_label(egui::accesskit::Role::Button, "1")
            .rect();
        assert!(first.top() >= 0.0 && first.bottom() < size[1] - 80.0);
        h.render()
            .unwrap()
            .save(out.join(format!("{name}-picker-chapters.png")))
            .unwrap();
        h.get_by_role_and_label(egui::accesskit::Role::Button, "16")
            .scroll_to_me();
        h.run_ok();
        h.get_by_role_and_label(egui::accesskit::Role::Button, "16")
            .click();
        h.run_ok();
        assert_eq!(h.state().picker_stage, PickerStage::Verse);
        assert_eq!(
            h.state().state.as_ref().unwrap().session.selected_ref,
            43003016
        );
        let first = h
            .get_by_role_and_label(egui::accesskit::Role::Button, "1")
            .rect();
        assert!(first.top() >= 0.0 && first.bottom() < size[1] - 80.0);
        h.render()
            .unwrap()
            .save(out.join(format!("{name}-picker-verses.png")))
            .unwrap();
        h.get_by_role_and_label(egui::accesskit::Role::Button, "1")
            .click();
        h.run_ok();
        wait(&mut h);
        assert!(!h.state().picker);
        assert_eq!(
            h.state().state.as_ref().unwrap().session.selected_ref,
            20016001
        );
        h.get_by_role_and_label(egui::accesskit::Role::Button, "Proverbs 16:1")
            .click();
        h.run_ok();
        h.get_by_label("Prov").scroll_to_me();
        h.run_ok();
        h.get_by_label("Prov").click();
        h.run_ok();
        h.get_by_role_and_label(egui::accesskit::Role::Button, "16")
            .click();
        h.run_ok();
        h.get_by_label("Chapter\n16").click();
        h.run_ok();
        assert_eq!(h.state().picker_stage, PickerStage::Chapter);
        h.get_by_role_and_label(egui::accesskit::Role::Button, "2")
            .click();
        h.run_ok();
        let open = h.get_by_label("Open Proverbs 2:1").rect();
        assert!(open.right() <= size[0] && open.bottom() <= size[1]);
        h.get_by_label("Open Proverbs 2:1").click();
        h.run_ok();
        wait(&mut h);
        assert_eq!(
            h.state().state.as_ref().unwrap().session.selected_ref,
            20002001
        );
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
        h.run_ok();
        h.get_by_label("Book\nProv").click();
        h.run_ok();
        if name == "small-mobile" {
            h.get_by_label("Ps").scroll_to_me();
            h.run_ok();
            h.get_by_label("Ps").click();
            h.run_ok();
            assert!(h.query_by_label("150").is_some());
            h.get_by_role_and_label(egui::accesskit::Role::Button, "119")
                .scroll_to_me();
            h.run_ok();
            h.get_by_role_and_label(egui::accesskit::Role::Button, "119")
                .click();
            h.run_ok();
            let first = h
                .get_by_role_and_label(egui::accesskit::Role::Button, "1")
                .rect();
            assert!(first.top() >= 0.0 && first.bottom() < size[1] - 80.0);
            h.get_by_role_and_label(egui::accesskit::Role::Button, "176")
                .scroll_to_me();
            h.run_ok();
            h.get_by_role_and_label(egui::accesskit::Role::Button, "176")
                .click();
            h.run_ok();
            wait(&mut h);
            assert_eq!(
                h.state().state.as_ref().unwrap().session.selected_ref,
                19119176
            );
            h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
            h.run_ok();
            h.get_by_label("Ps").scroll_to_me();
            h.run_ok();
            h.get_by_label("Ps").click();
            h.run_ok();
            h.get_by_role_and_label(egui::accesskit::Role::Button, "150")
                .scroll_to_me();
            h.run_ok();
            h.get_by_role_and_label(egui::accesskit::Role::Button, "150")
                .click();
            h.run_ok();
            assert_eq!(h.state().picker_verse, 1);
            assert!(h.query_by_label("176").is_none());
            h.get_by_role_and_label(egui::accesskit::Role::Button, "6")
                .click();
            h.run_ok();
            wait(&mut h);
            assert_eq!(
                h.state().state.as_ref().unwrap().session.selected_ref,
                19150006
            );
            h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
            h.run_ok();
        }
        h.get_by_label("New Testament").click();
        h.run_ok();
        h.get_by_label("John").click();
        h.run_ok();
        h.get_by_role_and_label(egui::accesskit::Role::Button, "3")
            .click();
        h.run_ok();
        assert!(h.get_by_label("Open John 3:1").rect().right() <= size[0]);
        h.get_by_role_and_label(egui::accesskit::Role::Button, "16")
            .scroll_to_me();
        h.run_ok();
        h.get_by_role_and_label(egui::accesskit::Role::Button, "16")
            .click();
        h.run_ok();
        wait(&mut h);
        assert!(!h.state().picker);
        assert_eq!(
            h.state().state.as_ref().unwrap().session.selected_ref,
            43003016
        );
        h.get_by_label("Notes").click();
        h.run_ok();
        h.get_by_label("Add a note to selected verse").click();
        h.run_ok();
        h.state_mut().note.as_mut().unwrap().text = "A render-test observation".into();
        h.run_ok();
        h.render()
            .unwrap()
            .save(out.join(format!("{name}-note.png")))
            .unwrap();
        h.get_by_label("Save").click();
        h.run_ok();
        wait(&mut h);
        assert_eq!(h.state().state.as_ref().unwrap().annotations.len(), 1);
        assert!(h.state().note.is_none());
        // A replacement snapshot at the same canonical location must invalidate old content caches.
        h.state_mut().state.as_mut().unwrap().content_id = "previous-snapshot".into();
        h.state_mut().word = Some(Data::Word {
            code: "G0025".into(),
            headwords: vec![],
            definitions: vec![],
            occurrences: vec![],
            total: 0,
        });
        h.state_mut().dispatch(Command::Workspace {});
        wait(&mut h);
        assert!(h.state().word.is_none());
        h.get_by_label("Search").click();
        h.run_ok();
        h.state_mut().query = "forgiveness".into();
        h.state_mut().dispatch(Command::Search {
            query: "forgiveness".into(),
            limit: 30,
            offset: 0,
        });
        wait(&mut h);
        assert!(!h.state().hits.is_empty());
        h.render()
            .unwrap()
            .save(out.join(format!("{name}-search.png")))
            .unwrap();
        h.get_by_label("Study").click();
        h.run_ok();
        wait(&mut h);
        h.render()
            .unwrap()
            .save(out.join(format!("{name}-study.png")))
            .unwrap();
        if h.state().state.as_ref().unwrap().has_study {
            h.get_by_label("Words").click();
            h.run_ok();
            let surface = match h.state().context.as_ref().unwrap() {
                Data::Context { tokens, .. } => tokens
                    .iter()
                    .find(|t| t.strongs.as_deref() == Some("G0025"))
                    .unwrap()
                    .surface
                    .clone(),
                _ => panic!(),
            };
            h.get_by_label("G0025").click();
            h.run_ok();
            wait(&mut h);
            let heading = h.get_by_label(&surface);
            heading.scroll_to_me();
            h.run_ok();
            assert!(h.get_by_label(&surface).rect().height() >= 40.0);
            h.render()
                .unwrap()
                .save(out.join(format!("{name}-greek-word-heading.png")))
                .unwrap();
            h.state_mut().word_focus = None;
            h.state_mut().dispatch(Command::Word {
                code: "H3068".into(),
            });
            wait(&mut h);
            let text = match h.state().word.as_ref().unwrap() {
                Data::Word { headwords, .. } => headwords[0].text.clone(),
                _ => panic!(),
            };
            h.get_by_label(&text).scroll_to_me();
            h.run_ok();
            let rect = h.get_by_label(&text).rect();
            assert!(rect.height() >= 40.0 && rect.right() <= size[0]);
            h.render()
                .unwrap()
                .save(out.join(format!("{name}-hebrew-word-heading.png")))
                .unwrap();
            h.get_by_label("← Words in this verse").click();
            h.run_ok();
        }
        if name == "desktop" && h.state().state.as_ref().unwrap().has_study {
            let verse = h
                .state()
                .state
                .as_ref()
                .unwrap()
                .passage
                .iter()
                .find(|v| v.refs.contains(&43003016))
                .unwrap()
                .text
                .clone();
            assert!(
                h.get_by_value(&verse).rect().right() < h.get_by_label("Commentary").rect().left(),
                "Study must remain beside the reading surface"
            );
            h.get_by_value(&verse).focus();
            h.run_ok();
            h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
            h.run_ok();
            assert_eq!(h.state().selected_text.as_ref().unwrap().text, verse);
            h.key_press(egui::Key::ArrowLeft);
            h.run_ok();
            let loved_start = verse[..verse.find("loved").unwrap()].chars().count();
            for _ in 0..loved_start {
                h.key_press(egui::Key::ArrowRight);
            }
            for _ in 0..5 {
                h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowRight);
            }
            h.run_ok();
            assert_eq!(h.state().selected_text.as_ref().unwrap().text, "loved");
            h.get_by_value(&verse).click_secondary();
            h.run_ok();
            h.render()
                .unwrap()
                .save(out.join("desktop-word-menu.png"))
                .unwrap();
            let direct = h.query_by_label("Look up Strong’s G0025").is_some();
            h.get_by_label(if direct {
                "Look up Strong’s G0025"
            } else {
                "Look up original word…"
            })
            .click();
            h.run_ok();
            wait(&mut h);
            assert!(h.state().study_tab == StudyTab::Words);
            h.render()
                .unwrap()
                .save(out.join("desktop-original-words.png"))
                .unwrap();
            if !direct {
                h.get_by_label("G0025").click();
                h.run_ok();
                wait(&mut h);
            }
            assert!(
                matches!(&h.state().word, Some(Data::Word { code, definitions, .. }) if code == "G0025" && !definitions.is_empty())
            );
            assert_eq!(
                h.state().state.as_ref().unwrap().session.selected_ref,
                43003016
            );
            assert!(h.query_by_value(&verse).is_some());
            h.render()
                .unwrap()
                .save(out.join("desktop-word-definition.png"))
                .unwrap();
            // Synthetic alignment exercises the direct menu branch; the shipped corpus has none.
            let selected = h
                .state_mut()
                .state
                .as_mut()
                .unwrap()
                .passage
                .iter_mut()
                .find(|v| v.refs.contains(&43003016))
                .unwrap();
            let byte_start = selected.text.find("loved").unwrap();
            let start = selected.text[..byte_start].chars().count();
            selected.word_links.push(WordLink {
                start,
                end: start + 5,
                surface: "ηγαπησεν".into(),
                witness: "synthetic-ui-link".into(),
                strongs: "G0025".into(),
                method: "reviewed".into(),
            });
            h.run_ok();
            h.get_by_value(&verse).focus();
            h.run_ok();
            h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
            h.run_ok();
            h.get_by_value(&verse).click_secondary();
            h.run_ok();
            h.get_by_label("Look up Strong’s G0025").click();
            h.run_ok();
            wait(&mut h);
            assert!(matches!(&h.state().word, Some(Data::Word { code, .. }) if code == "G0025"));
            h.state_mut().dispatch(Command::Select { ref_id: 41001004 });
            wait(&mut h);
            assert!(h.state().word.is_none());
            assert!(h.state().word_focus.is_none());
            assert!(h.state().selected_text.is_none());
            assert!(matches!(
                &h.state().context,
                Some(Data::Context {
                    ref_id: 41001004,
                    ..
                })
            ));
            h.get_by_label("Commentary").click();
            h.run_ok();
            h.render()
                .unwrap()
                .save(out.join("desktop-mark-study.png"))
                .unwrap();
            h.get_by_label("Cross-references").click();
            h.run_ok();
            h.render()
                .unwrap()
                .save(out.join("desktop-mark-references.png"))
                .unwrap();
            let text = h
                .state()
                .state
                .as_ref()
                .unwrap()
                .passage
                .iter()
                .find(|v| v.refs.contains(&41001004))
                .unwrap()
                .text
                .clone();
            h.get_by_value(&text).focus();
            h.run_ok();
            h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
            h.run_ok();
            assert!(h.state().selected_text.is_some());
            h.state_mut().dispatch(Command::Translation {
                id: "translation:web".into(),
            });
            wait(&mut h);
            assert!(
                h.state().selected_text.is_none(),
                "Translation changes must clear native selection state as well as its view model"
            );
            assert_eq!(
                h.state().state.as_ref().unwrap().session.translation,
                "translation:web"
            );
        }
        if h.state().state.as_ref().unwrap().has_study {
            h.get_by_label("Commentary").click();
            h.run_ok();
            h.state_mut().dispatch(Command::Resource {
                id: "resource:matthew-henry:Mark.1.1".into(),
            });
            wait(&mut h);
            let resource = h.state().resource.as_ref().unwrap();
            assert!(resource.content.contains("Roma à fratribus"));
            assert!(!resource.content.contains("&#224;"));
            let paragraphs: Vec<_> = resource
                .content
                .lines()
                .filter(|p| !p.trim().is_empty())
                .map(str::to_owned)
                .collect();
            let first = h.get_by_label(&paragraphs[0]).rect();
            let second = h.get_by_label(&paragraphs[1]).rect();
            assert!(
                second.top() >= first.bottom() + 12.0,
                "Source paragraphs need visible spacing"
            );
            assert!(
                second.right() <= size[0],
                "Article prose must wrap inside the viewport"
            );
            h.render()
                .unwrap()
                .save(out.join(format!("{name}-matthew-henry.png")))
                .unwrap();
        }
    }
}

#[test]
#[ignore = "downloads a real signed library; requires internet and a wgpu adapter"]
fn first_run_downloads_and_opens_published_library() {
    let dir = tempfile::tempdir().unwrap();
    let out = PathBuf::from(std::env::var("FONTES_CAPTURE_DIR").expect("Set FONTES_CAPTURE_DIR"));
    std::fs::create_dir_all(&out).unwrap();
    let mut h = Harness::builder()
        .with_size([390.0, 844.0])
        .build_eframe(|cc| FontesApp::new(cc, None, dir.path().to_owned()));
    h.render()
        .unwrap()
        .save(out.join("first-run-choose.png"))
        .unwrap();
    h.get_by_label("Continue").click();
    h.run_ok();
    let start = Instant::now();
    while h.state().setup.offer.is_none() {
        assert!(
            start.elapsed() < Duration::from_secs(90),
            "Setup discovery timed out"
        );
        assert!(
            h.state().setup.error.is_none(),
            "{:?}",
            h.state().setup.error
        );
        std::thread::sleep(Duration::from_millis(20));
        h.step();
    }
    h.run_ok();
    h.render()
        .unwrap()
        .save(out.join("first-run-confirm.png"))
        .unwrap();
    h.get_by_label("Download and open").click();
    h.run_ok();
    while h.state().state.is_none() {
        assert!(
            start.elapsed() < Duration::from_secs(300),
            "Download timed out"
        );
        assert!(
            h.state().setup.error.is_none(),
            "{:?}",
            h.state().setup.error
        );
        assert!(h.state().error.is_none(), "{:?}", h.state().error);
        std::thread::sleep(Duration::from_millis(20));
        h.step();
    }
    h.run_ok();
    assert!(
        h.state()
            .state
            .as_ref()
            .unwrap()
            .passage
            .iter()
            .any(|v| v.text.contains("For God so loved"))
    );
    h.render()
        .unwrap()
        .save(out.join("first-run-reading.png"))
        .unwrap();
    h.state_mut().choose(Destination::Preferences);
    h.run_ok();
    h.get_by_label("Download a different library")
        .scroll_to_me();
    h.run_ok();
    h.get_by_label("Download a different library").click();
    h.run_ok();
    assert!(h.state().setup.visible);
    assert!(
        h.state().state.is_some(),
        "Reopening setup must retain the active library"
    );
    h.get_by_label("Back to your library").click();
    h.run_ok();
    assert!(!h.state().setup.visible);
    assert!(h.state().setup.job.is_none());
    drop(h);
    // Restart uses verified local content and does not start a download job.
    let mut h = Harness::builder()
        .with_size([390.0, 844.0])
        .build_eframe(|cc| FontesApp::new(cc, None, dir.path().to_owned()));
    wait(&mut h);
    assert!(h.state().state.is_some());
    assert!(h.state().setup.job.is_none());
}

#[test]
#[ignore = "requires the updated Hebrew Study library and a working wgpu adapter"]
fn real_library_ot_word_lookup() {
    let library = PathBuf::from(std::env::var("FONTES_TEST_LIBRARY").unwrap());
    let out = PathBuf::from(std::env::var("FONTES_CAPTURE_DIR").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    for (name, size) in [
        ("desktop", [1440.0, 960.0]),
        ("mobile", [390.0, 844.0]),
        ("small-mobile", [320.0, 640.0]),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut h = Harness::builder()
            .with_size(size)
            .build_eframe(|cc| FontesApp::new(cc, Some(library.clone()), dir.path().to_owned()));
        wait(&mut h);
        for (reference, code, capture) in [
            ("Genesis 1:1", "H7225", "genesis"),
            ("Proverbs 16:1", "H3068", "proverbs"),
            ("Psalm 3:1", "H1732", "psalm-title"),
        ] {
            h.state_mut().dispatch(Command::Navigate {
                reference: reference.into(),
            });
            wait(&mut h);
            let selected = h.state().state.as_ref().unwrap().session.selected_ref;
            h.state_mut().open_study(selected, StudyTab::Words);
            wait(&mut h);
            let surface = match h.state().context.as_ref().unwrap() {
                Data::Context { tokens, .. } => tokens
                    .iter()
                    .find(|t| t.strongs.as_deref() == Some(code))
                    .expect("source Hebrew word must be installed")
                    .surface
                    .clone(),
                _ => panic!(),
            };
            h.get_by_label(code).scroll_to_me();
            h.run_ok();
            h.get_by_label(code).click();
            h.run_ok();
            wait(&mut h);
            assert!(h.state().word_focus.is_some());
            assert!(
                matches!(h.state().word.as_ref(),Some(Data::Word {definitions,total,..}) if !definitions.is_empty() && *total>0)
            );
            h.get_by_label(&surface).scroll_to_me();
            h.run_ok();
            let rect = h.get_by_label(&surface).rect();
            assert!(rect.height() >= 40.0 && rect.left() >= 0.0 && rect.right() <= size[0]);
            h.input_mut().events.push(egui::Event::PointerGone);
            h.run_ok();
            h.render()
                .unwrap()
                .save(out.join(format!("{name}-{capture}-hebrew-lookup.png")))
                .unwrap();
            h.get_by_label("← Words in this verse").click();
            h.run_ok();
        }
        h.state_mut().dispatch(Command::Navigate {
            reference: "Nehemiah 7:68".into(),
        });
        wait(&mut h);
        let reference = h.state().state.as_ref().unwrap().session.selected_ref;
        h.state_mut().open_study(reference, StudyTab::Words);
        wait(&mut h);
        assert!(
            matches!(h.state().context.as_ref(),Some(Data::Context {tokens,..}) if tokens.is_empty()),
            "A source omission must not be filled with invented Hebrew"
        );
    }
}

#[test]
#[ignore = "requires a locally evaluated KJV-tagged Study candidate and wgpu"]
fn real_library_kjv_source_word_lookup() {
    let library = PathBuf::from(std::env::var("FONTES_TEST_LIBRARY").unwrap());
    let out = PathBuf::from(std::env::var("FONTES_CAPTURE_DIR").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    for (name, size) in [
        ("desktop", [1440.0, 960.0]),
        ("mobile", [390.0, 844.0]),
        ("small-mobile", [320.0, 640.0]),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut h = Harness::builder()
            .with_size(size)
            .build_eframe(|cc| FontesApp::new(cc, Some(library.clone()), dir.path().to_owned()));
        wait(&mut h);
        h.state_mut().dispatch(Command::Navigate {
            reference: "Proverbs 16:1".into(),
        });
        wait(&mut h);
        let verse = h
            .state()
            .state
            .as_ref()
            .unwrap()
            .passage
            .iter()
            .find(|v| v.refs.contains(&20016001))
            .unwrap()
            .clone();
        assert!(
            verse.word_links.is_empty(),
            "Source dictionary associations must not invent Hebrew token identity"
        );
        let start = verse.text[..verse.text.find("LORD").unwrap()]
            .chars()
            .count();
        assert!(
            verse
                .strongs_links
                .iter()
                .any(|l| l.start == start && l.end == start + 4 && l.strongs == "H3068")
        );
        h.get_by_value(&verse.text).scroll_to_me();
        h.run_ok();
        h.state_mut().selected_text = Some(SelectedText {
            verse_id: verse.id.clone(),
            ref_id: 20016001,
            range: start..start + 4,
            text: "LORD".into(),
        });
        h.get_by_value(&verse.text).click_secondary();
        h.run_ok();
        h.get_by_label("Look up Strong’s H3068").scroll_to_me();
        h.run_ok();
        h.render()
            .unwrap()
            .save(out.join(format!("{name}-direct-word-action.png")))
            .unwrap();
        h.get_by_label("Look up Strong’s H3068").click();
        h.run_ok();
        wait(&mut h);
        let headword = match h.state().word.as_ref().unwrap() {
            Data::Word {
                headwords, total, ..
            } => {
                assert!(*total > 0);
                headwords[0].text.clone()
            }
            _ => panic!(),
        };
        assert!(
            h.state().word_focus.is_none(),
            "A dictionary tag is not a chosen verse form"
        );
        h.get_by_label(&headword).scroll_to_me();
        h.run_ok();
        assert!(h.get_by_label(&headword).rect().height() >= 40.0);
        h.input_mut().events.push(egui::Event::PointerGone);
        h.run_ok();
        h.render()
            .unwrap()
            .save(out.join(format!("{name}-direct-hebrew-lookup.png")))
            .unwrap();
        h.state_mut().dispatch(Command::Translation {
            id: "translation:web".into(),
        });
        wait(&mut h);
        assert!(
            h.state()
                .state
                .as_ref()
                .unwrap()
                .passage
                .iter()
                .all(|v| v.strongs_links.is_empty()),
            "KJV tags must never transfer to WEB"
        );
    }
}
