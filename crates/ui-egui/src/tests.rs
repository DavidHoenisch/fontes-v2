use super::*;
use egui_kittest::{Harness, kittest::Queryable};
use std::time::{Duration, Instant};
#[test]
fn first_run_provides_recoverable_offline_open_flow() {
    let dir = tempfile::tempdir().unwrap();
    for size in [[320.0, 640.0], [390.0, 844.0], [1440.0, 960.0]] {
        let mut harness = Harness::builder()
            .with_size(size)
            .build_eframe(|cc| FontesApp::new(cc, None, dir.path().to_owned()));
        harness.get_by_label("Open library").click();
        harness.run_ok();
        wait(&mut harness);
        assert!(harness.state().state.is_none());
        assert!(harness.state().error.is_some());
        assert!(harness.get_by_label("Open library").rect().width() <= size[0]);
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
        h.get_by_role_and_label(egui::accesskit::Role::Button, "John 3:16")
            .click();
        h.run_ok();
        assert!(h.state().picker);
        h.state_mut().picker_book = Some(43);
        h.state_mut().picker_chapter = 3;
        h.state_mut().picker_verse = 16;
        h.run_ok();
        assert!(h.get_by_label("Open John 3:16").rect().right() <= size[0]);
        h.render()
            .unwrap()
            .save(out.join(format!("{name}-picker.png")))
            .unwrap();
        h.get_by_label("Open John 3:16").click();
        h.run_ok();
        wait(&mut h);
        assert!(!h.state().picker);
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
            h.get_by_label("Look up original word…").click();
            h.run_ok();
            wait(&mut h);
            assert!(h.state().study_tab == StudyTab::Words);
            h.render()
                .unwrap()
                .save(out.join("desktop-original-words.png"))
                .unwrap();
            h.get_by_label("G0025").click();
            h.run_ok();
            wait(&mut h);
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
