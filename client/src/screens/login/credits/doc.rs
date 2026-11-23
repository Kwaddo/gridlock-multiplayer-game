//! CREDITS.md (repo root), compiled into the game and read as a list of rows.
//!
//! Only the Markdown the file uses: `#`/`##` headings, paragraphs, `- ` lists,
//! tables (first row is the header and is skipped) and `[text](url)` links.
//! HTML comments are left out. Compiled in, so the browser build needs no fetch.

/// The credits as written in the repo.
pub const CREDITS_MD: &str = include_str!("../../../../../CREDITS.md");

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// `# ...`: the document title (the screen has its own, so it is not drawn).
    Title,
    /// `## ...`: one creator or group.
    Heading,
    /// A paragraph or a `- ` list item.
    Text,
    /// A table row: what it is in the game, then the credited work.
    Entry,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Row {
    pub kind: Kind,
    /// For an entry, what it is in the game; empty otherwise.
    pub label: String,
    /// The text with link markup taken out.
    pub text: String,
    /// The row's first link, opened when it is clicked.
    pub link: Option<String>,
}

/// Every row of `md`, in order.
pub fn parse(md: &str) -> Vec<Row> {
    let md = strip_comments(md);
    let mut rows = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    let mut table_header_seen = false;

    let flush = |paragraph: &mut Vec<&str>, rows: &mut Vec<Row>| {
        if !paragraph.is_empty() {
            rows.push(text_row(Kind::Text, "", &paragraph.join(" ")));
            paragraph.clear();
        }
    };

    for line in md.lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            table_header_seen = false;
        }
        if line.is_empty() {
            flush(&mut paragraph, &mut rows);
        } else if let Some(rest) = line.strip_prefix("## ") {
            flush(&mut paragraph, &mut rows);
            rows.push(text_row(Kind::Heading, "", rest));
        } else if let Some(rest) = line.strip_prefix("# ") {
            flush(&mut paragraph, &mut rows);
            rows.push(text_row(Kind::Title, "", rest));
        } else if let Some(rest) = line.strip_prefix("- ") {
            flush(&mut paragraph, &mut rows);
            rows.push(text_row(Kind::Text, "", &format!("- {}", rest.trim())));
        } else if line.starts_with('|') {
            flush(&mut paragraph, &mut rows);
            let cells = table_cells(line);
            if cells.iter().all(|c| !c.is_empty() && c.chars().all(|ch| matches!(ch, '-' | ':' | ' '))) {
                continue; // |---|---|
            }
            if !table_header_seen {
                table_header_seen = true;
                continue;
            }
            if let Some((first, rest)) = cells.split_first() {
                let (label, _) = strip_links(first);
                let mut row = text_row(Kind::Entry, &label, &rest.join(" - "));
                if row.link.is_none() {
                    row.link = strip_links(first).1;
                }
                rows.push(row);
            }
        } else {
            paragraph.push(line);
        }
    }
    flush(&mut paragraph, &mut rows);
    rows
}

fn text_row(kind: Kind, label: &str, raw: &str) -> Row {
    let (text, link) = strip_links(raw);
    Row { kind, label: label.to_string(), text, link }
}

/// The cells of `| a | b |`, trimmed.
fn table_cells(line: &str) -> Vec<&str> {
    let inner = line.trim().trim_start_matches('|').trim_end_matches('|');
    inner.split('|').map(str::trim).collect()
}

/// Drops `<!-- ... -->`, even over several lines (an unclosed one runs to the end).
fn strip_comments(md: &str) -> String {
    let mut out = String::with_capacity(md.len());
    let mut rest = md;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            Some(end) => rest = &rest[start + end + 3..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// `see [this](https://x)` -> ("see this", Some("https://x")): the text as shown,
/// and the first link's target. Brackets that are not a link stay as they are.
fn strip_links(s: &str) -> (String, Option<String>) {
    let mut text = String::with_capacity(s.len());
    let mut link = None;
    let mut rest = s;
    while let Some(open) = rest.find('[') {
        // the label may hold quotes but not `](`; the closing `)` is the URL's end
        let Some(mid) = rest[open..].find("](") else { break };
        let label_end = open + mid;
        let url_start = label_end + 2;
        let Some(url_len) = rest[url_start..].find(')') else { break };
        text.push_str(&rest[..open]);
        text.push_str(&rest[open + 1..label_end]);
        if link.is_none() {
            link = Some(rest[url_start..url_start + url_len].trim().to_string());
        }
        rest = &rest[url_start + url_len + 1..];
    }
    text.push_str(rest);
    (text.trim().to_string(), link)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_paragraphs_and_tables() {
        let md = "# Credits\n\nIntro line one\nand two.\n\n<!-- hidden\n | x | y | -->\n\
                  ## Guns by [Tony](https://a.example/tony)\n\n\
                  Licensed [CC BY 4.0](https://cc.example/by).\n\n\
                  | In game | Model |\n|---|---|\n\
                  | M1911 | [\"Low-Poly M1911\"](https://a.example/m1911) |\n\
                  | Chars | Pete, Amy |\n\n- a list item\n";
        let rows = parse(md);
        let kinds: Vec<Kind> = rows.iter().map(|r| r.kind).collect();
        assert_eq!(
            kinds,
            [Kind::Title, Kind::Text, Kind::Heading, Kind::Text, Kind::Entry, Kind::Entry, Kind::Text]
        );
        assert_eq!(rows[1].text, "Intro line one and two.");
        assert_eq!(rows[2].text, "Guns by Tony");
        assert_eq!(rows[2].link.as_deref(), Some("https://a.example/tony"));
        assert_eq!(rows[3].link.as_deref(), Some("https://cc.example/by"));
        assert_eq!(rows[4].label, "M1911");
        assert_eq!(rows[4].text, "\"Low-Poly M1911\"");
        assert_eq!(rows[4].link.as_deref(), Some("https://a.example/m1911"));
        assert_eq!((rows[5].label.as_str(), rows[5].text.as_str(), rows[5].link.as_ref()), ("Chars", "Pete, Amy", None));
        assert_eq!(rows[6].text, "- a list item");
    }

    #[test]
    fn links_with_quotes_in_the_label() {
        let (text, link) = strip_links("[\"S&W Model 39 \"Hushpuppy\"\"](https://a.example/h) x");
        assert_eq!(text, "\"S&W Model 39 \"Hushpuppy\"\" x");
        assert_eq!(link.as_deref(), Some("https://a.example/h"));
        assert_eq!(strip_links("no [link] here").0, "no [link] here");
    }

    #[test]
    fn unclosed_comment_hides_the_rest() {
        assert_eq!(strip_comments("a<!-- b\nc"), "a");
        assert_eq!(strip_comments("a<!--b-->c<!--d-->e"), "ace");
    }

    // The shipped file: something to show, every row readable in the pixel font,
    // and every link one the game will open.
    #[test]
    fn the_real_credits_read_cleanly() {
        let rows = parse(CREDITS_MD);
        assert!(rows.iter().filter(|r| r.kind == Kind::Heading).count() >= 5);
        assert!(rows.iter().filter(|r| r.kind == Kind::Entry).count() >= 30);
        for row in &rows {
            for s in [&row.label, &row.text] {
                assert!(s.chars().all(|c| c == ' ' || c.is_ascii_graphic()), "not plain ASCII: {s}");
                assert!(!s.contains("](") && !s.contains("<!--"), "markup left in: {s}");
            }
            if let Some(link) = &row.link {
                assert!(link.starts_with("https://") || link.starts_with("http://"), "bad link: {link}");
            }
        }
        // the creators whose licences require it are credited by name
        for name in ["TastyTony", "DJMaesen", "PolyProps3D", "Matrix Rex"] {
            assert!(rows.iter().any(|r| r.kind == Kind::Heading && r.text.contains(name)), "{name} missing");
        }
    }
}
