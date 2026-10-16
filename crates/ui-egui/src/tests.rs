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
        h.state_mut().choose(Destination::Study);
        wait(&mut h);
        h.render()
            .unwrap()
            .save(out.join(format!("{name}-study.png")))
            .unwrap();
    }
}
