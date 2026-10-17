//! Pinned OSHB words and explicit MT → canonical verse mapping. No English alignment.
use crate::{
    Registry,
    study::{StudyImport, strong_code},
};
use anyhow::{Context, Result, ensure};
use rusqlite::{Transaction, params};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::Path,
};

pub const WITNESS: &str = "witness:oshb-wlc";
pub const ATTRIBUTION: &str = "Original work of the Open Scriptures Hebrew Bible available at https://github.com/openscriptures/morphhb";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mapping {
    pub mapping_version: u32,
    pub source_sha256: String,
    pub review: String,
    pub partial_word_boundaries: BTreeMap<String, Vec<(usize, Option<usize>, String)>>,
    pub psalm_superscriptions: BTreeSet<String>,
    pub source_omissions: BTreeSet<String>,
    pub expected_canonical_verses: usize,
}

fn member(archive: &mut zip::ZipArchive<fs::File>, suffix: &str, limit: u64) -> Result<String> {
    let names: Vec<_> = archive
        .file_names()
        .collect::<std::result::Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|n| n.ends_with(suffix))
        .map(|s| s.into_owned())
        .collect();
    ensure!(
        names.len() == 1,
        "missing or duplicate OSHB member {suffix}"
    );
    let entry = archive.by_name(&names[0])?;
    ensure!(entry.size() <= limit, "OSHB member exceeds limit: {suffix}");
    let mut text = String::new();
    entry.take(limit + 1).read_to_string(&mut text)?;
    ensure!(text.len() as u64 <= limit, "OSHB member exceeds limit");
    Ok(text)
}

/// OSHB's lemma attribute uses augmented Strong's numbers and prefix codes;
/// it is not a dictionary headword. Keep only explicit numeric code components.
pub fn strongs(lemma: &str) -> Result<BTreeSet<String>> {
    let mut result = BTreeSet::new();
    for component in lemma.split('/') {
        let number: String = component.chars().take_while(char::is_ascii_digit).collect();
        if number.is_empty() {
            ensure!(
                !component.is_empty() && component.chars().all(|c| c.is_ascii_lowercase()),
                "invalid OSHB prefix {component}"
            );
        } else {
            let suffix = &component[number.len()..];
            ensure!(
                suffix.is_empty()
                    || suffix == "+"
                    || (suffix.starts_with(' ')
                        && suffix[1..].chars().all(|c| c.is_ascii_lowercase())),
                "invalid OSHB lexeme suffix {component}"
            );
            result.insert(strong_code('H', &number)?);
        }
    }
    Ok(result)
}

pub fn import(
    tx: &Transaction<'_>,
    snapshot: &str,
    path: &Path,
    registry: &Registry,
    mapping: &Mapping,
    normalized: &Path,
) -> Result<StudyImport> {
    ensure!(
        mapping.mapping_version == 1 && !mapping.review.is_empty(),
        "invalid OSHB mapping review"
    );
    ensure!(
        crate::file_digest(path)? == mapping.source_sha256,
        "OSHB mapping is bound to different source bytes"
    );
    let mut archive = zip::ZipArchive::new(fs::File::open(path)?)?;
    ensure!(archive.len() < 4000, "OSHB archive has too many members");
    let xml = member(&mut archive, "/wlc/VerseMap.xml", 512 * 1024)?;
    let doc = roxmltree::Document::parse(&xml)?;
    let mut full = BTreeMap::new();
    let mut partial_sources = BTreeSet::new();
    for row in doc.descendants().filter(|n| n.has_tag_name("verse")) {
        let source = row.attribute("wlc").context("OSHB map source missing")?;
        let target = row.attribute("kjv").context("OSHB map target missing")?;
        match row.attribute("type") {
            Some("full") => {
                ensure!(
                    full.insert(source.to_owned(), target.to_owned()).is_none(),
                    "duplicate OSHB full map"
                );
            }
            Some("partial") => {
                partial_sources.insert(source.split('!').next().unwrap().to_owned());
            }
            _ => anyhow::bail!("unsupported OSHB mapping type"),
        }
    }
    ensure!(
        partial_sources == mapping.partial_word_boundaries.keys().cloned().collect(),
        "unreviewed OSHB partial mappings"
    );
    tx.execute("INSERT INTO text_witnesses VALUES (?1,'Open Scriptures Hebrew Bible (WLC)','he','Pinned OSHB WLC with supplied Strong’s and OSHM','MT; explicit canonical remapping',?2)",params![WITNESS,snapshot])?;
    let mut report = StudyImport {
        source_id: "oshb-hebrew".into(),
        ..Default::default()
    };
    let mut normalized = std::io::BufWriter::new(fs::File::create(normalized)?);
    let mut covered = BTreeSet::new();
    let mut headings = BTreeSet::new();
    let mut partials = BTreeSet::new();
    let mut positions = BTreeMap::<i64, usize>::new();
    for book in registry.books.iter().filter(|b| b.testament == "OT") {
        let xml = member(
            &mut archive,
            &format!("/wlc/{}.xml", book.osis),
            8 * 1024 * 1024,
        )?;
        let doc = roxmltree::Document::parse_with_options(
            &xml,
            roxmltree::ParsingOptions {
                nodes_limit: 1_000_000,
                ..Default::default()
            },
        )?;
        for verse in doc.descendants().filter(|n| n.has_tag_name("verse")) {
            let reference = verse
                .attribute("osisID")
                .context("OSHB verse reference missing")?;
            ensure!(
                reference.starts_with(&format!("{}.", book.osis)),
                "OSHB book/reference mismatch"
            );
            let words: Vec<_> = verse.children().filter(|n| n.has_tag_name("w")).collect();
            // Qere/variant words inside notes stay in the retained source, not
            // silently mixed into the primary reading's positions.
            let targets = if let Some(parts) = mapping.partial_word_boundaries.get(reference) {
                partials.insert(reference.to_owned());
                parts.clone()
            } else if mapping.psalm_superscriptions.contains(reference) {
                headings.insert(reference.to_owned());
                vec![(
                    0,
                    None,
                    format!(
                        "{}.1",
                        reference
                            .rsplit_once('.')
                            .context("invalid Psalm reference")?
                            .0
                    ),
                )]
            } else {
                vec![(
                    0,
                    None,
                    full.get(reference)
                        .cloned()
                        .unwrap_or_else(|| reference.to_owned()),
                )]
            };
            let mut cursor = 0;
            for (start, end, target) in targets {
                let end = end.unwrap_or(words.len());
                ensure!(
                    start == cursor && start < end && end <= words.len(),
                    "invalid OSHB partial boundary {reference}"
                );
                cursor = end;
                let canonical = if target.is_empty() {
                    None
                } else {
                    Some(crate::study::range_for(registry, &target, false)?)
                };
                let segment = format!("segment:oshb:{reference}:{start}-{end}");
                tx.prepare_cached("INSERT INTO source_segments VALUES (?1,?2,?3,?4,?5)")?
                    .execute(params![
                        segment,
                        snapshot,
                        WITNESS,
                        reference,
                        format!("{reference}:{start}-{end}")
                    ])?;
                if let Some(id) = canonical {
                    covered.insert(id);
                    tx.prepare_cached("INSERT INTO segment_refs VALUES (?1,?2,?3)")?
                        .execute(params![
                            segment,
                            id,
                            if start == 0 && end == words.len() && !reference.starts_with("Ps.") {
                                "exact"
                            } else {
                                "part"
                            }
                        ])?;
                }
                for (offset, word) in words[start..end].iter().enumerate() {
                    let source_id = word
                        .attribute("id")
                        .context("OSHB stable word ID missing")?;
                    ensure!(
                        source_id.len() == 5
                            && source_id.chars().all(|c| c.is_ascii_alphanumeric()),
                        "invalid OSHB word ID"
                    );
                    let surface: String = word
                        .descendants()
                        .filter(|n| n.is_text())
                        .filter_map(|n| n.text())
                        .collect();
                    ensure!(!surface.is_empty(), "empty OSHB word");
                    let lemma = word
                        .attribute("lemma")
                        .context("OSHB lexeme codes missing")?;
                    let morph = word.attribute("morph").context("OSHB morphology missing")?;
                    ensure!(
                        morph.starts_with('H') || morph.starts_with('A'),
                        "invalid OSHB morphology language"
                    );
                    let codes =
                        strongs(lemma).with_context(|| format!("{reference}:{source_id}"))?;
                    let position = canonical.map(|id| positions.entry(id).or_default());
                    let index = position.as_deref().copied().unwrap_or(start + offset);
                    if let Some(position) = position {
                        *position += 1;
                    }
                    let token = format!("token:oshb:{source_id}");
                    tx.prepare_cached("INSERT INTO original_tokens VALUES (?1,?2,?3,?4,?5,NULL)")?
                        .execute(params![token, WITNESS, segment, surface, index])?;
                    tx.prepare_cached("INSERT INTO token_analyses VALUES (?1,0,NULL,'oshm',?2)")?
                        .execute(params![token, morph])?;
                    for code in &codes {
                        tx.prepare_cached("INSERT INTO token_strongs VALUES (?1,0,?2)")?
                            .execute(params![token, code])?;
                    }
                    serde_json::to_writer(
                        &mut normalized,
                        &serde_json::json!({"source_id":source_id,"source_ref":reference,"canonical_ref":canonical,"surface":surface,"lexeme_codes":lemma,"morphology":morph,"strongs":codes}),
                    )?;
                    normalized.write_all(b"\n")?;
                }
            }
            ensure!(cursor == words.len(), "OSHB words lost at {reference}");
            report.source_records += 1;
        }
    }
    normalized.flush()?;
    ensure!(
        headings == mapping.psalm_superscriptions
            && partials == mapping.partial_word_boundaries.keys().cloned().collect(),
        "stale OSHB mapping review"
    );
    let mut expected = BTreeSet::new();
    for book in registry.books.iter().filter(|b| b.testament == "OT") {
        for (chapter, count) in book.verse_counts.iter().enumerate() {
            for verse in 1..=*count {
                let reference = format!("{}.{}.{}", book.osis, chapter + 1, verse);
                if !mapping.source_omissions.contains(&reference) {
                    expected.insert(crate::study::range_for(registry, &reference, false)?);
                }
            }
        }
    }
    ensure!(
        expected.len() == mapping.expected_canonical_verses - mapping.source_omissions.len()
            && covered == expected,
        "OSHB canonical coverage mismatch"
    );
    let license = member(&mut archive, "/LICENSE.md", 64 * 1024)?;
    ensure!(
        license.contains(ATTRIBUTION) && license.contains("Attribution 4.0"),
        "OSHB license changed"
    );
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lexeme_components_preserve_explicit_codes_without_inventing_prefix_entries() {
        assert_eq!(
            strongs("c/m/6529").unwrap(),
            BTreeSet::from(["H6529".to_owned()])
        );
        assert_eq!(
            strongs("5921 a").unwrap(),
            BTreeSet::from(["H5921".to_owned()])
        );
        assert!(strongs("l").unwrap().is_empty());
        assert!(strongs("8675").is_err());
        assert!(strongs("m/430unexpected").is_err());
    }
}
