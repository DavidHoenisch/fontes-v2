//! Source-provided KJV dictionary tags. Never join original words by Strong's code.
use crate::{
    Registry, digest,
    study::{StudyImport, strong_code},
};
use anyhow::{Context, Result, bail, ensure};
use fontes_data_contracts::{StrongsSpan, VerseStrongs};
use rusqlite::{Transaction, params};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::Path,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mapping {
    pub mapping_version: u32,
    pub source_sha256: String,
    pub review: String,
    pub text_mismatches: BTreeSet<String>,
}

#[derive(Default)]
struct Plain {
    chars: Vec<char>,
    pending_space: bool,
    spans: Vec<StrongsSpan>,
}
impl Plain {
    fn append(&mut self, text: &str) {
        for c in text.chars() {
            if c.is_whitespace() {
                self.pending_space = true;
            } else if c != '¶' {
                if self.pending_space && !self.chars.is_empty() {
                    self.chars.push(' ');
                }
                self.pending_space = false;
                self.chars.push(c);
            }
        }
    }
    fn word(&mut self, text: &str, codes: Vec<String>) -> Result<()> {
        ensure!(
            !text.trim().is_empty() && text.trim() == text && !text.contains('\\'),
            "invalid tagged USFM word"
        );
        let start = self.chars.len() + usize::from(self.pending_space && !self.chars.is_empty());
        self.append(text);
        ensure!(self.chars.len() > start, "empty tagged USFM surface");
        self.spans.push(StrongsSpan {
            start,
            end: self.chars.len(),
            codes,
        });
        Ok(())
    }
}

/// Preserve text inside supported inline styles, omit notes/section headings,
/// and retain explicit word boundaries. Unknown markup fails closed.
pub fn parse(input: &str) -> Result<(String, Vec<StrongsSpan>)> {
    let mut plain = Plain::default();
    let mut rest = input;
    let mut styles = Vec::new();
    while !rest.is_empty() {
        if !rest.starts_with('\\') {
            let end = rest.find('\\').unwrap_or(rest.len());
            plain.append(&rest[..end]);
            rest = &rest[end..];
            continue;
        }
        rest = &rest[1..];
        let end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '+' || c == '*'))
            .unwrap_or(rest.len());
        let marker = &rest[..end];
        rest = &rest[end..];
        if !marker.ends_with('*') {
            rest = rest.strip_prefix(' ').unwrap_or(rest);
        }
        match marker {
            "f" | "x" => {
                let close = format!("\\{marker}*");
                let end = rest.find(&close).context("unclosed USFM note")?;
                rest = &rest[end + close.len()..];
            }
            "s1" => {
                rest = rest.split_once('\n').map(|(_, s)| s).unwrap_or("");
                plain.pending_space = true;
            }
            "w" | "+w" => {
                let close = format!("\\{marker}*");
                let end = rest.find(&close).context("unclosed USFM word")?;
                let (surface, attributes) = rest[..end]
                    .split_once('|')
                    .context("word has no source tags")?;
                let codes = attributes
                    .trim()
                    .strip_prefix("strong=\"")
                    .and_then(|s| s.strip_suffix('"'))
                    .context("unsupported USFM word attributes")?;
                let codes: BTreeSet<_> = codes
                    .split_whitespace()
                    .map(|code| {
                        let prefix = code.chars().next().context("empty source code")?;
                        strong_code(
                            prefix,
                            code.get(prefix.len_utf8()..)
                                .context("invalid source code")?,
                        )
                    })
                    .collect::<Result<_>>()?;
                ensure!(!codes.is_empty(), "empty USFM word codes");
                plain.word(surface, codes.into_iter().collect())?;
                rest = &rest[end + close.len()..];
            }
            "add" | "+add" | "nd" | "+nd" | "wj" | "tl" => styles.push(marker),
            "add*" | "+add*" | "nd*" | "+nd*" | "wj*" | "tl*" => {
                ensure!(
                    styles.pop() == Some(&marker[..marker.len() - 1]),
                    "unbalanced USFM styles"
                );
            }
            "p" | "q1" | "b" | "d" => plain.pending_space = true,
            _ => bail!("unsupported USFM marker {marker}"),
        }
    }
    ensure!(styles.is_empty(), "unclosed USFM style");
    Ok((plain.chars.into_iter().collect(), plain.spans))
}

fn normalize_target(text: &str) -> (String, Vec<usize>) {
    let mut chars = Vec::new();
    let mut positions = Vec::new();
    let mut space = None;
    for (i, c) in text.chars().enumerate() {
        if c.is_whitespace() {
            space.get_or_insert(i);
        } else if c != '¶' {
            if let Some(i) = space.take().filter(|_| !chars.is_empty()) {
                chars.push(' ');
                positions.push(i);
            }
            chars.push(c);
            positions.push(i);
        }
    }
    (chars.into_iter().collect(), positions)
}

pub fn import(
    tx: &Transaction<'_>,
    snapshot: &str,
    path: &Path,
    registry: &Registry,
    mapping: &Mapping,
    output: &Path,
) -> Result<StudyImport> {
    ensure!(
        mapping.mapping_version == 1
            && !mapping.review.is_empty()
            && crate::file_digest(path)? == mapping.source_sha256,
        "source tags require a matching reviewed map"
    );
    let mut archive = zip::ZipArchive::new(fs::File::open(path)?)?;
    ensure!(archive.len() < 500, "too many tagged KJV members");
    let names: Vec<_> = archive
        .file_names()
        .collect::<std::result::Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|n| n.ends_with(".usfm"))
        .map(|n| n.into_owned())
        .collect();
    let mut output = std::io::BufWriter::new(fs::File::create(output)?);
    // Resolve the exact installed edition once; avoid scanning every reading
    // source segment for each of the 31,102 annotated verses.
    let mut reading:BTreeMap<String,(String,String,Option<String>)> = tx.prepare("SELECT s.source_ref,v.id,v.text,v.display_json FROM verse_texts v JOIN source_segments s ON s.id=v.segment_id WHERE v.translation_id='translation:kjv'")?.query_map([],|r|Ok((r.get(0)?,(r.get(1)?,r.get(2)?,r.get(3)?))))?.collect::<rusqlite::Result<_>>()?;
    let mut report = StudyImport {
        source_id: "kjv-strongs".into(),
        ..Default::default()
    };
    let mut seen = BTreeSet::new();
    let mut mismatches = BTreeSet::new();
    for name in names {
        let entry = archive.by_name(&name)?;
        ensure!(
            entry.size() < 4 * 1024 * 1024,
            "tagged KJV member too large"
        );
        let mut input = String::new();
        entry.take(4 * 1024 * 1024 + 1).read_to_string(&mut input)?;
        let book = input
            .trim_start_matches('\u{feff}')
            .strip_prefix("\\id ")
            .context("missing USFM book")?
            .split_whitespace()
            .next()
            .context("missing book code")?;
        if !registry.books.iter().any(|b| b.usfm == book) {
            continue;
        }
        for chapter in input.split("\\c ").skip(1) {
            let (number, body) = chapter
                .split_once(char::is_whitespace)
                .context("invalid chapter")?;
            let number: u16 = number.parse()?;
            let mut verses = body.split("\\v ");
            let prefix = verses.next().unwrap_or("");
            let title = if book == "PSA" && prefix.contains("\\d ") {
                prefix
            } else {
                ""
            };
            for verse in verses {
                let (verse, body) = verse
                    .split_once(char::is_whitespace)
                    .context("invalid verse")?;
                let verse: u16 = verse.parse()?;
                let reference = format!("{book}.{number}.{verse}");
                ensure!(
                    seen.insert(reference.clone()),
                    "duplicate source word-tag verse"
                );
                let body = if verse == 1 {
                    format!("{title}{body}")
                } else {
                    body.to_owned()
                };
                let (source, spans) = parse(&body).with_context(|| reference.clone())?;
                let (id, text, display) = reading
                    .remove(&reference)
                    .context("Tagged source verse missing from reading edition")?;
                let (target, positions) = normalize_target(&text);
                if target != source {
                    ensure!(
                        mapping.text_mismatches.contains(&reference),
                        "unreviewed KJV text difference at {reference}"
                    );
                    mismatches.insert(reference.clone());
                    serde_json::to_writer(
                        &mut output,
                        &serde_json::json!({"source_ref":reference,"status":"text_mismatch","source_text":source,"target_text":text}),
                    )?;
                } else {
                    let spans = spans
                        .into_iter()
                        .map(|s| StrongsSpan {
                            start: positions[s.start],
                            end: positions[s.end - 1] + 1,
                            codes: s.codes,
                        })
                        .collect();
                    let tags = VerseStrongs {
                        version: 1,
                        verse_text_id: id.clone(),
                        source_snapshot_id: snapshot.to_owned(),
                        text_sha256: digest(text.as_bytes()),
                        spans,
                    };
                    tags.validate(&id, &text, &digest(text.as_bytes()))
                        .map_err(anyhow::Error::msg)?;
                    let mut display = display
                        .map(|s| serde_json::from_str(&s))
                        .transpose()?
                        .unwrap_or_else(|| serde_json::json!({"type":"verse_annotations"}));
                    display["source_strongs"] = serde_json::to_value(&tags)?;
                    tx.prepare_cached("UPDATE verse_texts SET display_json=?1 WHERE id=?2")?
                        .execute(params![display.to_string(), id])?;
                    serde_json::to_writer(&mut output, &tags)?;
                }
                output.write_all(b"\n")?;
                report.source_records += 1;
            }
        }
    }
    output.flush()?;
    ensure!(
        seen.len() == 31102 && reading.is_empty() && mismatches == mapping.text_mismatches,
        "KJV word-tag coverage changed"
    );
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_word_styles_notes_and_unicode_offsets_preserve_exact_words() {
        let (text,spans)=parse("¶ The \\nd \\+w LORD|strong=\"H3068\"\\+w*\\nd* \\add is\\add* here.\\f + \\ft Not verse text.\\f*").unwrap();
        assert_eq!(text, "The LORD is here.");
        assert_eq!((spans[0].start, spans[0].end), (4, 8));
        assert_eq!(spans[0].codes, vec!["H3068"]);
        assert!(parse("\\w bad|strong=\"H3068\"").is_err());
        assert!(parse("\\unknown unreviewed content").is_err());
        assert!(parse("\\nd unclosed").is_err());
    }

    #[test]
    #[ignore = "local source evaluation only; needs real Study and tagged KJV paths"]
    fn real_source_tags_match_exact_installed_kjv_without_token_guesses() {
        let base = std::path::PathBuf::from(std::env::var("FONTES_TEST_LIBRARY").unwrap());
        let source = std::path::PathBuf::from(std::env::var("FONTES_TAGGED_KJV_SOURCE").unwrap());
        let output = std::path::PathBuf::from(std::env::var("FONTES_WORD_TAG_DB").unwrap());
        assert!(!output.exists(), "Refuse to overwrite a data generation");
        fs::copy(base, &output).unwrap();
        let source_hash = crate::file_digest(&source).unwrap();
        let snapshot = format!("source:kjv-strongs@{source_hash}");
        let mut db = rusqlite::Connection::open(&output).unwrap();
        let tx = db.transaction().unwrap();
        tx.execute("INSERT INTO source_snapshots VALUES (?1,'kjv-strongs',?2,'https://ebible.org/Scriptures/eng-kjv_usfm.zip',NULL,'local-license-evaluation-only','kjv-usfm-strongs-v1')",params![snapshot,source_hash]).unwrap();
        tx.execute(
            "UPDATE build_info SET content_id=?1",
            [format!("local-word-tags-evaluation-{}", source_hash)],
        )
        .unwrap();
        let mapping=Mapping {mapping_version:1,source_sha256:source_hash,review:"Local evaluation: source and target differ by brackets in 1 John 2:23; withhold that verse’s tags.".into(),text_mismatches:BTreeSet::from(["1JN.2.23".into()])};
        let report = import(
            &tx,
            &snapshot,
            &source,
            &Registry::load().unwrap(),
            &mapping,
            &output.with_extension("tags.jsonl"),
        )
        .unwrap();
        assert_eq!(report.source_records, 31102);
        let tagged:u64=tx.query_row("SELECT count(*) FROM verse_texts WHERE json_extract(display_json,'$.source_strongs.version')=1",[],|r|r.get(0)).unwrap();
        assert_eq!(tagged, 31101);
        let unaligned: u64 = tx
            .query_row("SELECT count(*) FROM word_alignments", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            unaligned, 0,
            "Dictionary tags must not invent original-word alignment records"
        );
        tx.commit().unwrap();
    }
}
