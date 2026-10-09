//! Marked-up text made plain: the words and the line breaks, without the
//! tags.

/// The words of some HTML-like markup, as Flash text fields hold it: tags
/// dropped, paragraphs and line breaks turned into new lines, and the common
/// entities turned back into their characters.
pub(super) fn plain_text(markup: &str) -> String {
    let mut out = String::new();
    let mut rest = markup;
    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..open]);
        let Some(close) = rest[open..].find('>') else {
            // An unfinished tag: keep it as written.
            out.push_str(&rest[open..]);
            rest = "";
            break;
        };
        let tag = rest[open + 1..open + close].trim().to_ascii_lowercase();
        let name = tag.trim_start_matches('/').split_whitespace().next();
        // A line ends at a break, and at the end of a paragraph.
        if matches!(name, Some("br" | "br/")) || tag.starts_with("/p") {
            out.push('\n');
        }
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    let out = [
        ("&nbsp;", " "),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&apos;", "'"),
        ("&amp;", "&"),
    ]
    .iter()
    .fold(out, |text, (entity, character)| {
        text.replace(entity, character)
    });
    out.trim_end_matches('\n').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markup_becomes_plain_words_and_lines() {
        let markup = r#"<p align="left"><font face="Arial" size="12">Top &amp; <b>bold</b></font></p><p>Next<br>line</p>"#;
        assert_eq!(plain_text(markup), "Top & bold\nNext\nline");
        assert_eq!(plain_text("no tags at all"), "no tags at all");
        assert_eq!(plain_text("a < b"), "a < b");
    }

    #[test]
    fn a_line_ends_at_a_break_however_it_is_written_and_at_the_end_of_a_paragraph() {
        assert_eq!(
            plain_text("one<br>two<BR/>three<br />four"),
            "one\ntwo\nthree\nfour"
        );
        assert_eq!(plain_text("<P>one</P><p>two</p>"), "one\ntwo");
        // Empty lines are kept at the start and in the middle, and dropped
        // from the end.
        assert_eq!(plain_text("<br>one<br><br>two<br><br>"), "\none\n\ntwo");
        // Any other tag is dropped and leaves the line as it was.
        assert_eq!(plain_text("<b>one</b> <i>line</i>"), "one line");
    }

    #[test]
    fn what_stands_for_a_character_becomes_it_once_the_tags_are_gone() {
        assert_eq!(
            plain_text("&lt;b&gt; &quot;hi&quot; &apos;there&apos;&nbsp;you"),
            "<b> \"hi\" 'there' you"
        );
        // An ampersand that was written out is turned back once and no more.
        assert_eq!(plain_text("&amp;lt;"), "&lt;");
        assert_eq!(plain_text("this &amp; that"), "this & that");
    }

    #[test]
    fn a_tag_that_is_never_closed_is_kept_as_it_was_written() {
        assert_eq!(plain_text("<b>bold</b> and <i"), "bold and <i");
        assert_eq!(plain_text("<"), "<");
        assert_eq!(plain_text(""), "");
    }
}
