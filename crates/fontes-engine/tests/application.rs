use fontes_data_contracts::{CANONICAL_REGISTRY_JSON, CONTENT_SCHEMA_SQL};
use fontes_engine::transport::{MAX_REQUEST_BYTES, control_request, mcp_request, serve};
use fontes_engine::*;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use tempfile::TempDir;
struct Fixture {
    dir: TempDir,
}
impl Fixture {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let mut db = Connection::open(dir.path().join("content.sqlite")).unwrap();
        db.execute_batch(CONTENT_SCHEMA_SQL).unwrap();
        let registry: Value = serde_json::from_str(CANONICAL_REGISTRY_JSON).unwrap();
        let tx = db.transaction().unwrap();
        tx.execute(
            "INSERT INTO build_info VALUES(1,'fixture-content','study',1,1,'digest','test')",
            [],
        )
        .unwrap();
        for b in registry["books"].as_array().unwrap() {
            let id = b["id"].as_i64().unwrap();
            tx.execute(
                "INSERT INTO books VALUES(?1,?2,?3,?4,?1)",
                params![
                    id,
                    b["osis"].as_str(),
                    b["name"].as_str(),
                    b["testament"].as_str()
                ],
            )
            .unwrap();
            for (c, count) in b["verse_counts"].as_array().unwrap().iter().enumerate() {
                for v in 1..=count.as_i64().unwrap() {
                    tx.execute(
                        "INSERT INTO canonical_refs VALUES(?1,?2,?3,?4)",
                        params![id * 1_000_000 + (c as i64 + 1) * 1000 + v, id, c + 1, v],
                    )
                    .unwrap();
                }
            }
        }
        tx.execute_batch("INSERT INTO source_snapshots VALUES('source','fixture','hash','https://example.org/source',NULL,'fixture-rights','test');
            INSERT INTO text_witnesses VALUES('kjv-witness','KJV','en','fixture','KJV','source'),('web-witness','WEB','en','fixture','KJV','source');
            INSERT INTO translations VALUES('kjv','KJV','King James Version','en','kjv-witness'),('web','WEB','World English Bible','en','web-witness');
            INSERT INTO source_segments VALUES('kjv16','source','kjv-witness','John 3:16','kjv16'),('web16','source','web-witness','John 3:16','web16'),('merged','source','web-witness','John 3:18-19','merged');
            INSERT INTO segment_refs VALUES('kjv16',43003016,'exact'),('web16',43003016,'exact'),('merged',43003018,'merged'),('merged',43003019,'merged');
            INSERT INTO verse_texts VALUES('kjv16','kjv','kjv16','For God so loved the world.',NULL),('web16','web','web16','For God so loved the world, WEB.',NULL),('merged','web','merged','Merged source text.',NULL);
            INSERT INTO reference_ranges VALUES('john16',43003016,43003016),('rom1',45001003,45001003);
            INSERT INTO resources VALUES('mh','source','mh','commentary','Matthew Henry','Matthew Henry','A commentary about love.',NULL),('nave','source','nave','topic','Love','Nave','A second resource about love.',NULL),('tsk','source','tsk','cross_reference','TSK','TSK','See Romans 1:3.',NULL),('strong','source','strong','dictionary','G0025',NULL,'To love.',NULL);
            INSERT INTO strongs_codes VALUES('G0025','grc');
            INSERT INTO resource_links VALUES('mh-link','mh','john16',NULL,NULL,'applies_to'),('nave-link','nave','john16',NULL,NULL,'citation'),('tsk-origin','tsk','john16',NULL,NULL,'applies_to'),('tsk-target','tsk','rom1',NULL,NULL,'cross_reference'),('definition','strong',NULL,'G0025',NULL,'definition');
            INSERT INTO original_tokens VALUES('token','kjv-witness','kjv16','ἠγάπησεν',1,NULL);
            INSERT INTO token_analyses VALUES('token',0,NULL,'robinson','V-AAI-3S'),('token',1,NULL,'robinson','V-API-3S');
            INSERT INTO token_strongs VALUES('token',0,'G0025'),('token',1,'G0025');
            INSERT INTO search_documents VALUES(1,'kjv16',NULL,'For God so loved the world.'),(2,'web16',NULL,'For God so loved the world, WEB.'),(3,NULL,'mh','A commentary about love.'),(4,NULL,'nave','A second resource about love.');
            INSERT INTO search_fts(search_fts) VALUES('rebuild');").unwrap();
        tx.commit().unwrap();
        Self { dir }
    }
    fn engine(&self) -> Engine {
        Engine::open(
            &self.dir.path().join("content.sqlite"),
            &self.dir.path().join("user.sqlite"),
        )
        .unwrap()
    }
}
#[test]
fn navigation_aliases_ambiguity_and_bounds_preserve_selection() {
    let f = Fixture::new();
    let mut e = f.engine();
    for reference in ["John 3:16", "j3 16", "Jn 3:16", "JOHN 3:16"] {
        assert_eq!(e.resolve(reference).unwrap(), 43003016);
    }
    assert_eq!(e.resolve("1 John 2:3").unwrap(), 62002003);
    assert_eq!(e.resolve("Psalm 23").unwrap(), 19023001);
    for bad in [
        "Jo 3:16",
        "John 0:1001",
        "John 3:1001",
        "John 22",
        "John 3:0",
        "unknown 3",
        "John 3:16-18",
        "",
        "Rom 1:3 extra",
    ] {
        assert!(e.resolve(bad).is_err(), "{bad}");
    }
    let before = e.workspace().unwrap().session.selected_ref;
    assert!(
        e.dispatch(Command::Navigate {
            reference: "John 99:99".into()
        })
        .is_err()
    );
    assert_eq!(e.workspace().unwrap().session.selected_ref, before);
    e.dispatch(Command::Navigate {
        reference: "Matthew 28:20".into(),
    })
    .unwrap();
    assert_eq!(
        e.dispatch(Command::Chapter { delta: 1 })
            .unwrap()
            .state
            .reference_label(),
        "Mark 1:1"
    );
    assert_eq!(
        e.dispatch(Command::Chapter { delta: -1 })
            .unwrap()
            .state
            .reference_label(),
        "Matthew 28:1"
    );
}
#[test]
fn merged_source_segments_render_once_and_translation_preserves_notes() {
    let f = Fixture::new();
    let mut e = f.engine();
    let saved = e
        .dispatch(Command::SaveAnnotation {
            id: None,
            start_ref: 43003016,
            end_ref: 43003019,
            kind: AnnotationKind::Note,
            color: None,
            content: Some("A private observation".into()),
        })
        .unwrap();
    let Data::SavedAnnotation { id } = saved.data else {
        panic!()
    };
    let state = e
        .dispatch(Command::Translation { id: "web".into() })
        .unwrap()
        .state;
    assert_eq!(state.session.selected_ref, 43003016);
    assert_eq!(state.annotations[0].id, id);
    assert_eq!(state.passage.len(), 2);
    assert_eq!(state.passage[1].refs, vec![43003018, 43003019]);
    assert!(!state.passage.iter().any(|v| v.refs.contains(&43003017)));
    drop(e);
    let mut e = f.engine();
    assert_eq!(e.workspace().unwrap().session.translation, "web");
    assert_eq!(
        e.workspace().unwrap().annotations[0].content.as_deref(),
        Some("A private observation")
    );
    e.dispatch(Command::SaveAnnotation {
        id: Some(id.clone()),
        start_ref: 45001003,
        end_ref: 45001003,
        kind: AnnotationKind::Note,
        color: None,
        content: Some("Edited".into()),
    })
    .unwrap();
    assert_eq!(e.workspace().unwrap().annotations[0].start_ref, 45001003);
    e.dispatch(Command::DeleteAnnotation { id }).unwrap();
    assert!(e.workspace().unwrap().annotations.is_empty());
}
#[test]
fn rejects_bad_notes_preferences_and_ids_without_partial_writes() {
    let f = Fixture::new();
    let mut e = f.engine();
    for (start, end, color) in [
        (43003017, 43003016, "amber"),
        (43003016, 43003016, "purple"),
        (43099016, 43099016, "amber"),
    ] {
        assert!(
            e.dispatch(Command::SaveAnnotation {
                id: None,
                start_ref: start,
                end_ref: end,
                kind: AnnotationKind::Highlight,
                color: Some(color.into()),
                content: None
            })
            .is_err()
        );
    }
    assert!(e.workspace().unwrap().annotations.is_empty());
    assert!(
        e.dispatch(Command::Preferences {
            preferences: Preferences {
                text_size: f32::NAN,
                ..Default::default()
            }
        })
        .is_err()
    );
    assert!(
        e.dispatch(Command::Translation {
            id: "not-installed".into()
        })
        .is_err()
    );
    e.dispatch(Command::Favorite {
        ref_id: 43003016,
        enabled: true,
    })
    .unwrap();
    e.dispatch(Command::Favorite {
        ref_id: 43003016,
        enabled: true,
    })
    .unwrap();
    assert_eq!(e.workspace().unwrap().saved.len(), 1);
    e.dispatch(Command::Favorite {
        ref_id: 43003016,
        enabled: false,
    })
    .unwrap();
    assert!(e.workspace().unwrap().saved.is_empty());
}
#[test]
fn context_keeps_typed_links_and_source_analyses() {
    let f = Fixture::new();
    let mut e = f.engine();
    let Data::Context {
        resources,
        tokens,
        cross_references,
        ..
    } = e
        .dispatch(Command::Context { ref_id: 43003016 })
        .unwrap()
        .data
    else {
        panic!()
    };
    assert!(
        resources
            .iter()
            .any(|r| r.id == "mh" && r.basis == "applies_to")
    );
    assert!(
        resources
            .iter()
            .any(|r| r.id == "nave" && r.basis == "citation")
    );
    assert_eq!(cross_references.len(), 1);
    assert_eq!(cross_references[0].start_ref, 45001003);
    assert_eq!(cross_references[0].label, "Romans 1:3");
    assert!(
        cross_references[0].preview.is_none(),
        "Unavailable verse text must not be inferred"
    );
    assert_eq!(tokens.len(), 2);
    assert!(tokens.iter().all(|t| t.lemma.is_none()));
    let Data::Resource { resource } = e
        .dispatch(Command::Resource { id: "tsk".into() })
        .unwrap()
        .data
    else {
        panic!()
    };
    assert!(
        resource
            .links
            .iter()
            .any(|l| l.start_ref == Some(45001003) && l.basis == "cross_reference")
    );
    let Data::Word {
        code,
        total,
        occurrences,
        definitions,
        headwords,
    } = e
        .dispatch(Command::Word { code: "g25".into() })
        .unwrap()
        .data
    else {
        panic!()
    };
    assert_eq!(code, "G0025");
    assert_eq!(total, 1);
    assert_eq!(occurrences.len(), 1);
    assert_eq!(definitions.len(), 1);
    assert!(
        headwords.is_empty(),
        "A legacy title/definition is not structured headword metadata"
    );
}
#[test]
fn dictionary_headwords_use_structured_metadata_and_preserve_language_text() {
    let f = Fixture::new();
    let db = Connection::open(f.dir.path().join("content.sqlite")).unwrap();
    db.execute(
        "UPDATE resources SET title='Unstructured display title',display_json=?1 WHERE id='strong'",
        [
            json!({"type":"dictionary_entry","headword":"ἀγαπάω","transliteration":"agapáō"})
                .to_string(),
        ],
    )
    .unwrap();
    db.execute_batch("INSERT INTO strongs_codes VALUES('H3068','he');
        INSERT INTO resources VALUES('hebrew','source','hebrew','dictionary','Unstructured Hebrew title',NULL,'A definition.',NULL);
        INSERT INTO resource_links VALUES('hebrew-definition','hebrew',NULL,'H3068',NULL,'definition');").unwrap();
    db.execute(
        "UPDATE resources SET display_json=?1 WHERE id='hebrew'",
        [
            json!({"type":"dictionary_entry","headword":"יְהֹוָה","transliteration":"Yᵉhôvâh"})
                .to_string(),
        ],
    )
    .unwrap();
    let mut engine = f.engine();
    for (code, text, transliteration, id) in [
        ("G25", "ἀγαπάω", "agapáō", "strong"),
        ("H3068", "יְהֹוָה", "Yᵉhôvâh", "hebrew"),
    ] {
        let Data::Word { headwords, .. } = engine
            .dispatch(Command::Word { code: code.into() })
            .unwrap()
            .data
        else {
            panic!()
        };
        assert_eq!(headwords.len(), 1);
        assert_eq!(headwords[0].text, text);
        assert_eq!(headwords[0].resource_id, id);
        assert_eq!(
            headwords[0].transliteration.as_deref(),
            Some(transliteration)
        );
    }
    db.execute(
        "UPDATE resources SET display_json=?1 WHERE id='strong'",
        [json!({"type":"other","headword":"Not a dictionary headword"}).to_string()],
    )
    .unwrap();
    let Data::Word { headwords, .. } = engine
        .dispatch(Command::Word { code: "G25".into() })
        .unwrap()
        .data
    else {
        panic!()
    };
    assert!(headwords.is_empty());
    db.execute(
        "UPDATE resources SET display_json='malformed legacy metadata' WHERE id='strong'",
        [],
    )
    .unwrap();
    let Data::Word {
        headwords,
        definitions,
        ..
    } = engine
        .dispatch(Command::Word { code: "G25".into() })
        .unwrap()
        .data
    else {
        panic!()
    };
    assert!(headwords.is_empty());
    assert_eq!(
        definitions.len(),
        1,
        "Unavailable headword metadata must not hide a definition"
    );
    let legacy: Data = serde_json::from_value(
        json!({"type":"word","code":"G0025","definitions":[],"occurrences":[],"total":0}),
    )
    .unwrap();
    assert!(matches!(legacy, Data::Word { headwords, .. } if headwords.is_empty()));
}
#[test]
fn literal_search_filters_translation_and_pages_stably() {
    let f = Fixture::new();
    let mut e = f.engine();
    let Data::Search { hits, has_more, .. } = e
        .dispatch(Command::Search {
            query: "God".into(),
            limit: 1,
            offset: 0,
        })
        .unwrap()
        .data
    else {
        panic!()
    };
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "kjv16");
    assert!(!has_more);
    let Data::Search { hits, has_more, .. } = e
        .dispatch(Command::Search {
            query: "love".into(),
            limit: 1,
            offset: 0,
        })
        .unwrap()
        .data
    else {
        panic!()
    };
    assert_eq!(hits.len(), 1);
    assert!(has_more);
    for q in ["\" OR *", "NEAR(foo)", "' OR 1=1 --"] {
        let result = e.dispatch(Command::Search {
            query: q.into(),
            limit: 10,
            offset: 0,
        });
        assert!(result.is_ok(), "{q}: {result:?}");
    }
}
#[test]
fn transports_share_dispatch_and_reject_invalid_arguments() {
    let f = Fixture::new();
    let mut e = f.engine();
    let catalog = command_catalog();
    assert_eq!(
        catalog,
        serde_json::from_str::<Value>(include_str!("../../../specs/app/commands-v1.json")).unwrap()
    );
    assert_eq!(catalog["commands"].as_array().unwrap().len(), 13);
    for value in [
        json!({"command":"reading.select","params":{"ref_id":"wrong"}}),
        json!({"command":"workspace.get","params":{"unexpected":true}}),
        json!({"command":"execute.sql","params":{}}),
    ] {
        assert_eq!(control_request(&mut e, value)["ok"], false);
    }
    let call=mcp_request(&mut e,json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"reading.navigate","arguments":{"reference":"Rom 1:3"}}})).unwrap();
    assert_eq!(call["id"], 7);
    assert_eq!(
        call["result"]["structuredContent"]["state"]["session"]["selected_ref"],
        45001003
    );
    assert!(
        mcp_request(
            &mut e,
            json!({"jsonrpc":"2.0","method":"notifications/initialized"})
        )
        .is_none()
    );
    assert_eq!(
        mcp_request(&mut e, json!({"jsonrpc":"2.0","id":9,"method":"missing"})).unwrap()["error"]["code"],
        -32601
    );
    let mut out = Vec::new();
    serve(
        &mut e,
        &b"{broken}\n{\"command\":\"workspace.get\",\"params\":{}}\n"[..],
        &mut out,
        false,
    )
    .unwrap();
    assert_eq!(String::from_utf8(out).unwrap().lines().count(), 2);
    assert!(
        serve(
            &mut e,
            &vec![b'x'; MAX_REQUEST_BYTES + 1][..],
            Vec::new(),
            false
        )
        .is_err()
    );
    let lines=[
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"reading.navigate","arguments":{"reference":"John 3:16"}}}),
    ].map(|v|v.to_string()).join("\n")+"\n";
    let mut out = Vec::new();
    serve(&mut e, lines.as_bytes(), &mut out, true).unwrap();
    let replies = String::from_utf8(out)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(replies.len(), 3);
    assert_eq!(replies[0]["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(replies[1]["result"]["tools"].as_array().unwrap().len(), 13);
    assert_eq!(
        replies[2]["result"]["structuredContent"]["state"]["session"]["selected_ref"],
        43003016
    );
}
#[test]
fn content_and_user_identity_checks_prevent_wrong_file_writes() {
    let f = Fixture::new();
    let path = f.dir.path().join("content.sqlite");
    assert!(Engine::open(&path, &path).is_err());
    let wrong = f.dir.path().join("wrong.sqlite");
    let db = Connection::open(&wrong).unwrap();
    db.execute("CREATE TABLE personal_data(value TEXT)", [])
        .unwrap();
    drop(db);
    assert!(Engine::open(&path, &wrong).is_err());
    let db = Connection::open(&wrong).unwrap();
    assert_eq!(
        db.query_row("PRAGMA application_id", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    let user = f.dir.path().join("user.sqlite");
    let db = Connection::open(&path).unwrap();
    db.execute("DELETE FROM canonical_refs WHERE id=1001001", [])
        .unwrap();
    drop(db);
    assert!(Engine::open(&path, &user).is_err());
    assert!(!user.exists());
}

#[test]
fn word_links_use_explicit_edition_specific_unicode_spans_only() {
    let f = Fixture::new();
    let db = Connection::open(f.dir.path().join("content.sqlite")).unwrap();
    db.execute(
        "UPDATE verse_texts SET text='ἦ God loved.' WHERE id='kjv16'",
        [],
    )
    .unwrap();
    db.execute("INSERT INTO word_alignments VALUES('reviewed-link','kjv16','token','source',6,11,'reviewed')", []).unwrap();
    db.execute_batch("INSERT INTO original_tokens VALUES('foreign-token','web-witness','merged','δοκιμή',1,NULL); INSERT INTO token_analyses VALUES('foreign-token',0,NULL,NULL,NULL); INSERT INTO token_strongs VALUES('foreign-token',0,'G0025'); INSERT INTO word_alignments VALUES('bad-canonical-link','kjv16','foreign-token','source',2,5,'source');").unwrap();
    let mut e = f.engine();
    let verse = e.workspace().unwrap().passage.remove(0);
    assert_eq!(
        verse.word_links.len(),
        1,
        "Alternative source analyses must not duplicate the same code"
    );
    let link = &verse.word_links[0];
    assert_eq!(
        verse
            .text
            .chars()
            .skip(link.start)
            .take(link.end - link.start)
            .collect::<String>(),
        "loved"
    );
    assert_eq!(link.strongs, "G0025");
    assert_eq!(link.method, "reviewed");
    assert!(
        e.dispatch(Command::Translation { id: "web".into() })
            .unwrap()
            .state
            .passage
            .iter()
            .all(|v| v.word_links.is_empty()),
        "Shared verse/token data cannot establish an English alignment"
    );
    e.dispatch(Command::Translation { id: "kjv".into() })
        .unwrap();
    drop(e);
    db.execute("UPDATE word_alignments SET word_end=200", [])
        .unwrap();
    assert!(f.engine().workspace().is_err());
}

#[test]
fn cross_reference_previews_use_active_edition_and_preserve_targets() {
    let f = Fixture::new();
    let db = Connection::open(f.dir.path().join("content.sqlite")).unwrap();
    db.execute("INSERT INTO resource_links VALUES('known-target','tsk','john16',NULL,NULL,'cross_reference')", []).unwrap();
    let mut e = f.engine();
    for (edition, expected) in [
        ("kjv", "For God so loved the world."),
        ("web", "For God so loved the world, WEB."),
    ] {
        e.dispatch(Command::Translation { id: edition.into() })
            .unwrap();
        let Data::Context {
            cross_references, ..
        } = e
            .dispatch(Command::Context { ref_id: 43003016 })
            .unwrap()
            .data
        else {
            panic!()
        };
        assert_eq!(cross_references.len(), 2);
        let target = cross_references
            .iter()
            .find(|r| r.start_ref == 43003016)
            .unwrap();
        assert_eq!(target.preview.as_deref(), Some(expected));
        assert_eq!(target.resource_id, "tsk");
        assert!(
            cross_references
                .iter()
                .any(|r| r.start_ref == 45001003 && r.preview.is_none())
        );
    }
}

#[test]
fn legacy_mhc_entity_display_repair_is_source_scoped_and_read_only() {
    let f = Fixture::new();
    let db = Connection::open(f.dir.path().join("content.sqlite")).unwrap();
    db.execute_batch("INSERT INTO source_snapshots VALUES('legacy-mhc','matthew-henry','hash','https://example.org',NULL,'review','adapter'); UPDATE resources SET source_snapshot_id='legacy-mhc',content='Mark
Roma &#224; fratribus.
I. Concerning this witness.' WHERE id='mh'; UPDATE resources SET content='Literal &#224; &amp; prose.' WHERE id='nave';").unwrap();
    let mut e = f.engine();
    let Data::Resource { resource } = e
        .dispatch(Command::Resource { id: "mh".into() })
        .unwrap()
        .data
    else {
        panic!()
    };
    assert_eq!(
        resource.content,
        "Mark\nRoma à fratribus.\nI. Concerning this witness."
    );
    let stored: String = db
        .query_row("SELECT content FROM resources WHERE id='mh'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(
        stored.contains("&#224;"),
        "Opening an article must preserve the original database bytes"
    );
    let Data::Resource { resource } = e
        .dispatch(Command::Resource { id: "nave".into() })
        .unwrap()
        .data
    else {
        panic!()
    };
    assert_eq!(resource.content, "Literal &#224; &amp; prose.");
    db.execute("UPDATE resources SET content='An intentional literal: &#224;', display_json=?1 WHERE id='mh'", [r#"{"text_normalization":"mhc-numeric-v1"}"#]).unwrap();
    let Data::Resource { resource } = e
        .dispatch(Command::Resource { id: "mh".into() })
        .unwrap()
        .data
    else {
        panic!()
    };
    assert_eq!(
        resource.content, "An intentional literal: &#224;",
        "New normalized content must not receive a second repair"
    );
}

#[test]
fn source_dictionary_tags_are_edition_bound_and_never_claim_original_token_identity() {
    let f = Fixture::new();
    let db = Connection::open(f.dir.path().join("content.sqlite")).unwrap();
    let text = "Élan loved loved.";
    let tags = fontes_data_contracts::VerseStrongs {
        version: 1,
        verse_text_id: "kjv16".into(),
        source_snapshot_id: "source".into(),
        text_sha256: hex::encode(
            ring::digest::digest(&ring::digest::SHA256, text.as_bytes()).as_ref(),
        ),
        spans: vec![fontes_data_contracts::StrongsSpan {
            start: 5,
            end: 10,
            codes: vec!["G0025".into()],
        }],
    };
    let metadata = json!({"type":"verse_annotations","source_strongs":tags});
    db.execute(
        "UPDATE verse_texts SET text=?1,display_json=?2 WHERE id='kjv16'",
        params![text, metadata.to_string()],
    )
    .unwrap();
    let mut engine = f.engine();
    let state = engine.workspace().unwrap();
    let verse = state.passage.iter().find(|v| v.id == "kjv16").unwrap();
    assert!(
        verse.word_links.is_empty(),
        "Dictionary tags cannot invent token alignments"
    );
    assert_eq!(verse.strongs_links.len(), 1);
    assert_eq!(
        verse
            .text
            .chars()
            .skip(verse.strongs_links[0].start)
            .take(verse.strongs_links[0].end - verse.strongs_links[0].start)
            .collect::<String>(),
        "loved"
    );
    assert_eq!(
        verse.strongs_links[0].end, 10,
        "The repeated second word must remain untagged"
    );
    assert!(
        engine
            .dispatch(Command::Translation { id: "web".into() })
            .unwrap()
            .state
            .passage
            .iter()
            .all(|v| v.strongs_links.is_empty())
    );
    drop(engine);
    for mutation in 0..5 {
        let mut metadata = metadata.clone();
        match mutation {
            0 => metadata["source_strongs"]["text_sha256"] = "wrong text".into(),
            1 => metadata["source_strongs"]["spans"][0]["end"] = 999.into(),
            2 => metadata["source_strongs"]["verse_text_id"] = "web16".into(),
            3 => metadata["source_strongs"]["source_snapshot_id"] = "missing".into(),
            _ => metadata["source_strongs"]["spans"][0]["codes"][0] = "G+025".into(),
        }
        db.execute(
            "UPDATE verse_texts SET display_json=?1 WHERE id='kjv16'",
            [metadata.to_string()],
        )
        .unwrap();
        let mut engine = f.engine();
        let result = engine.dispatch(Command::Translation { id: "kjv".into() });
        assert!(
            result.is_err(),
            "Invalid source metadata must fail closed: {mutation}"
        );
    }
}

#[test]
fn reading_themes_persist_and_legacy_preferences_default_without_touching_notes() {
    let f = Fixture::new();
    let old =
        json!({"text_size":22.0,"line_spacing":1.65,"verse_numbers":true,"scripture_serif":true});
    let legacy: Preferences = serde_json::from_value(old.clone()).unwrap();
    assert_eq!(legacy.theme, ReadingTheme::SeaGlass);
    let mut invalid = old.clone();
    invalid["theme"] = json!("not-a-theme");
    assert!(serde_json::from_value::<Preferences>(invalid).is_err());
    let mut e = f.engine();
    e.dispatch(Command::SaveAnnotation {
        id: None,
        start_ref: 43003016,
        end_ref: 43003016,
        kind: AnnotationKind::Note,
        color: None,
        content: Some("Keep this note".into()),
    })
    .unwrap();
    for theme in ReadingTheme::ALL {
        let mut prefs = e.workspace().unwrap().session.preferences;
        prefs.theme = theme;
        e.dispatch(Command::Preferences { preferences: prefs })
            .unwrap();
        drop(e);
        e = f.engine();
        let state = e.workspace().unwrap();
        assert_eq!(state.session.preferences.theme, theme);
        assert_eq!(state.session.selected_ref, 43003016);
        assert_eq!(
            state.annotations[0].content.as_deref(),
            Some("Keep this note")
        );
    }
    drop(e);
    // Simulate the actual pre-theme session representation on disk.
    let db = Connection::open(f.dir.path().join("user.sqlite")).unwrap();
    let raw: String = db
        .query_row("SELECT json FROM fontes_app_session_v1", [], |r| r.get(0))
        .unwrap();
    let mut session: Value = serde_json::from_str(&raw).unwrap();
    session["preferences"]
        .as_object_mut()
        .unwrap()
        .remove("theme");
    db.execute(
        "UPDATE fontes_app_session_v1 SET json=?1",
        [session.to_string()],
    )
    .unwrap();
    drop(db);
    assert_eq!(
        f.engine().workspace().unwrap().session.preferences.theme,
        ReadingTheme::SeaGlass
    );
}
