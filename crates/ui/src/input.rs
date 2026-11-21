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
