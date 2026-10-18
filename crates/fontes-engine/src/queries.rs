use crate::*;
use rusqlite::{OptionalExtension, params};
impl Engine {
    pub fn workspace(&self) -> Result<Workspace> {
        let lower = self.session.selected_ref / 1000 * 1000 + 1;
        let upper = lower + 998;
        // Each source segment appears once even when mapped to multiple canonical anchors.
        let mut passage = Vec::<Verse>::new();
        let mut indices = std::collections::HashMap::<String, usize>::new();
        let mut stmt=self.content.prepare("SELECT id,canonical_ref_id,text,display_json FROM verses_by_reference WHERE translation_id=?1 AND canonical_ref_id BETWEEN ?2 AND ?3 ORDER BY canonical_ref_id,id")?;
        for row in stmt.query_map(params![self.session.translation, lower, upper], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        })? {
            let (id, reference, text, display) = row?;
            if let Some(i) = indices.get(&id) {
                passage[*i].refs.push(reference);
            } else {
                let mut strongs_links = Vec::new();
                if let Some(display) = display {
                    let display: serde_json::Value = serde_json::from_str(&display)
                        .map_err(|_| Error::invalid("Invalid verse display metadata."))?;
                    if let Some(tags) = display.get("source_strongs") {
                        let tags: fontes_data_contracts::VerseStrongs =
                            serde_json::from_value(tags.clone())
                                .map_err(|_| Error::invalid("Invalid source word tags."))?;
                        let hash = hex::encode(
                            ring::digest::digest(&ring::digest::SHA256, text.as_bytes()).as_ref(),
                        );
                        tags.validate(&id, &text, &hash).map_err(Error::invalid)?;
                        let source_exists: bool = self.content.query_row(
                            "SELECT EXISTS(SELECT 1 FROM source_snapshots WHERE id=?1)",
                            [&tags.source_snapshot_id],
                            |r| r.get(0),
                        )?;
                        if !source_exists {
                            return Err(Error::invalid(
                                "Source word tags reference a missing source.",
                            ));
                        }
                        for span in tags.spans {
                            for code in span.codes {
                                let present: bool = self.content.query_row(
                                    "SELECT EXISTS(SELECT 1 FROM strongs_codes WHERE id=?1)",
                                    [&code],
                                    |r| r.get(0),
                                )?;
                                if !present {
                                    return Err(Error::invalid(
                                        "Source word tags reference a missing dictionary code.",
                                    ));
                                }
                                strongs_links.push(StrongsLink {
                                    start: span.start,
                                    end: span.end,
                                    strongs: code,
                                    source_snapshot_id: tags.source_snapshot_id.clone(),
                                });
                            }
                        }
                    }
                }
                indices.insert(id.clone(), passage.len());
                passage.push(Verse {
                    id,
                    refs: vec![reference],
                    text,
                    word_links: vec![],
                    strongs_links,
                });
            }
        }
        // Use only explicit alignment records, never a shared Strong's number or word order.
        let mut links = self.content.prepare("SELECT DISTINCT wa.verse_text_id,wa.word_start,wa.word_end,t.surface,t.witness_id,ts.strongs,wa.alignment_method FROM word_alignments wa JOIN verse_texts v ON v.id=wa.verse_text_id JOIN segment_refs sr ON sr.segment_id=v.segment_id JOIN original_tokens t ON t.id=wa.original_token_id JOIN token_strongs ts ON ts.token_id=t.id WHERE v.translation_id=?1 AND sr.canonical_ref_id BETWEEN ?2 AND ?3 AND EXISTS (SELECT 1 FROM segment_refs tr WHERE tr.segment_id=t.segment_id AND tr.canonical_ref_id=sr.canonical_ref_id) ORDER BY wa.verse_text_id,wa.word_start,wa.word_end,t.witness_id,ts.strongs")?;
        for row in links.query_map(params![self.session.translation, lower, upper], |r| {
            Ok((
                r.get::<_, String>(0)?,
                WordLink {
                    start: r.get(1)?,
                    end: r.get(2)?,
                    surface: r.get(3)?,
                    witness: r.get(4)?,
                    strongs: r.get(5)?,
                    method: r.get(6)?,
                },
            ))
        })? {
            let (id, link) = row?;
            if let Some(i) = indices.get(&id) {
                let verse = &mut passage[*i];
                if link.start >= link.end || link.end > verse.text.chars().count() {
                    return Err(Error::invalid(
                        "An installed word alignment is outside its verse text.",
                    ));
                }
                verse.word_links.push(link);
            }
        }
        let encoded=self.user.prepare("SELECT annotation_id,style_json,selection_json FROM fontes_highlights_v1 ORDER BY annotation_id")?.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?)))?.collect::<std::result::Result<Vec<_>,_>>()?;
        let highlights = encoded
            .into_iter()
            .map(|(annotation_id, style, selection)| {
                Ok(crate::RichHighlight {
                    annotation_id,
                    style: serde_json::from_str(&style)?,
                    selection: selection.map(|s| serde_json::from_str(&s)).transpose()?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let annotations=self.user.prepare("SELECT a.id,r.start_ref_id,r.end_ref_id,a.type,a.color,a.content,a.updated_at FROM annotations a JOIN reference_ranges r ON r.id=a.range_id ORDER BY a.updated_at DESC,a.id")?.query_map([],|r|Ok(Annotation{id:r.get(0)?,start_ref:r.get(1)?,end_ref:r.get(2)?,kind:if r.get::<_,String>(3)?=="note"{AnnotationKind::Note}else{AnnotationKind::Highlight},color:r.get(4)?,content:r.get(5)?,updated_at:r.get(6)?}))?.collect::<std::result::Result<Vec<_>,_>>()?;
        let saved=self.user.prepare("SELECT r.start_ref_id,s.type FROM saved_references s JOIN reference_ranges r ON r.id=s.range_id ORDER BY s.type,s.last_used_at DESC,s.id")?.query_map([],|r|{let id=r.get(0)?;Ok(SavedReference{ref_id:id,label:label(&self.books,id),kind:r.get(1)?})})?.collect::<std::result::Result<Vec<_>,_>>()?;
        let notes = self
            .user
            .prepare(
                "SELECT annotation_id,document_json FROM fontes_notes_v1 ORDER BY annotation_id",
            )?
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?
            .into_iter()
            .map(|(annotation_id, json)| {
                Ok(crate::RichNote {
                    annotation_id,
                    document: serde_json::from_str(&json)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Workspace {
            content_id: self.content_id.clone(),
            profile: self.profile.clone(),
            session: self.session.clone(),
            books: self.books.clone(),
            translations: self.translations.clone(),
            passage,
            annotations,
            highlights,
            notes,
            saved,
            has_study: self.has_study,
        })
    }
    pub(crate) fn search(&self, query: &str, limit: u32, offset: u32) -> Result<Data> {
        if query.trim().is_empty()
            || query.len() > 512
            || !(1..=100).contains(&limit)
            || offset > 10_000
        {
            return Err(Error::invalid(
                "Enter 1–512 bytes of search text; limit 1–100, offset at most 10000.",
            ));
        }
        // Literal AND terms, never executable FTS syntax. Quotes/operators in input remain literal.
        let terms = query
            .split_whitespace()
            .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" AND ");
        let mut stmt=self.content.prepare("SELECT COALESCE(v.id,r.id), (SELECT min(sr.canonical_ref_id) FROM segment_refs sr WHERE sr.segment_id=v.segment_id), COALESCE(r.title,''), snippet(search_fts,0,'','',' … ',32) FROM search_fts JOIN search_documents d ON d.id=search_fts.rowid LEFT JOIN verse_texts v ON v.id=d.verse_text_id LEFT JOIN resources r ON r.id=d.resource_id WHERE search_fts MATCH ?1 AND (v.translation_id=?2 OR r.id IS NOT NULL) ORDER BY rank,d.id LIMIT ?3 OFFSET ?4")?;
        let mut hits = stmt
            .query_map(
                params![terms, self.session.translation, limit + 1, offset],
                |r| {
                    let reference = r.get::<_, Option<i64>>(1)?;
                    Ok(SearchHit {
                        id: r.get(0)?,
                        ref_id: reference,
                        title: reference
                            .map(|id| label(&self.books, id))
                            .unwrap_or(r.get(2)?),
                        excerpt: r.get(3)?,
                    })
                },
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let has_more = hits.len() > limit as usize;
        hits.truncate(limit as usize);
        Ok(Data::Search {
            hits,
            offset,
            has_more,
        })
    }
    pub(crate) fn context(&self, reference: i64) -> Result<Data> {
        let resources=self.content.prepare("SELECT r.id,r.title,r.author,r.type,l.link_type,substr(r.content,1,260) FROM resources r JOIN resource_links l ON l.resource_id=r.id JOIN reference_ranges rr ON rr.id=l.range_id WHERE rr.start_ref_id<=?1 AND rr.end_ref_id>=?1 GROUP BY r.id,l.link_type ORDER BY CASE l.link_type WHEN 'applies_to' THEN 0 ELSE 1 END,min(rr.end_ref_id-rr.start_ref_id),r.type,r.title,r.id LIMIT 100")?.query_map([reference],summary)?.collect::<std::result::Result<Vec<_>,_>>()?;
        let tokens=self.content.prepare("SELECT t.witness_id,t.position,t.surface,a.analysis_index,l.lemma,a.morph_system,a.morph_code,ts.strongs FROM original_tokens t JOIN segment_refs sr ON sr.segment_id=t.segment_id JOIN token_analyses a ON a.token_id=t.id LEFT JOIN lemmas l ON l.id=a.lemma_id LEFT JOIN token_strongs ts ON ts.token_id=a.token_id AND ts.analysis_index=a.analysis_index WHERE sr.canonical_ref_id=?1 ORDER BY t.witness_id,t.position,a.analysis_index,ts.strongs LIMIT 2000")?.query_map([reference],|r|Ok(TokenAnalysis{witness:r.get(0)?,position:r.get(1)?,surface:r.get(2)?,analysis_index:r.get(3)?,lemma:r.get(4)?,morph_system:r.get(5)?,morph_code:r.get(6)?,strongs:r.get(7)?}))?.collect::<std::result::Result<Vec<_>,_>>()?;
        let cross_references = self.content.prepare("SELECT target_range.start_ref_id,target_range.end_ref_id,min(target.resource_id),(SELECT text FROM verses_by_reference v WHERE v.canonical_ref_id=target_range.start_ref_id AND v.translation_id=?2 ORDER BY v.id LIMIT 1) FROM resource_links origin JOIN reference_ranges coverage ON coverage.id=origin.range_id JOIN resources r ON r.id=origin.resource_id JOIN resource_links target ON target.resource_id=r.id JOIN reference_ranges target_range ON target_range.id=target.range_id WHERE r.type='cross_reference' AND origin.link_type='applies_to' AND coverage.start_ref_id<=?1 AND coverage.end_ref_id>=?1 AND target.link_type='cross_reference' GROUP BY target_range.start_ref_id,target_range.end_ref_id ORDER BY min(coverage.end_ref_id-coverage.start_ref_id),target_range.start_ref_id,target_range.end_ref_id LIMIT 100")?.query_map(params![reference, self.session.translation], |r| {
            let start_ref = r.get(0)?;
            let end_ref = r.get(1)?;
            let title = if start_ref == end_ref { label(&self.books, start_ref) } else { format!("{} – {}", label(&self.books, start_ref), label(&self.books, end_ref)) };
            Ok(CrossReference { start_ref, end_ref, label: title, resource_id: r.get(2)?, preview: r.get(3)? })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(Data::Context {
            ref_id: reference,
            resources,
            tokens,
            cross_references,
        })
    }
    pub(crate) fn resource(&self, id: &str) -> Result<ResourceDetail> {
        let (mut resource, display)=self.content.query_row("SELECT r.id,r.title,r.author,r.content,s.source_id,s.rights_review_id,r.display_json FROM resources r JOIN source_snapshots s ON s.id=r.source_snapshot_id WHERE r.id=?1",[id],|r|Ok((ResourceDetail{id:r.get(0)?,title:r.get(1)?,author:r.get(2)?,content:r.get(3)?,source:r.get(4)?,rights_review:r.get(5)?,links:vec![]},r.get::<_,Option<String>>(6)?))).optional()?.ok_or_else(||Error::missing("This resource is not installed."))?;
        // Compatibility display repair for previously compiled MHC snapshots;
        // the read-only database and its signed bytes are never rewritten.
        let normalized = display
            .as_deref()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .is_some_and(|v| v["text_normalization"] == "mhc-numeric-v1");
        if resource.source == "matthew-henry" && !normalized {
            resource.content =
                fontes_data_contracts::decode_numeric_text_entities(&resource.content);
        }
        resource.links=self.content.prepare("SELECT rr.start_ref_id,rr.end_ref_id,l.strongs,l.link_type FROM resource_links l LEFT JOIN reference_ranges rr ON rr.id=l.range_id WHERE l.resource_id=?1 ORDER BY l.link_type,rr.start_ref_id,l.strongs LIMIT 500")?.query_map([id],|r|Ok(ResourceLink{start_ref:r.get(0)?,end_ref:r.get(1)?,strongs:r.get(2)?,basis:r.get(3)?}))?.collect::<std::result::Result<Vec<_>,_>>()?;
        Ok(resource)
    }
    pub(crate) fn word(&self, input: &str) -> Result<Data> {
        let input = input.trim().to_uppercase();
        let code = match input.as_bytes().first() {
            Some(b'G' | b'H') => {
                let number = input[1..]
                    .parse::<u32>()
                    .ok()
                    .filter(|n| *n > 0 && *n <= 999_999)
                    .ok_or_else(|| Error::invalid("Use a Strong's code such as G25 or H7225."))?;
                format!("{}{number:04}", &input[..1])
            }
            _ => return Err(Error::invalid("Use a Strong's code such as G25 or H7225.")),
        };
        if !self.content.query_row(
            "SELECT EXISTS(SELECT 1 FROM strongs_codes WHERE id=?1)",
            [&code],
            |r| r.get::<_, bool>(0),
        )? {
            return Err(Error::missing(
                "This Strong's code is not present. Open a study profile for dictionary data.",
            ));
        }
        let definitions=self.content.prepare("SELECT r.id,r.title,r.author,r.type,l.link_type,substr(r.content,1,1500) FROM resources r JOIN resource_links l ON l.resource_id=r.id WHERE l.strongs=?1 AND l.link_type='definition' ORDER BY r.id")?.query_map([&code],summary)?.collect::<std::result::Result<Vec<_>,_>>()?;
        // Read source-supplied dictionary metadata, never infer a lemma from a title or definition.
        let metadata=self.content.prepare("SELECT DISTINCT r.id,r.display_json FROM resources r JOIN resource_links l ON l.resource_id=r.id WHERE l.strongs=?1 AND l.link_type='definition' ORDER BY r.id")?.query_map([&code],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Option<String>>(1)?)))?.collect::<std::result::Result<Vec<_>,_>>()?;
        let headwords = metadata
            .into_iter()
            .filter_map(|(id, display)| dictionary_headword(id, display))
            .collect();
        let total=self.content.query_row("SELECT count(DISTINCT sr.canonical_ref_id) FROM token_strongs ts JOIN original_tokens t ON t.id=ts.token_id JOIN segment_refs sr ON sr.segment_id=t.segment_id WHERE ts.strongs=?1",[&code],|r|r.get(0))?;
        let occurrences=self.content.prepare("SELECT DISTINCT sr.canonical_ref_id FROM token_strongs ts JOIN original_tokens t ON t.id=ts.token_id JOIN segment_refs sr ON sr.segment_id=t.segment_id WHERE ts.strongs=?1 ORDER BY sr.canonical_ref_id LIMIT 100")?.query_map([&code],|r|{let id=r.get(0)?;Ok(SavedReference{ref_id:id,label:label(&self.books,id),kind:"strongs_occurrence".into()})})?.collect::<std::result::Result<Vec<_>,_>>()?;
        Ok(Data::Word {
            code,
            headwords,
            definitions,
            occurrences,
            total,
        })
    }
}
fn summary(r: &rusqlite::Row<'_>) -> rusqlite::Result<ResourceSummary> {
    Ok(ResourceSummary {
        id: r.get(0)?,
        title: r.get(1)?,
        author: r.get(2)?,
        kind: r.get(3)?,
        basis: r.get(4)?,
        excerpt: r.get(5)?,
    })
}

fn dictionary_headword(resource_id: String, display: Option<String>) -> Option<DictionaryHeadword> {
    let metadata: serde_json::Value = serde_json::from_str(display.as_deref()?).ok()?;
    if metadata["type"] != "dictionary_entry" {
        return None;
    }
    let text = metadata["headword"].as_str()?;
    if text.trim().is_empty() {
        return None;
    }
    Some(DictionaryHeadword {
        resource_id,
        text: text.to_owned(),
        transliteration: metadata["transliteration"].as_str().map(str::to_owned),
    })
}

impl Engine {
    pub(crate) fn library_catalog(&self, source: Option<&str>, offset: u32) -> Result<Data> {
        if offset > 100000 || source.is_some_and(|s| s.len() > 512) {
            return Err(Error::invalid("Invalid library page."));
        }
        let sources=self.content.prepare("SELECT s.source_id,r.type,count(*),min(r.author) FROM resources r JOIN source_snapshots s ON s.id=r.source_snapshot_id GROUP BY s.source_id,r.type ORDER BY s.source_id,r.type")?.query_map([],|r|Ok(LibrarySource{id:r.get(0)?,kind:r.get(1)?,count:r.get(2)?,author:r.get(3)?}))?.collect::<std::result::Result<Vec<_>,_>>()?;
        let mut resources = if let Some(source) = source {
            self.content.prepare("SELECT r.id,r.title,r.author,r.type,'installed',substr(r.content,1,260) FROM resources r JOIN source_snapshots s ON s.id=r.source_snapshot_id WHERE s.source_id=?1 ORDER BY r.title,r.id LIMIT 51 OFFSET ?2")?.query_map(params![source,offset],summary)?.collect::<std::result::Result<Vec<_>,_>>()?
        } else {
            vec![]
        };
        let has_more = resources.len() > 50;
        resources.truncate(50);
        Ok(Data::Library {
            sources,
            resources,
            offset,
            has_more,
        })
    }
}
