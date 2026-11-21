//! The composer's text buffer.
//!
//! GPUI ships no text input element, so the buffer, the caret and the editing
//! keys are ours. This module is pure state with no rendering: the composer
//! paints it, the shell handles the keys.
//!
//! Deliberately small for the first slice: a byte caret that always sits on a
//! character boundary, insertion, deletion on both sides, and horizontal
//! movement. No selection, no undo, no IME composition, no wrapping-aware
//! vertical movement. Those come with the editor work, not with sending a
//! prompt.

use gpui::{
    AnyElement, Font, FontFeatures, FontStyle, FontWeight, IntoElement, StyledText, TextRun, div,
    prelude::*, px,
};

use crate::theme::Theme;

pub(crate) struct TextInput {
    text: String,
    /// Byte offset into `text`, always on a character boundary.
    caret: usize,
}

impl Default for TextInput {
    fn default() -> Self {
        Self::new()
    }
}

impl TextInput {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            caret: 0,
        }
    }

    /// A buffer that already contains a draft (the development prompt).
    pub fn with_text(text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            caret: text.len(),
            text,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn caret(&self) -> usize {
        self.caret
    }

    pub fn is_blank(&self) -> bool {
        self.text.trim().is_empty()
    }

    /// Inserts at the caret and leaves it after the inserted text.
    pub fn insert(&mut self, text: &str) {
        self.text.insert_str(self.caret, text);
        self.caret += text.len();
    }

    /// Removes the character before the caret.
    pub fn backspace(&mut self) {
        let Some(previous) = self.text[..self.caret].chars().next_back() else {
            return;
        };
        self.caret -= previous.len_utf8();
        self.text.remove(self.caret);
    }

    /// Removes the character after the caret.
    pub fn delete(&mut self) {
        if self.caret < self.text.len() {
            self.text.remove(self.caret);
        }
    }

    pub fn left(&mut self) {
        if let Some(previous) = self.text[..self.caret].chars().next_back() {
            self.caret -= previous.len_utf8();
        }
    }

    pub fn right(&mut self) {
        if let Some(next) = self.text[self.caret..].chars().next() {
            self.caret += next.len_utf8();
        }
    }

    pub fn home(&mut self) {
        self.caret = 0;
    }

    pub fn end(&mut self) {
        self.caret = self.text.len();
    }

    /// The text, emptied. What sending does.
    pub fn take(&mut self) -> String {
        let text = std::mem::take(&mut self.text);
        self.caret = 0;
        text
    }
}

/// The field as the window paints it: the text with a block caret behind the
/// character it sits on, or the placeholder when empty and not focused.
///
/// Shared by the composer and by a question card's text answer, so both read
/// the same.
pub(crate) fn field(
    theme: &Theme,
    input: &TextInput,
    focused: bool,
    placeholder: &str,
) -> AnyElement {
    if input.text().is_empty() {
        if focused {
            return div()
                .h(px(17.))
                .w(px(1.5))
                .bg(theme.text)
                .into_any_element();
        }
        return div()
            .text_color(theme.text_faint)
            .child(placeholder.to_owned())
            .into_any_element();
    }

    let text = input.text();
    let caret = input.caret().min(text.len());
    let mut styled = Vec::new();
    let mut runs = Vec::new();

    if caret > 0 {
        styled.push(text[..caret].to_owned());
        runs.push(run(&text[..caret], theme.text, None));
    }

    if focused {
        // Highlight the character the caret sits on; at the end, a space.
        let next = text[caret..].chars().next();
        let marked: String = match next {
            Some(character) => character.to_string(),
            None => " ".to_owned(),
        };
        styled.push(marked.clone());
        runs.push(run(&marked, theme.canvas, Some(theme.accent)));

        if next.is_some() {
            let rest = &text[caret + marked.len()..];
            if !rest.is_empty() {
                styled.push(rest.to_owned());
                runs.push(run(rest, theme.text, None));
            }
        }
    } else if caret < text.len() {
        styled.push(text[caret..].to_owned());
        runs.push(run(&text[caret..], theme.text, None));
    }

    StyledText::new(styled.concat())
        .with_runs(runs)
        .into_any_element()
}

fn run(text: &str, color: gpui::Rgba, background: Option<gpui::Rgba>) -> TextRun {
    TextRun {
        len: text.len(),
        font: Font {
            family: ".SystemUIFont".into(),
            features: FontFeatures::default(),
            fallbacks: None,
            weight: FontWeight::NORMAL,
            style: FontStyle::Normal,
        },
        color: color.into(),
        background_color: background.map(Into::into),
        underline: None,
        strikethrough: None,
    }
}

#[cfg(test)]
mod tests {
    use super::TextInput;

    #[test]
    fn typing_inserts_at_the_caret() {
        let mut input = TextInput::new();
        input.insert("hello");
        input.insert(" world");
        assert_eq!(input.text(), "hello world");
        assert_eq!(input.caret(), "hello world".len());
    }

    #[test]
    fn backspace_removes_a_whole_character() {
        let mut input = TextInput::with_text("añ");
        input.backspace();
        assert_eq!(input.text(), "a", "ñ is two bytes, one character");
        assert_eq!(input.caret(), 1);
    }

    #[test]
    fn deletion_works_on_both_sides_of_the_caret() {
        let mut input = TextInput::with_text("abc");
        input.home();
        input.delete();
        assert_eq!(input.text(), "bc");
        input.insert("a");
        assert_eq!(input.text(), "abc");
        input.backspace();
        assert_eq!(input.text(), "bc");
    }

    #[test]
    fn the_caret_stops_at_the_edges() {
        let mut input = TextInput::with_text("ab");
        input.home();
        input.left();
        assert_eq!(input.caret(), 0);
        input.end();
        input.right();
        assert_eq!(input.caret(), 2);
    }

    #[test]
    fn movement_counts_characters_not_bytes() {
        let mut input = TextInput::with_text("añb");
        input.end();
        input.left();
        assert_eq!(input.caret(), 3, "b is one byte after ñ");
        input.left();
        assert_eq!(input.caret(), 1, "skip the two bytes of ñ");
    }

    #[test]
    fn sending_empties_the_buffer() {
        let mut input = TextInput::with_text("prompt");
        assert_eq!(input.take(), "prompt");
        assert!(input.text().is_empty());
        assert!(input.is_blank());
        assert_eq!(input.caret(), 0);
    }

    #[test]
    fn whitespace_counts_as_blank() {
        assert!(TextInput::with_text("   \n ").is_blank());
        assert!(!TextInput::with_text(" a ").is_blank());
    }
}
