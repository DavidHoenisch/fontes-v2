//! Study adapters. Source identities and verse anchors survive normalization.
use crate::{Registry, digest, write_json};
use anyhow::{Context, Result, bail, ensure};
use roxmltree::Node;
use rusqlite::{Connection, Transaction, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub const SWORD_VERSION: &str = "fontes-sword-export-v1/sword-1.9.0";
fn exporter() -> PathBuf {
    std::env::var_os("FONTES_SWORD_EXPORTER")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/fontes-sword-export"))
}
pub fn check_exporter() -> Result<()> {
    let result = Command::new(exporter())
        .arg("--version")
        .output()
        .context("build tools/sword-export.cpp against SWORD 1.9.0; see README")?;
    ensure!(
        result.status.success() && std::str::from_utf8(&result.stdout)?.trim() == SWORD_VERSION,
        "unsupported SWORD exporter"
    );
    Ok(())
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StudyCounts {
    pub strongs_codes: u64,
    pub original_tokens: u64,
    pub token_analyses: u64,
    pub token_strongs: u64,
    pub greek_segments: u64,
    pub resources_by_source: BTreeMap<String, u64>,
    pub passage_links_by_type: BTreeMap<String, u64>,
}
impl StudyCounts {
    pub fn read(db: &Connection) -> Result<Self> {
        let count = |table: &str| -> Result<u64> {
            Ok(db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))?)
        };
        Ok(Self {
            strongs_codes: count("strongs_codes")?, original_tokens: count("original_tokens")?,
            token_analyses: count("token_analyses")?, token_strongs: count("token_strongs")?,
            greek_segments: db.query_row("SELECT count(*) FROM source_segments WHERE witness_id LIKE 'witness:robinson-tr-%'", [], |r| r.get(0))?,
            resources_by_source: db.prepare("SELECT s.source_id,count(*) FROM resources r JOIN source_snapshots s ON s.id=r.source_snapshot_id GROUP BY s.source_id")?.query_map([], |r| Ok((r.get(0)?,r.get(1)?)))?.collect::<rusqlite::Result<_>>()?,
            passage_links_by_type: db.prepare("SELECT link_type,count(*) FROM resource_links WHERE range_id IS NOT NULL GROUP BY link_type")?.query_map([], |r| Ok((r.get(0)?,r.get(1)?)))?.collect::<rusqlite::Result<_>>()?,
        })
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct StudyImport {
    pub source_id: String,
    pub source_records: usize,
    pub linked_aliases: usize,
    pub resources: usize,
    pub citations: usize,
    pub non_scripture_links: usize,
    pub unmapped_citations: Vec<CitationIssue>,
}
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CitationIssue {
    pub source_id: String,
    pub record: String,
    pub citation: String,
    pub reason: String,
}

fn text(node: Node<'_, '_>) -> String {
    fn visit(node: Node<'_, '_>, out: &mut String) {
        if node.is_text() {
            out.push_str(node.text().unwrap_or_default());
            return;
        }
        match node.tag_name().name() {
            "strongsref" => {
                let prefix = if node.attribute("language") == Some("HEBREW") {
                    'H'
                } else {
                    'G'
                };
                if let Some(n) = node.attribute("strongs") {
                    out.push_str(&format!("{prefix}{n}"));
                }
            }
            "greek" => {
                if let Some(word) = node.attribute("unicode") {
                    out.push_str(word);
                    out.push(' ');
                }
            }
            "w" if node.children().next().is_none() => {
                if let Some(code) = node.attribute("gloss") {
                    out.push_str(code);
                    out.push(' ');
                }
            }
            _ => {}
        }
        if matches!(
            node.tag_name().name(),
            "item" | "note" | "strongs_derivation" | "strongs_def" | "kjv_def" | "list"
        ) {
            out.push(' ');
        }
        for child in node.children() {
            visit(child, out);
        }
        if matches!(
            node.tag_name().name(),
            "item" | "note" | "strongs_derivation" | "strongs_def" | "kjv_def" | "list"
        ) {
            out.push(' ');
        }
    }
    let mut out = String::new();
    visit(node, &mut out);
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
#[allow(clippy::too_many_arguments)]
fn resource(
    tx: &Transaction<'_>,
    snapshot: &str,
    source: &str,
    key: &str,
    kind: &str,
    title: &str,
    author: &str,
    body: &str,
    display: serde_json::Value,
) -> Result<String> {
    let id = format!("resource:{source}:{key}");
    tx.prepare_cached("INSERT INTO resources VALUES (?1,?2,?3,?4,?5,?6,?7,?8)")?
        .execute(params![
            id,
            snapshot,
            key,
            kind,
            title,
            author,
            body,
            serde_json::to_string(&display)?
        ])?;
    tx.prepare_cached(
        "INSERT INTO search_documents(verse_text_id,resource_id,text) VALUES(NULL,?1,?2)",
    )?
    .execute(params![id, format!("{title}\n{body}")])?;
    Ok(id)
}
pub fn strong_code(prefix: char, n: &str) -> Result<String> {
    ensure!(
        matches!(prefix, 'G' | 'H'),
        "invalid Strong's language prefix"
    );
    let number: u32 = n.parse().context("invalid Strong's number")?;
    ensure!(
        number > 0 && number <= if prefix == 'G' { 5624 } else { 8674 },
        "Strong's number out of dictionary range: {prefix}{n}"
    );
    Ok(format!("{prefix}{number:04}"))
}
pub fn import_strongs(
    tx: &Transaction<'_>,
    source: &str,
    snapshot: &str,
    path: &Path,
) -> Result<StudyImport> {
    let xml = fs::read_to_string(path)?;
    // roxmltree never performs network/file I/O. The Greek source's internal DTD
    // declares its grammar; no external/entity declarations are accepted here.
    ensure!(
        !xml.contains("<!ENTITY") && !xml.contains(" SYSTEM ") && !xml.contains(" PUBLIC "),
        "external XML declarations are unsupported"
    );
    let doc = roxmltree::Document::parse_with_options(
        xml.trim_start_matches('\u{feff}'),
        roxmltree::ParsingOptions {
            allow_dtd: true,
            nodes_limit: 2_000_000,
            ..Default::default()
        },
    )?;
    let greek = source == "strongs-greek";
    let mut report = StudyImport {
        source_id: source.into(),
        ..Default::default()
    };
    for entry in doc.descendants().filter(|n| {
        n.is_element()
            && if greek {
                n.has_tag_name("entry")
            } else {
                n.tag_name().name() == "div" && n.attribute("type") == Some("entry")
            }
    }) {
        let code = strong_code(
            if greek { 'G' } else { 'H' },
            entry
                .attribute(if greek { "strongs" } else { "n" })
                .context("entry number missing")?,
        )?;
        let head = entry
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == if greek { "greek" } else { "w" });
        let headword = head
            .and_then(|n| n.attribute(if greek { "unicode" } else { "lemma" }))
            .unwrap_or("");
        let title = format!("{code} {headword}").trim().to_owned();
        let mut body = text(entry);
        if body.is_empty() {
            body = "Source entry contains no definition.".into();
        }
        tx.prepare_cached("INSERT INTO strongs_codes VALUES (?1,?2)")?
            .execute(params![code, if greek { "grc" } else { "he" }])?;
        let id = resource(
            tx,
            snapshot,
            source,
            &code,
            "strongs_dictionary",
            &title,
            "James Strong",
            &body,
            serde_json::json!({"type":"dictionary_entry","headword":headword,"transliteration":head.and_then(|n|n.attribute(if greek {"translit"} else {"xlit"})),"source_number":entry.attribute(if greek {"strongs"} else {"n"})}),
        )?;
        tx.prepare_cached("INSERT INTO resource_links VALUES (?1,?2,NULL,?3,NULL,'definition')")?
            .execute(params![format!("{id}:definition"), id, code])?;
        report.source_records += 1;
        report.resources += 1;
    }
    ensure!(
        report.resources == if greek { 5624 } else { 8674 },
        "incomplete Strong's dictionary"
    );
    Ok(report)
}

/// Unaccented Robinson ASCII: c=chi, q=theta, y=psi, v=final sigma.
pub fn greek_surface(ascii: &str) -> Result<String> {
    let alphabet = "abgdezhqiklmnxoprstufcywv";
    let unicode = "αβγδεζηθικλμνξοπρστυφχψως";
    let map: BTreeMap<_, _> = alphabet.chars().zip(unicode.chars()).collect();
    ascii
        .chars()
        .map(|c| {
            map.get(&c)
                .copied()
                .with_context(|| format!("unknown Robinson Greek character {c:?}"))
        })
        .collect()
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Analysis {
    pub strongs: Vec<String>,
    pub parsing_numbers: Vec<u16>,
    pub morph: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GreekToken {
    pub ascii: String,
    pub surface: String,
    pub analyses: Vec<Analysis>,
}

pub fn parse_tokens(input: &str) -> Result<Vec<GreekToken>> {
    let mut result: Vec<GreekToken> = Vec::new();
    let parts: Vec<_> = input.split_whitespace().collect();
    let mut i = 0;
    while i < parts.len() {
        let part = parts[i];
        if part.starts_with('(') {
            ensure!(
                part.ends_with(')') && part.contains(':'),
                "invalid numbering note"
            );
            i += 1;
            continue;
        }
        if part.bytes().all(|c| c.is_ascii_alphabetic()) {
            result.push(GreekToken {
                ascii: part.into(),
                surface: greek_surface(part)?,
                analyses: Vec::new(),
            });
            i += 1;
        }
        let start = i;
        while i < parts.len() && parts[i].bytes().all(|c| c.is_ascii_digit()) {
            i += 1;
        }
        ensure!(
            i > start && i < parts.len(),
            "missing numeric analysis near {part}"
        );
        let numbers = parts[start..i]
            .iter()
            .map(|s| s.parse())
            .collect::<std::result::Result<Vec<u16>, _>>()?;
        let strongs = numbers
            .iter()
            .filter(|n| **n > 0 && **n <= 5624)
            .map(|n| strong_code('G', &n.to_string()))
            .collect::<Result<Vec<_>>>()?;
        ensure!(
            !strongs.is_empty(),
            "analysis has no lexical Strong's numbers"
        );
        let parsing_numbers = numbers
            .into_iter()
            .filter(|n| *n == 0 || *n > 5624)
            .collect();
        let morph = parts[i]
            .strip_prefix('{')
            .and_then(|s| s.strip_suffix('}'))
            .context("missing morphology braces")?;
        ensure!(
            !morph.is_empty()
                && morph
                    .bytes()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'-'),
            "invalid morphology code"
        );
        result
            .last_mut()
            .context("analysis without token")?
            .analyses
            .push(Analysis {
                strongs,
                parsing_numbers,
                morph: morph.into(),
            });
        i += 1;
    }
    ensure!(
        result.iter().all(|t| !t.analyses.is_empty()),
        "token without analysis"
    );
    Ok(result)
}

/// Resolve triple-pipe alternatives as two independent reading streams. For
/// spelling-only alternatives the shared numeric analysis follows the last pipe.
pub fn reading_streams(input: &str) -> Result<[String; 2]> {
    let chunks: Vec<_> = input.split('|').collect();
    ensure!(chunks.len() % 3 == 1, "unbalanced source variant pipes");
    let mut out = [String::from(chunks[0]), String::from(chunks[0])];
    for group in chunks[1..].chunks(3) {
        let mut a = group[0].trim().to_owned();
        let b = group[1].trim();
        let tail = group[2];
        if !a.is_empty() && !a.contains('{') {
            let analysis = tail
                .split_whitespace()
                .take_while(|s| {
                    s.bytes().all(|c| c.is_ascii_digit())
                        || (s.starts_with('{') && s.ends_with('}'))
                })
                .collect::<Vec<_>>()
                .join(" ");
            ensure!(!analysis.is_empty(), "unparsed spelling variant");
            a.push(' ');
            a.push_str(&analysis);
            let remainder = tail.trim_start().strip_prefix(&analysis);
            // Whitespace wrapping may differ: remove by token count instead.
            let rest = if let Some(rest) = remainder {
                rest.to_owned()
            } else {
                tail.split_whitespace()
                    .skip(analysis.split_whitespace().count())
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            out[0].push_str(&format!(" {a} {rest} "));
            out[1].push_str(&format!(" {b} {analysis} {rest} "));
        } else {
            out[0].push_str(&format!(" {a} {tail} "));
            out[1].push_str(&format!(" {b} {tail} "));
        }
    }
    Ok(out)
}

pub fn range_for(registry: &Registry, reference: &str, end: bool) -> Result<i64> {
    let p: Vec<_> = reference.split('.').collect();
    ensure!(
        !p.is_empty() && p.len() <= 3,
        "invalid OSIS reference {reference}"
    );
    let book = registry
        .books
        .iter()
        .find(|b| b.osis == p[0])
        .with_context(|| format!("unknown OSIS book {reference}"))?;
    let c = if p.len() > 1 {
        p[1].parse::<usize>()?
    } else if end {
        book.verse_counts.len()
    } else {
        1
    };
    ensure!(
        c > 0 && c <= book.verse_counts.len(),
        "chapter outside registry: {reference}"
    );
    let v = if p.len() > 2 {
        p[2].parse::<u16>()?
    } else if end {
        book.verse_counts[c - 1]
    } else {
        1
    };
    ensure!(
        v > 0 && v <= book.verse_counts[c - 1],
        "verse outside registry: {reference}"
    );
    fontes_data_contracts::canonical_ref_id(book.id, c as u16, v).map_err(anyhow::Error::msg)
}
fn link(tx: &Transaction<'_>, id: &str, a: i64, b: i64, kind: &str) -> Result<()> {
    ensure!(a <= b, "reversed reference range");
    let range = format!("range:{a}-{b}");
    tx.prepare_cached("INSERT OR IGNORE INTO reference_ranges VALUES (?1,?2,?3)")?
        .execute(params![range, a, b])?;
    tx.prepare_cached("INSERT OR IGNORE INTO resource_links VALUES (?1,?2,?3,NULL,NULL,?4)")?
        .execute(params![format!("{id}:{kind}:{a}-{b}"), id, range, kind])?;
    Ok(())
}
fn own_range(registry: &Registry, key: &str) -> Result<Option<(i64, i64)>> {
    if key.is_empty() || key == "__testament.0" {
        return Ok(None);
    }
    if key == "__testament.1" {
        return Ok(Some((
            range_for(registry, "Gen", false)?,
            range_for(registry, "Mal", true)?,
        )));
    }
    if key == "__testament.2" {
        return Ok(Some((
            range_for(registry, "Matt", false)?,
            range_for(registry, "Rev", true)?,
        )));
    }
    let key = key.trim_end_matches(".0").trim_end_matches(".0");
    Ok(Some((
        range_for(registry, key, false)?,
        range_for(registry, key, true)?,
    )))
}

const TR_BOOKS: [&str; 27] = [
    "MT", "MR", "LU", "JOH", "AC", "RO", "1CO", "2CO", "GA", "EPH", "PHP", "COL", "1TH", "2TH",
    "1TI", "2TI", "TIT", "PHM", "HEB", "JAS", "1PE", "2PE", "1JO", "2JO", "3JO", "JUDE", "RE",
];
pub fn import_greek(
    tx: &Transaction<'_>,
    snapshot: &str,
    path: &Path,
    registry: &Registry,
    normalized: &Path,
) -> Result<StudyImport> {
    let mut archive = zip::ZipArchive::new(File::open(path)?)?;
    ensure!(archive.len() <= 100, "Greek archive entry limit");
    let mut report = StudyImport {
        source_id: "robinson-tr".into(),
        ..Default::default()
    };
    for reading in ["first", "second"] {
        tx.execute("INSERT INTO text_witnesses VALUES (?1,?2,'grc',?3,'KJV verse anchors; source alternate numbering retained',?4)",params![format!("witness:robinson-tr-{reading}"),format!("Robinson Textus Receptus ({reading} source reading)"),"2017-05-23 locked UTR; unaccented",snapshot])?;
    }
    let mut output = std::io::BufWriter::new(File::create(normalized)?);
    let mut all_refs = BTreeSet::new();
    for (i, code) in TR_BOOKS.iter().enumerate() {
        let suffix = format!("/parsed/{code}.UTR");
        let names: Vec<_> = archive
            .file_names()
            .collect::<std::result::Result<Vec<_>, _>>()?
            .into_iter()
            .filter(|n| n.ends_with(&suffix))
            .map(|n| n.into_owned())
            .collect();
        ensure!(names.len() == 1, "Greek book missing or duplicated: {code}");
        let mut input = String::new();
        let entry = archive.by_name(&names[0])?;
        ensure!(entry.size() <= 1024 * 1024, "Greek member too large");
        entry.take(1024 * 1024 + 1).read_to_string(&mut input)?;
        let book = &registry.books[39 + i];
        let words: Vec<_> = input.split_whitespace().collect();
        let mut cursor = 0;
        while cursor < words.len() {
            let key = words[cursor];
            ensure!(
                key.bytes().all(|c| c.is_ascii_digit() || c == b':') && key.contains(':'),
                "expected source verse boundary {code}:{key}"
            );
            cursor += 1;
            let start = cursor;
            while cursor < words.len()
                && !(words[cursor].contains(':')
                    && words[cursor]
                        .bytes()
                        .all(|c| c.is_ascii_digit() || c == b':'))
            {
                cursor += 1;
            }
            let raw = words[start..cursor].join(" ");
            let reference = format!("{}.{}", book.osis, key.replace(':', "."));
            let canonical = range_for(registry, &reference, false)?;
            ensure!(
                all_refs.insert(canonical),
                "duplicate Greek verse {reference}"
            );
            // Resolve alternatives before separating terminal subscriptions: some
            // subscriptions themselves occur inside a triple-pipe alternative.
            let streams =
                reading_streams(&raw).with_context(|| format!("variant syntax {reference}"))?;
            let subscription = raw.find('[').map(|offset| &raw[offset..]);
            let verse = |stream: &str| -> Result<String> {
                if let Some((verse, note)) = stream.split_once('[') {
                    ensure!(note.contains(']'), "unclosed subscription");
                    Ok(verse.into())
                } else {
                    Ok(stream.into())
                }
            };
            let tokens = [
                parse_tokens(&verse(&streams[0])?)
                    .with_context(|| format!("first reading {reference}"))?,
                parse_tokens(&verse(&streams[1])?)
                    .with_context(|| format!("second reading {reference}"))?,
            ];
            for (reading, tokens) in ["first", "second"].into_iter().zip(&tokens) {
                let witness = format!("witness:robinson-tr-{reading}");
                let segment = format!("segment:robinson-tr:{reading}:{reference}");
                tx.prepare_cached("INSERT INTO source_segments VALUES (?1,?2,?3,?4,?5)")?
                    .execute(params![
                        segment,
                        snapshot,
                        witness,
                        reference,
                        format!("{code}:{key}:{reading}")
                    ])?;
                tx.prepare_cached("INSERT INTO segment_refs VALUES (?1,?2,'exact')")?
                    .execute(params![segment, canonical])?;
                for (position, token) in tokens.iter().enumerate() {
                    let id = format!("token:tr:{reading}:{reference}:{position}");
                    tx.prepare_cached("INSERT INTO original_tokens VALUES (?1,?2,?3,?4,?5,NULL)")?
                        .execute(params![id, witness, segment, token.surface, position])?;
                    for (analysis, a) in token.analyses.iter().enumerate() {
                        tx.prepare_cached(
                            "INSERT INTO token_analyses VALUES (?1,?2,NULL,'robinson',?3)",
                        )?
                        .execute(params![id, analysis, a.morph])?;
                        for code in &a.strongs {
                            tx.prepare_cached("INSERT INTO token_strongs VALUES (?1,?2,?3)")?
                                .execute(params![id, analysis, code])?;
                        }
                    }
                }
            }
            let body = tokens[0]
                .iter()
                .map(|t| t.surface.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let display = serde_json::json!({"type":"greek_source","source_ascii":raw,"subscription":subscription,"witnesses":["witness:robinson-tr-first","witness:robinson-tr-second"]});
            let id = resource(
                tx,
                snapshot,
                "robinson-tr",
                &reference,
                "greek_source",
                &format!("Robinson TR {reference}"),
                "Maurice A. Robinson",
                &body,
                display.clone(),
            )?;
            link(tx, &id, canonical, canonical, "source_text")?;
            serde_json::to_writer(
                &mut output,
                &serde_json::json!({"reference":reference,"data":display}),
            )?;
            output.write_all(b"\n")?;
            report.source_records += 1;
            report.resources += 1;
        }
    }
    output.flush()?;
    let expected: BTreeSet<_> = registry
        .references()?
        .into_iter()
        .filter(|r| *r >= 40_000_000)
        .collect();
    ensure!(
        all_refs == expected,
        "incomplete Greek NT canonical coverage"
    );
    Ok(report)
}

/// Strip source markup without SWORD's display-option filters (which can remove
/// entire preverse commentary blocks). Decode entities only after recognizing
/// tags, so escaped angle brackets remain visible prose.
fn plain_markup(raw: &str) -> Result<String> {
    fn decode(segment: &str, out: &mut String) -> Result<()> {
        let mut rest = segment;
        while let Some((before, entity)) = rest.split_once('&') {
            out.push_str(before);
            let (name, after) = entity
                .split_once(';')
                .context("unterminated source entity")?;
            let c = match name {
                "amp" => '&',
                "lt" => '<',
                "gt" => '>',
                "quot" => '"',
                "apos" => '\'',
                "nbsp" => ' ',
                _ => {
                    let n = if let Some(hex) = name.strip_prefix("#x") {
                        u32::from_str_radix(hex, 16)?
                    } else if let Some(n) = name.strip_prefix('#') {
                        n.parse()?
                    } else {
                        bail!("unsupported source entity &{name};")
                    };
                    char::from_u32(n).context("invalid source entity code point")?
                }
            };
            out.push(c);
            rest = after;
        }
        out.push_str(rest);
        Ok(())
    }
    let mut out = String::new();
    let mut rest = raw;
    while let Some((before, tag)) = rest.split_once('<') {
        decode(before, &mut out)?;
        let candidate = tag
            .trim_start_matches('/')
            .split(|c: char| c.is_whitespace() || c == '/' || c == '>')
            .next()
            .unwrap_or("");
        if !matches!(
            candidate,
            "hi" | "reference"
                | "div"
                | "title"
                | "chapter"
                | "l"
                | "milestone"
                | "lg"
                | "note"
                | "ref"
                | "item"
                | "lb"
                | "entryFree"
                | "def"
                | "list"
                | "scripRef"
                | "br"
                | "sync"
                | "p"
        ) {
            out.push('<');
            rest = tag;
            continue;
        }
        let mut quote = None;
        let mut end = None;
        for (i, c) in tag.char_indices() {
            if let Some(q) = quote {
                if c == q {
                    quote = None;
                }
            } else if c == '\'' || c == '"' {
                quote = Some(c);
            } else if c == '>' {
                end = Some(i);
                break;
            }
        }
        let end = end.context("unclosed source markup tag")?;
        let markup = &tag[..end];
        let name = markup
            .trim_start_matches('/')
            .split(|c: char| c.is_whitespace() || c == '/')
            .next()
            .unwrap_or("");
        if matches!(
            name,
            "br" | "lb" | "p" | "div" | "title" | "item" | "list" | "l" | "lg"
        ) {
            out.push('\n');
        }
        rest = &tag[end + 1..];
    }
    decode(rest, &mut out)?;
    Ok(out
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n"))
}

#[derive(Deserialize)]
struct SwordEntry {
    key: String,
    #[serde(default)]
    same_as: Option<String>,
    #[serde(default)]
    raw: String,
    #[serde(default)]
    citations: Vec<Citation>,
}
#[derive(Serialize, Deserialize)]
struct Citation {
    source: String,
    kind: String,
    ranges: Vec<[String; 2]>,
}
fn citation_ranges(c: &Citation, registry: &Registry) -> Result<Vec<(i64, i64)>> {
    if c.kind == "osisRef" {
        c.source
            .split_whitespace()
            .map(|part| {
                let (start, end) = part.split_once('-').unwrap_or((part, part));
                let a = range_for(registry, start, false)?;
                let b = range_for(registry, end, true)?;
                ensure!(a <= b, "reversed OSIS reference");
                Ok((a, b))
            })
            .collect()
    } else {
        ensure!(!c.ranges.is_empty(), "SWORD could not parse reference");
        c.ranges
            .iter()
            .map(|[start, end]| {
                let a = range_for(registry, start, false)?;
                let b = range_for(registry, end, true)?;
                ensure!(a <= b, "reversed reference");
                Ok((a, b))
            })
            .collect()
    }
}
fn extract_module(path: &Path, dir: &Path) -> Result<()> {
    let mut zip = zip::ZipArchive::new(File::open(path)?)?;
    ensure!(zip.len() <= 128, "SWORD archive member limit");
    let mut total = 0;
    for i in 0..zip.len() {
        let entry = zip.by_index(i)?;
        ensure!(
            entry.unix_mode().is_none_or(|m| m & 0o170000 != 0o120000),
            "SWORD archive symlink"
        );
        let safe = entry.enclosed_name().context("unsafe SWORD archive path")?;
        ensure!(
            safe.starts_with("mods.d") || safe.starts_with("modules"),
            "unexpected SWORD archive member"
        );
        let target = dir.join(safe);
        if entry.is_dir() {
            fs::create_dir_all(target)?;
            continue;
        }
        ensure!(
            entry.size() <= 64 * 1024 * 1024,
            "SWORD archive member limit"
        );
        total += entry.size();
        ensure!(total <= 128 * 1024 * 1024, "SWORD archive size limit");
        fs::create_dir_all(target.parent().context("no member parent")?)?;
        let bytes = std::io::copy(
            &mut entry.take(64 * 1024 * 1024 + 1),
            &mut File::create(target)?,
        )?;
        ensure!(bytes <= 64 * 1024 * 1024, "SWORD decompression limit");
    }
    Ok(())
}
pub fn import_sword(
    tx: &Transaction<'_>,
    source: &str,
    snapshot: &str,
    path: &Path,
    registry: &Registry,
    normalized: &Path,
) -> Result<StudyImport> {
    let temp = tempfile::tempdir()?;
    extract_module(path, temp.path())?;
    let (module, kind, author) = match source {
        "matthew-henry" => ("MHC", "commentary", "Matthew Henry and continuators"),
        "nave" => ("Nave", "topic", "Orville J. Nave"),
        "tsk" => (
            "TSK",
            "cross_reference",
            "Treasury of Scripture Knowledge contributors",
        ),
        _ => bail!("unknown SWORD source"),
    };
    let export = File::create(normalized)?;
    let status = Command::new(exporter())
        .arg(temp.path())
        .arg(module)
        .stdout(Stdio::from(export))
        .status()?;
    ensure!(
        status.success(),
        "SWORD export failed for {module}: {status}"
    );
    import_sword_records(tx, source, snapshot, registry, normalized, kind, author)
}
fn import_sword_records(
    tx: &Transaction<'_>,
    source: &str,
    snapshot: &str,
    registry: &Registry,
    normalized: &Path,
    kind: &str,
    author: &str,
) -> Result<StudyImport> {
    let module = match source {
        "matthew-henry" => "MHC",
        "nave" => "Nave",
        _ => "TSK",
    };
    let mut report = StudyImport {
        source_id: source.into(),
        ..Default::default()
    };
    let mut aliases: BTreeMap<String, Option<String>> = BTreeMap::new();
    for line in BufReader::new(File::open(normalized)?).lines() {
        let entry: SwordEntry = serde_json::from_str(&line?)?;
        report.source_records += 1;
        let plain = plain_markup(&entry.raw)
            .with_context(|| format!("plain text {source}:{}", entry.key))?;
        // MHC's locked OSIS contains numeric references escaped a second time
        // (e.g. &amp;#224;). Decode that documented layer after stripping markup.
        let plain = if source == "matthew-henry" {
            fontes_data_contracts::decode_numeric_text_entities(&plain)
        } else {
            plain
        };
        let id = if let Some(first) = entry.same_as {
            report.linked_aliases += 1;
            aliases
                .get(&first)
                .with_context(|| format!("SWORD alias precedes content {first}"))?
                .clone()
        } else if plain.is_empty() {
            // Module metadata only; raw source retained in the exact archive and
            // normalized stream. No empty article is advertised to applications.
            None
        } else {
            let mut accepted = Vec::new();
            let mut unresolved = Vec::new();
            let mut see_also = Vec::new();
            for citation in &entry.citations {
                if citation.kind == "target" {
                    report.non_scripture_links += 1;
                    see_also.push(citation.source.clone());
                    continue;
                }
                report.citations += 1;
                match citation_ranges(citation, registry) {
                    Ok(ranges) => accepted.extend(ranges),
                    Err(e) => {
                        // Retain valid targets in a partly malformed citation list.
                        if citation.kind == "osisRef" {
                            for part in citation.source.split_whitespace() {
                                let single = Citation {
                                    source: part.into(),
                                    kind: "osisRef".into(),
                                    ranges: vec![],
                                };
                                if let Ok(ranges) = citation_ranges(&single, registry) {
                                    accepted.extend(ranges);
                                }
                            }
                        } else {
                            for range in &citation.ranges {
                                let single = Citation {
                                    source: citation.source.clone(),
                                    kind: citation.kind.clone(),
                                    ranges: vec![range.clone()],
                                };
                                if let Ok(ranges) = citation_ranges(&single, registry) {
                                    accepted.extend(ranges);
                                }
                            }
                        }
                        unresolved.push(citation.source.clone());
                        report.unmapped_citations.push(CitationIssue {
                            source_id: source.into(),
                            record: entry.key.clone(),
                            citation: citation.source.clone(),
                            reason: e.to_string(),
                        });
                    }
                }
            }
            accepted.sort_unstable();
            accepted.dedup();
            let key = if entry.key.is_empty() {
                "module-heading"
            } else {
                &entry.key
            };
            let mut display = serde_json::json!({"type":"study_article","format":"plain_paragraphs","source_key":entry.key,"citations":entry.citations.iter().map(|c|serde_json::json!({"source":c.source,"kind":c.kind})).collect::<Vec<_>>(),"unmapped_citations":unresolved,"see_also":see_also,"raw_sha256":digest(entry.raw.as_bytes())});
            if source == "matthew-henry" {
                display["text_normalization"] = "mhc-numeric-v1".into();
            }
            let id = resource(
                tx,
                snapshot,
                source,
                key,
                kind,
                &format!("{module}: {key}"),
                author,
                &plain,
                display,
            )?;
            for (a, b) in accepted {
                link(
                    tx,
                    &id,
                    a,
                    b,
                    if source == "tsk" {
                        "cross_reference"
                    } else {
                        "citation"
                    },
                )?;
            }
            report.resources += 1;
            Some(id)
        };
        if let Some(id) = &id
            && source != "nave"
            && let Some((a, b)) = own_range(registry, &entry.key)?
        {
            link(tx, id, a, b, "applies_to")?;
        }
        ensure!(
            aliases.insert(entry.key, id).is_none(),
            "duplicate SWORD key"
        );
    }
    report.unmapped_citations.sort();
    Ok(report)
}

pub fn validate_study(db: &Connection, counts: &StudyCounts) -> Result<()> {
    ensure!(counts.strongs_codes == 14298, "incomplete dictionaries");
    ensure!(
        counts.greek_segments == 15914,
        "Greek reading streams incomplete"
    );
    let greek_tokens: u64 = db.query_row(
        "SELECT count(*) FROM original_tokens WHERE witness_id LIKE 'witness:robinson-tr-%'",
        [],
        |r| r.get(0),
    )?;
    ensure!(
        greek_tokens > 280000 && greek_tokens < 300000,
        "Greek analysis coverage mismatch"
    );
    for (source, min) in [
        ("strongs-greek", 5624),
        ("strongs-hebrew", 8674),
        ("robinson-tr", 7957),
        ("matthew-henry", 5000),
        ("nave", 5300),
        ("tsk", 29000),
    ] {
        ensure!(
            counts
                .resources_by_source
                .get(source)
                .is_some_and(|n| *n >= min),
            "missing study resource coverage: {source}"
        );
    }
    let empty:u64=db.query_row("SELECT count(*) FROM original_tokens t WHERE t.witness_id LIKE 'witness:robinson-tr-%' AND (NOT EXISTS(SELECT 1 FROM token_analyses a WHERE a.token_id=t.id) OR NOT EXISTS(SELECT 1 FROM token_strongs s WHERE s.token_id=t.id))",[],|r|r.get(0))?;
    ensure!(empty == 0, "unanalyzed Greek tokens");
    let hebrew: u64 = db.query_row(
        "SELECT count(*) FROM original_tokens WHERE witness_id='witness:oshb-wlc'",
        [],
        |r| r.get(0),
    )?;
    if hebrew > 0 {
        ensure!(hebrew == 305507, "Hebrew word coverage mismatch");
        let verses:u64=db.query_row("SELECT count(DISTINCT sr.canonical_ref_id) FROM source_segments s JOIN segment_refs sr ON sr.segment_id=s.id WHERE s.witness_id='witness:oshb-wlc'",[],|r|r.get(0))?;
        ensure!(verses == 23144, "Hebrew canonical coverage mismatch");
        let missing:u64=db.query_row("SELECT count(*) FROM original_tokens t WHERE t.witness_id='witness:oshb-wlc' AND NOT EXISTS(SELECT 1 FROM token_analyses a WHERE a.token_id=t.id AND a.morph_system='oshm' AND a.morph_code IS NOT NULL)",[],|r|r.get(0))?;
        ensure!(missing == 0, "Hebrew morphology missing");
        let lord:u64=db.query_row("SELECT count(*) FROM original_tokens t JOIN token_strongs ts ON ts.token_id=t.id JOIN segment_refs sr ON sr.segment_id=t.segment_id WHERE sr.canonical_ref_id=20016001 AND ts.strongs='H3068'",[],|r|r.get(0))?;
        ensure!(lord == 1, "Proverbs 16:1 Hebrew lookup failed");
    }
    let tagged_source: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM source_snapshots WHERE source_id='kjv-strongs')",
        [],
        |r| r.get(0),
    )?;
    if tagged_source {
        let tagged: u64 = db.query_row("SELECT count(*) FROM verse_texts WHERE translation_id='translation:kjv' AND json_type(display_json,'$.source_strongs')='object'", [], |r| r.get(0))?;
        ensure!(tagged == 31101, "KJV dictionary tag coverage mismatch");
        let wrong_edition: u64 = db.query_row("SELECT count(*) FROM verse_texts WHERE translation_id!='translation:kjv' AND json_type(display_json,'$.source_strongs')='object'", [], |r| r.get(0))?;
        ensure!(
            wrong_edition == 0,
            "KJV dictionary tags leaked to another edition"
        );
    }
    let covered:u64=db.query_row("SELECT count(*) FROM (SELECT s.witness_id,r.canonical_ref_id FROM source_segments s JOIN segment_refs r ON r.segment_id=s.id WHERE s.witness_id LIKE 'witness:robinson-tr-%' GROUP BY 1,2)",[],|r|r.get(0))?;
    ensure!(covered == 15914, "Greek canonical coverage mismatch");
    let definitions:u64=db.query_row("SELECT count(*) FROM strongs_codes c WHERE NOT EXISTS(SELECT 1 FROM resource_links l WHERE l.strongs=c.id AND l.link_type='definition')",[],|r|r.get(0))?;
    ensure!(definitions == 0, "dictionary definition missing");
    let john:u64=db.query_row("SELECT count(*) FROM original_tokens t JOIN token_strongs s ON s.token_id=t.id JOIN token_analyses a ON a.token_id=t.id AND a.analysis_index=s.analysis_index JOIN segment_refs r ON r.segment_id=t.segment_id WHERE r.canonical_ref_id=43001001 AND t.surface='λογος' AND s.strongs='G3056' AND a.morph_code='N-NSM'",[],|r|r.get(0))?;
    ensure!(john == 6, "John 1:1 Greek smoke check failed");
    for source in ["matthew-henry", "nave", "tsk"] {
        let links:u64=db.query_row("SELECT count(*) FROM resource_links l JOIN reference_ranges rr ON rr.id=l.range_id JOIN resources r ON r.id=l.resource_id JOIN source_snapshots s ON s.id=r.source_snapshot_id WHERE s.source_id=?1 AND rr.start_ref_id<=43003016 AND rr.end_ref_id>=43003016",[source],|r|r.get(0))?;
        ensure!(links > 0, "John 3:16 passage lookup failed: {source}");
    }
    ensure!(
        counts
            .passage_links_by_type
            .get("cross_reference")
            .is_some_and(|n| *n > 300000),
        "TSK target links missing"
    );
    Ok(())
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ExpectedImport {
    source_records: usize,
    linked_aliases: usize,
    resources: usize,
    citations: usize,
    non_scripture_links: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CitationPolicy {
    mapping_version: u32,
    review: String,
    source_sha256s: BTreeMap<String, String>,
    expected_imports: BTreeMap<String, ExpectedImport>,
    known_unmapped_citations: Vec<CitationIssue>,
}
pub fn check_citation_policy(
    path: &Path,
    lock: &fontes_data_contracts::SourceLock,
    reports: &[StudyImport],
) -> Result<()> {
    let mut policy: CitationPolicy = serde_json::from_slice(&fs::read(path)?)?;
    ensure!(
        policy.mapping_version == 1 && !policy.review.is_empty(),
        "invalid study citation policy"
    );
    for id in ["matthew-henry", "nave", "tsk"] {
        ensure!(
            policy.source_sha256s.get(id)
                == lock
                    .sources
                    .iter()
                    .find(|s| s.source_id == id)
                    .map(|s| &s.sha256),
            "study source changed without citation review: {id}"
        );
    }
    ensure!(
        reports.len() == policy.expected_imports.len(),
        "study source import missing"
    );
    for report in reports {
        let actual = ExpectedImport {
            source_records: report.source_records,
            linked_aliases: report.linked_aliases,
            resources: report.resources,
            citations: report.citations,
            non_scripture_links: report.non_scripture_links,
        };
        ensure!(
            policy.expected_imports.get(&report.source_id) == Some(&actual),
            "study record or citation coverage changed: {}",
            report.source_id
        );
    }
    let mut actual: Vec<_> = reports
        .iter()
        .flat_map(|r| r.unmapped_citations.iter())
        .collect();
    actual.sort();
    policy.known_unmapped_citations.sort();
    ensure!(
        actual == policy.known_unmapped_citations.iter().collect::<Vec<_>>(),
        "study citation diagnostics changed; review data/mappings/study-v1.json"
    );
    Ok(())
}

pub fn write_audit(path: &Path, reports: &[StudyImport]) -> Result<()> {
    write_json(
        path,
        &serde_json::json!({"sword_exporter":SWORD_VERSION,"imports":reports}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn greek_decoding_and_multiple_analyses_keep_their_associations() {
        assert!(greek_surface("logov qeoû").is_err());
        assert_eq!(
            "logov qew xulon yuch"
                .split_whitespace()
                .map(|s| greek_surface(s).unwrap())
                .collect::<Vec<_>>()
                .join(" "),
            "λογος θεω ξυλον ψυχη"
        );
        let tokens = parse_tokens(
            "ercomenon 2064 5740 {V-PNP-ASM} 2064 5740 {V-PNP-NSN} eanper 0 1437 4007 {COND}",
        )
        .unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].analyses.len(), 2);
        assert_eq!(tokens[0].analyses[1].morph, "V-PNP-NSN");
        assert_eq!(tokens[0].analyses[1].parsing_numbers, [5740]);
        assert_eq!(tokens[1].analyses[0].strongs, ["G1437", "G4007"]);
        assert_eq!(tokens[1].analyses[0].parsing_numbers, [0]);
        assert!(parse_tokens("logov {N-NSM}").is_err());
        assert!(parse_tokens("logov 3056").is_err());
        assert!(parse_tokens("logov 9999 {N-NSM}").is_err());
    }
    #[test]
    fn variant_readings_support_shared_analyses_insertions_and_reordering() {
        let readings = reading_streams("| nazaret | nazareq | 3478 {N-PRI}").unwrap();
        assert_eq!(parse_tokens(&readings[0]).unwrap()[0].surface, "ναζαρετ");
        assert_eq!(parse_tokens(&readings[1]).unwrap()[0].surface, "ναζαρεθ");
        let readings =
            reading_streams("kai 2532 {CONJ} | | autou 846 {P-GSM} | logov 3056 {N-NSM}").unwrap();
        assert_eq!(parse_tokens(&readings[0]).unwrap().len(), 2);
        assert_eq!(parse_tokens(&readings[1]).unwrap().len(), 3);
        let readings = reading_streams(
            "| cristou 5547 {N-GSM} ihsou 2424 {N-GSM} | ihsou 2424 {N-GSM} cristou 5547 {N-GSM} |",
        )
        .unwrap();
        assert_eq!(parse_tokens(&readings[0]).unwrap()[0].surface, "χριστου");
        assert_eq!(parse_tokens(&readings[1]).unwrap()[0].surface, "ιησου");
        let readings = reading_streams("| mainan | menam 0 | 3104 {N-PRI}").unwrap();
        assert_eq!(
            parse_tokens(&readings[1]).unwrap()[0].analyses[0].strongs,
            ["G3104"]
        );
        assert!(reading_streams("| logov 3056 {N-NSM}").is_err());
    }
    #[test]
    fn references_are_checked_against_registry_and_intros_cover_their_scope() {
        let registry = Registry::load().unwrap();
        assert_eq!(
            own_range(&registry, "Gen").unwrap(),
            Some((1001001, 1050026))
        );
        assert_eq!(
            own_range(&registry, "John.3.0").unwrap(),
            Some((43003001, 43003036))
        );
        assert_eq!(
            own_range(&registry, "John.3.16").unwrap(),
            Some((43003016, 43003016))
        );
        assert!(range_for(&registry, "John.3.37", false).is_err());
        assert!(range_for(&registry, "Unknown.1.1", false).is_err());
        let c = Citation {
            source: "John.3.16-John.3.18 Rom.8".into(),
            kind: "osisRef".into(),
            ranges: vec![],
        };
        assert_eq!(
            citation_ranges(&c, &registry).unwrap(),
            [(43003016, 43003018), (45008001, 45008039)]
        );
    }
    #[test]
    fn commentary_aliases_keep_all_verses_and_preverse_prose() {
        let raw = r#"<div type="x-milestone" subType="x-preverse" sID="pv1"/><title>Preface</title><div sID="p1" type="x-p"/>God &amp; love. Roma &amp;#224;. <reference osisRef="John.3.18">John 3:18</reference><div eID="p1" type="x-p"/><div eID="pv1"/>"#;
        let plain = plain_markup(raw).unwrap();
        assert_eq!(plain, "Preface\nGod & love. Roma &#224;. John 3:18");
        assert_eq!(
            plain_markup("<p>&lt;b&gt; &amp; &#x3b1; &#945;</p>").unwrap(),
            "<b> & α α"
        );
        assert!(plain_markup("<p>&unknown;</p>").is_err());
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch(fontes_data_contracts::CONTENT_SCHEMA_SQL)
            .unwrap();
        let tx = db.transaction().unwrap();
        tx.execute("INSERT INTO books VALUES(43,'John','John','NT',43)", [])
            .unwrap();
        for verse in 16..=18 {
            tx.execute(
                "INSERT INTO canonical_refs VALUES(?1,43,3,?2)",
                params![43003000 + verse, verse],
            )
            .unwrap();
        }
        tx.execute("INSERT INTO source_snapshots VALUES('source','matthew-henry','hash','https://example.org',NULL,'review','adapter')",[]).unwrap();
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("records.jsonl");
        let first = serde_json::json!({"key":"John.3.16","raw":raw,"citations":[{"source":"John.3.18","kind":"osisRef","ranges":[]}]});
        let second = serde_json::json!({"key":"John.3.17","same_as":"John.3.16"});
        fs::write(&path, format!("{first}\n{second}\n")).unwrap();
        let report = import_sword_records(
            &tx,
            "matthew-henry",
            "source",
            &Registry::load().unwrap(),
            &path,
            "commentary",
            "Matthew Henry",
        )
        .unwrap();
        let imported: String = tx
            .query_row("SELECT content FROM resources", [], |r| r.get(0))
            .unwrap();
        assert_eq!(imported, "Preface\nGod & love. Roma à. John 3:18");
        let indexed: String = tx
            .query_row("SELECT text FROM search_documents", [], |r| r.get(0))
            .unwrap();
        assert!(indexed.contains("Roma à."));
        assert_eq!(report.resources, 1);
        assert_eq!(report.linked_aliases, 1);
        assert!(report.unmapped_citations.is_empty());
        let applies: u32 = tx
            .query_row(
                "SELECT count(*) FROM resource_links WHERE link_type='applies_to'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(applies, 2);
        let targets: u32 = tx
            .query_row(
                "SELECT count(*) FROM resource_links WHERE link_type='citation'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(targets, 1);
    }
    #[test]
    fn citation_policy_detects_lost_records_new_anomalies_and_source_replacements() {
        let policy_bytes = include_bytes!("../../../data/mappings/study-v1.json");
        let policy: CitationPolicy = serde_json::from_slice(policy_bytes).unwrap();
        let mut lock: fontes_data_contracts::SourceLock =
            serde_json::from_str(include_str!("../../../data/sources.lock.json")).unwrap();
        let mut reports: Vec<_> = policy
            .expected_imports
            .into_iter()
            .map(|(source_id, v)| {
                let issues = policy
                    .known_unmapped_citations
                    .iter()
                    .filter(|i| i.source_id == source_id)
                    .map(|i| CitationIssue {
                        source_id: i.source_id.clone(),
                        record: i.record.clone(),
                        citation: i.citation.clone(),
                        reason: i.reason.clone(),
                    })
                    .collect();
                StudyImport {
                    source_id,
                    source_records: v.source_records,
                    linked_aliases: v.linked_aliases,
                    resources: v.resources,
                    citations: v.citations,
                    non_scripture_links: v.non_scripture_links,
                    unmapped_citations: issues,
                }
            })
            .collect();
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("policy.json");
        fs::write(&path, policy_bytes).unwrap();
        check_citation_policy(&path, &lock, &reports).unwrap();
        let resource_index = reports.iter().position(|r| r.resources > 0).unwrap();
        reports[resource_index].resources -= 1;
        assert!(check_citation_policy(&path, &lock, &reports).is_err());
        reports[resource_index].resources += 1;
        let index = reports
            .iter()
            .position(|r| !r.unmapped_citations.is_empty())
            .unwrap();
        let issue = reports[index].unmapped_citations.pop().unwrap();
        assert!(check_citation_policy(&path, &lock, &reports).is_err());
        reports[index].unmapped_citations.push(issue);
        lock.sources
            .iter_mut()
            .find(|s| s.source_id == "tsk")
            .unwrap()
            .sha256 = "a".repeat(64);
        assert!(check_citation_policy(&path, &lock, &reports).is_err());
    }
    #[test]
    fn dictionary_mixed_text_keeps_embedded_cross_references_and_greek() {
        let xml = r#"<entry>from <strongsref language="GREEK" strongs="3056"/>; <strongs_def>the <em>Word</em> &amp; speech</strongs_def><greek unicode="λόγος"/></entry>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        assert_eq!(
            text(doc.root_element()),
            "from G3056; the Word & speech λόγος"
        );
        assert!(strong_code('G', "0").is_err());
        assert!(strong_code('G', "5707").is_err());
        assert_eq!(strong_code('H', "430").unwrap(), "H0430");
    }
}
