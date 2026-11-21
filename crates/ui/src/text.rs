//! Inline text: the small slice of markdown the transcript needs right now.
//!
//! **Bold** and `code` are the only inline styles in the reference transcript
//! that change weight or background. The real renderer will parse full
//! markdown; this keeps the layout honest until it does.

use gpui::{Div, FontWeight, div, prelude::*, px};

use crate::theme::Theme;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Kind {
    Normal,
    Bold,
    Code,
}

#[derive(Debug, PartialEq, Eq)]
struct Span<'a> {
    kind: Kind,
    text: &'a str,
}

/// Renders one line of inline text. Words are wrapped in a flex row so a line
/// can break between them; the trailing space stays outside a code chip so the
/// chip keeps its shape.
pub(crate) fn inline(text: &str, theme: &Theme) -> Div {
    let mut row = div().flex().flex_wrap();

    for span in parse(text) {
        let mut words = span.text.split_whitespace().peekable();
        while let Some(word) = words.next() {
            let trailing_space = words.peek().is_some();
            match span.kind {
                Kind::Normal => {
                    let word = if trailing_space {
                        format!("{word} ")
                    } else {
                        word.to_owned()
                    };
                    row = row.child(div().child(word));
                }
                Kind::Bold => {
                    let word = if trailing_space {
                        format!("{word} ")
                    } else {
                        word.to_owned()
                    };
                    row = row.child(div().font_weight(FontWeight::SEMIBOLD).child(word));
                }
                Kind::Code => {
                    row = row.child(
                        div()
                            .px(px(4.))
                            .py(px(1.))
                            .rounded(px(4.))
                            .bg(theme.chip)
                            .font_family("Menlo")
                            .text_size(px(12.5))
                            .child(word.to_owned()),
                    );
                    if trailing_space {
                        row = row.child(div().child(" "));
                    }
                }
            }
        }
    }

    row
}

/// Splits `**bold**` and `` `code` `` out of a line. An opening marker without
/// a closing one stays literal text instead of swallowing the rest of the line.
fn parse(text: &str) -> Vec<Span<'_>> {
    let mut spans = Vec::new();
    let bytes = text.as_bytes();
    let mut start = 0;
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i..].starts_with(b"**") {
            if let Some(close) = find(bytes, i + 2, b"**") {
                if i > start {
                    spans.push(Span {
                        kind: Kind::Normal,
                        text: &text[start..i],
                    });
                }
                spans.push(Span {
                    kind: Kind::Bold,
                    text: &text[i + 2..close],
                });
                i = close + 2;
                start = i;
                continue;
            }
            // No closing marker: keep the asterisks as text.
            i += 2;
            continue;
        }

        if bytes[i] == b'`' {
            if let Some(close) = find(bytes, i + 1, b"`") {
                if i > start {
                    spans.push(Span {
                        kind: Kind::Normal,
                        text: &text[start..i],
                    });
                }
                spans.push(Span {
                    kind: Kind::Code,
                    text: &text[i + 1..close],
                });
                i = close + 1;
                start = i;
                continue;
            }
        }

        i += 1;
    }

    if start < bytes.len() {
        spans.push(Span {
            kind: Kind::Normal,
            text: &text[start..],
        });
    }

    spans
}

fn find(haystack: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    if from > haystack.len() {
        return None;
    }
    haystack[from..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| offset + from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_is_one_span() {
        assert_eq!(
            parse("one two"),
            vec![Span {
                kind: Kind::Normal,
                text: "one two"
            }]
        );
    }

    #[test]
    fn splits_bold_and_code() {
        assert_eq!(
            parse("a **b** `c` d"),
            vec![
                Span {
                    kind: Kind::Normal,
                    text: "a "
                },
                Span {
                    kind: Kind::Bold,
                    text: "b"
                },
                Span {
                    kind: Kind::Normal,
                    text: " "
                },
                Span {
                    kind: Kind::Code,
                    text: "c"
                },
                Span {
                    kind: Kind::Normal,
                    text: " d"
                },
            ]
        );
    }

    #[test]
    fn unclosed_marker_stays_text() {
        assert_eq!(
            parse("a ** b"),
            vec![Span {
                kind: Kind::Normal,
                text: "a ** b"
            }]
        );
    }
}
