//! Source-provided dictionary associations, distinct from original-token alignments.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerseStrongs {
    pub version: u32,
    pub verse_text_id: String,
    pub source_snapshot_id: String,
    pub text_sha256: String,
    pub spans: Vec<StrongsSpan>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrongsSpan {
    /// Unicode scalar offsets, zero based and end exclusive.
    pub start: usize,
    pub end: usize,
    pub codes: Vec<String>,
}

impl VerseStrongs {
    pub fn validate(&self, verse_id: &str, text: &str, sha256: &str) -> Result<(), String> {
        if self.version != 1
            || self.verse_text_id != verse_id
            || self.text_sha256 != sha256
            || self.source_snapshot_id.is_empty()
            || self.spans.len() > 1000
        {
            return Err("Source word tags do not match their verse text or version.".into());
        }
        let length = text.chars().count();
        let mut previous_end = 0;
        for span in &self.spans {
            if span.start < previous_end
                || span.start >= span.end
                || span.end > length
                || span.codes.is_empty()
                || span.codes.len() > 20
            {
                return Err("A source word tag has invalid or overlapping offsets.".into());
            }
            let mut unique = std::collections::BTreeSet::new();
            for code in &span.codes {
                let number = code.get(1..).and_then(|s| s.parse::<u32>().ok());
                let max = match code.as_bytes().first() {
                    Some(b'H') => 8674,
                    Some(b'G') => 5624,
                    _ => 0,
                };
                if !code
                    .get(1..)
                    .is_some_and(|s| s.bytes().all(|c| c.is_ascii_digit()))
                    || code.len() != 5
                    || !number.is_some_and(|n| n > 0 && n <= max)
                    || !unique.insert(code)
                {
                    return Err("A source word tag has an invalid Strong’s code.".into());
                }
            }
            previous_end = span.end;
        }
        Ok(())
    }
}
