//! Complete source-token verse counts and bounded, edition-aware occurrence pages.
use crate::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OccurrenceBook {
    pub book_id: u16,
    pub label: String,
    pub verses: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OccurrenceVerse {
    pub ref_id: i64,
    pub label: String,
    pub preview: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WordOccurrences {
    pub code: String,
    pub books: Vec<OccurrenceBook>,
    pub total: u32,
    pub filtered_total: u32,
    pub book_id: Option<u16>,
    pub offset: u32,
    pub has_more: bool,
    pub verses: Vec<OccurrenceVerse>,
}
impl Engine {
    pub(crate) fn occurrences(
        &self,
        input: &str,
        book_id: Option<u16>,
        offset: u32,
        limit: u32,
    ) -> Result<Data> {
        if !(1..=50).contains(&limit)
            || offset > 100_000
            || book_id.is_some_and(|id| !self.books.iter().any(|b| b.id == id))
        {
            return Err(Error::invalid(
                "Choose an installed Bible book, 1–50 results, and an offset at most 100000.",
            ));
        }
        let input = input.trim().to_uppercase();
        if !matches!(input.as_bytes().first(), Some(b'G' | b'H')) {
            return Err(Error::invalid("Use a Strong’s code such as G25 or H7225."));
        }
        let number = input[1..]
            .parse::<u32>()
            .ok()
            .filter(|n| *n > 0 && *n <= 999_999)
            .ok_or_else(|| Error::invalid("Use a Strong’s code such as G25 or H7225."))?;
        let code = format!("{}{number:04}", &input[..1]);
        if !self.content.query_row(
            "SELECT EXISTS(SELECT 1 FROM strongs_codes WHERE id=?1)",
            [&code],
            |r| r.get::<_, bool>(0),
        )? {
            return Err(Error::missing("This Strong’s code is not installed."));
        }
        let books=self.content.prepare("SELECT sr.canonical_ref_id/1000000,count(DISTINCT sr.canonical_ref_id) FROM token_strongs ts JOIN original_tokens t ON t.id=ts.token_id JOIN segment_refs sr ON sr.segment_id=t.segment_id WHERE ts.strongs=?1 GROUP BY sr.canonical_ref_id/1000000 ORDER BY sr.canonical_ref_id/1000000")?.query_map([&code],|r|{let id=r.get::<_,u16>(0)?;Ok(OccurrenceBook{book_id:id,label:self.books.iter().find(|b|b.id==id).map(|b|b.name.clone()).unwrap_or_else(||id.to_string()),verses:r.get(1)?})})?.collect::<std::result::Result<Vec<_>,_>>()?;
        let total = books.iter().map(|b| b.verses).sum();
        let filtered_total = book_id.map_or(total, |id| {
            books
                .iter()
                .find(|b| b.book_id == id)
                .map_or(0, |b| b.verses)
        });
        let mut verses=self.content.prepare("SELECT ids.ref,(SELECT substr(v.text,1,500) FROM verses_by_reference v WHERE v.canonical_ref_id=ids.ref AND v.translation_id=?2 ORDER BY v.id LIMIT 1) FROM (SELECT DISTINCT sr.canonical_ref_id AS ref FROM token_strongs ts JOIN original_tokens t ON t.id=ts.token_id JOIN segment_refs sr ON sr.segment_id=t.segment_id WHERE ts.strongs=?1 AND (?3 IS NULL OR sr.canonical_ref_id/1000000=?3) ORDER BY sr.canonical_ref_id LIMIT ?4 OFFSET ?5) ids ORDER BY ids.ref")?.query_map(params![code,self.session.translation,book_id,limit+1,offset],|r|{let id=r.get(0)?;Ok(OccurrenceVerse{ref_id:id,label:label(&self.books,id),preview:r.get(1)?})})?.collect::<std::result::Result<Vec<_>,_>>()?;
        let has_more = verses.len() > limit as usize;
        verses.truncate(limit as usize);
        Ok(Data::Occurrences {
            occurrences: WordOccurrences {
                code,
                books,
                total,
                filtered_total,
                book_id,
                offset,
                has_more,
                verses,
            },
        })
    }
}
