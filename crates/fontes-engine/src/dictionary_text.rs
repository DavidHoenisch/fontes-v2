//! Display repair for Abbott-Smith snapshots compiled before TEI normalization.
//! Input is already entity-decoded text; never interpret it as HTML or decode it again.
pub(crate) fn abbott_text(input: &str) -> String {
    let mut out = String::new();
    let mut rest = input;
    while let Some((before, tail)) = rest.split_once('<') {
        out.push_str(before);
        let name = tail
            .trim_start_matches('/')
            .split(|c: char| c.is_whitespace() || c == '/' || c == '>')
            .next()
            .unwrap_or("");
        if !matches!(
            name,
            "entry"
                | "form"
                | "orth"
                | "foreign"
                | "sense"
                | "etym"
                | "gramGrp"
                | "gram"
                | "gen"
                | "gloss"
                | "pos"
                | "number"
                | "tns"
                | "mood"
                | "per"
                | "usg"
                | "subc"
                | "emph"
                | "ref"
                | "re"
                | "seg"
                | "hi"
                | "note"
                | "lb"
                | "pb"
                | "p"
                | "div"
                | "head"
                | "table"
                | "row"
                | "cell"
        ) {
            out.push('<');
            rest = tail;
            continue;
        }
        let mut quote = None;
        let end = tail.char_indices().find_map(|(i, c)| {
            if let Some(q) = quote {
                if c == q {
                    quote = None;
                }
            } else if c == '"' || c == '\'' {
                quote = Some(c);
            } else if c == '>' {
                return Some(i);
            }
            None
        });
        let Some(end) = end else {
            out.push('<');
            rest = tail;
            continue;
        };
        if matches!(name, "sense" | "p" | "div" | "lb" | "pb" | "row" | "cell") {
            out.push(' ');
        }
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
#[test]
fn removes_tei_preserving_greek_and_literal_comparisons() {
    assert_eq!(
        abbott_text(
            r#"<entry n="test"><form><orth>-βαίνω</orth>, <tns>fut.</tns> <foreign xml:lang="grc">-βήσομαι</foreign></form><sense>Love & kindness; 1 < 2; &lt;literal&gt;.</sense></entry>"#
        ),
        "-βαίνω, fut. -βήσομαι Love & kindness; 1 < 2; &lt;literal&gt;."
    );
}
