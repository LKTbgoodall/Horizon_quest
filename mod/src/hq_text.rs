// A text field of our own, for the book's edit forms: the game lends no text
// box to a mod screen, so the screen's character hook types into this and
// its key hook moves the cursor (`native_screen_char_typed_pre`,
// `native_screen_key_pressed_pre`). The cursor counts characters, not bytes.
// A field with several lines takes Enter as a new line. Pure: no game types.

/// A key a text field acts on, from the game's key names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Enter,
}

impl Key {
    /// The game's name for a key (`key.keyboard.left`) as a field key.
    pub fn named(name: &str) -> Option<Key> {
        Some(match name.strip_prefix("key.keyboard.")? {
            "backspace" => Key::Backspace,
            "delete" => Key::Delete,
            "left" => Key::Left,
            "right" => Key::Right,
            "up" => Key::Up,
            "down" => Key::Down,
            "home" => Key::Home,
            "end" => Key::End,
            "enter" | "keypad.enter" => Key::Enter,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextField {
    pub value: String,
    /// Where the cursor is: before character `cursor`.
    pub cursor: usize,
    pub multiline: bool,
    pub max_length: usize,
}

impl TextField {
    pub fn new(value: &str, max_length: usize) -> Self {
        let value: String = value.chars().take(max_length).collect();
        Self { cursor: value.chars().count(), value, multiline: false, max_length }
    }

    pub fn lines(value: &str, max_length: usize) -> Self {
        Self { multiline: true, ..Self::new(value, max_length) }
    }

    fn len(&self) -> usize {
        self.value.chars().count()
    }

    /// The byte where character `at` starts.
    fn byte(&self, at: usize) -> usize {
        self.value.char_indices().nth(at).map_or(self.value.len(), |(i, _)| i)
    }

    /// A character typed. Control characters are keys, not text.
    pub fn type_char(&mut self, c: char) {
        if c.is_control() || self.len() >= self.max_length {
            return;
        }
        let at = self.byte(self.cursor);
        self.value.insert(at, c);
        self.cursor += 1;
    }

    /// The (line, column) of the cursor, and each line's length.
    fn place(&self) -> (usize, usize, Vec<usize>) {
        let lengths: Vec<usize> = self.value.split('\n').map(|line| line.chars().count()).collect();
        let mut left = self.cursor;
        for (line, &len) in lengths.iter().enumerate() {
            if left <= len {
                return (line, left, lengths);
            }
            left -= len + 1;
        }
        let last = lengths.len() - 1;
        (last, lengths[last], lengths)
    }

    /// The cursor at column `column` of line `line` (or its end).
    fn go_to(&mut self, line: usize, column: usize, lengths: &[usize]) {
        let before: usize = lengths[..line].iter().map(|len| len + 1).sum();
        self.cursor = before + column.min(lengths[line]);
    }

    /// A key pressed. Returns false for Enter in a one-line field, which the
    /// form takes (leaving the field).
    pub fn key(&mut self, key: Key) -> bool {
        let len = self.len();
        match key {
            Key::Backspace if self.cursor > 0 => {
                let at = self.byte(self.cursor - 1);
                self.value.remove(at);
                self.cursor -= 1;
            }
            Key::Delete if self.cursor < len => {
                let at = self.byte(self.cursor);
                self.value.remove(at);
            }
            Key::Left => self.cursor = self.cursor.saturating_sub(1),
            Key::Right => self.cursor = (self.cursor + 1).min(len),
            Key::Home | Key::End => {
                let (line, _, lengths) = self.place();
                let column = if key == Key::Home { 0 } else { lengths[line] };
                self.go_to(line, column, &lengths);
            }
            Key::Up | Key::Down => {
                let (line, column, lengths) = self.place();
                let to = if key == Key::Up { line.checked_sub(1) } else { Some(line + 1).filter(|&l| l < lengths.len()) };
                if let Some(to) = to {
                    self.go_to(to, column, &lengths);
                }
            }
            Key::Enter if self.multiline => self.type_char_unchecked('\n'),
            Key::Enter => return false,
            Key::Backspace | Key::Delete => {}
        }
        true
    }

    fn type_char_unchecked(&mut self, c: char) {
        if self.len() < self.max_length {
            let at = self.byte(self.cursor);
            self.value.insert(at, c);
            self.cursor += 1;
        }
    }

    /// The cursor put where a click `x` pixels into line `line` lands.
    pub fn click(&mut self, line: usize, x: f32, scroll: usize, measure: &dyn Fn(&str) -> f32) {
        let (_, _, lengths) = self.place();
        let line = line.min(lengths.len() - 1);
        let text: Vec<char> = self.value.split('\n').nth(line).unwrap_or("").chars().collect();
        let start = if line == self.place().0 { scroll.min(text.len()) } else { 0 };
        let mut column = text.len();
        for k in start..text.len() {
            let left: String = text[start..k].iter().collect();
            let with: String = text[start..=k].iter().collect();
            if x < (measure(&left) + measure(&with)) / 2.0 {
                column = k;
                break;
            }
        }
        self.go_to(line, column, &lengths);
    }

    /// Line `line` as it fits in `width` pixels: the part shown, the first
    /// character shown, and the cursor's x in it when the cursor is on this
    /// line. The cursor's line scrolls to keep the cursor in sight; other
    /// lines show their start.
    pub fn shown(&self, line: usize, width: f32, measure: &dyn Fn(&str) -> f32) -> (String, usize, Option<f32>) {
        let (cursor_line, column, _) = self.place();
        let text: Vec<char> = self.value.split('\n').nth(line).unwrap_or("").chars().collect();
        let fits = |from: usize, to: usize| measure(&text[from..to].iter().collect::<String>()) <= width;
        let mut start = 0;
        if line == cursor_line {
            while start < column && !fits(start, column) {
                start += 1;
            }
        }
        let mut end = start;
        while end < text.len() && fits(start, end + 1) {
            end += 1;
        }
        let shown: String = text[start..end].iter().collect();
        let cursor_x = (line == cursor_line).then(|| measure(&text[start..column].iter().collect::<String>()));
        (shown, start, cursor_x)
    }

    /// The text cut into lines `width` wide: each line's first and last
    /// character (end excluded). A new line ends a line; a long line breaks
    /// after a space where it can, else where it must.
    pub fn wrapped(&self, width: f32, measure: &dyn Fn(&str) -> f32) -> Vec<(usize, usize)> {
        let chars: Vec<char> = self.value.chars().collect();
        let wide = |from: usize, to: usize| measure(&chars[from..to].iter().collect::<String>());
        let mut lines = Vec::new();
        let mut start = 0;
        loop {
            let end = (start..chars.len()).find(|&i| chars[i] == '\n').unwrap_or(chars.len());
            let mut from = start;
            while end > from && wide(from, end) > width {
                // The most that fits, then back to just after a space in it.
                let mut cut = from + 1;
                while cut < end && wide(from, cut + 1) <= width {
                    cut += 1;
                }
                if let Some(space) = (from + 1..cut).rev().find(|&i| chars[i - 1] == ' ') {
                    cut = space;
                }
                lines.push((from, cut));
                from = cut;
            }
            lines.push((from, end));
            if end >= chars.len() {
                return lines;
            }
            start = end + 1;
        }
    }

    /// The line of `lines` (from `wrapped`) the cursor is on, and how far
    /// into it.
    pub fn cursor_in(&self, lines: &[(usize, usize)], measure: &dyn Fn(&str) -> f32) -> (usize, f32) {
        let c = self.cursor;
        let line = (0..lines.len())
            .find(|&i| {
                let (start, end) = lines[i];
                start <= c && (c < end || (c == end && lines.get(i + 1).is_none_or(|next| next.0 != end)))
            })
            .unwrap_or(lines.len().saturating_sub(1));
        let start = lines.get(line).map_or(0, |l| l.0);
        let before: String = self.value.chars().skip(start).take(c.saturating_sub(start)).collect();
        (line, measure(&before))
    }

    /// The cursor put `x` pixels into line `line` of `lines`.
    pub fn place_in(&mut self, lines: &[(usize, usize)], line: usize, x: f32, measure: &dyn Fn(&str) -> f32) {
        let Some(&(start, end)) = lines.get(line.min(lines.len().saturating_sub(1))) else {
            return;
        };
        let chars: Vec<char> = self.value.chars().collect();
        let mut at = end;
        for k in start..end {
            let left: String = chars[start..k].iter().collect();
            let with: String = chars[start..=k].iter().collect();
            if x < (measure(&left) + measure(&with)) / 2.0 {
                at = k;
                break;
            }
        }
        // A line broken after a space: its end is the next line's start.
        if at == end && lines.get(line + 1).is_some_and(|next| next.0 == end) && end > start {
            at = end - 1;
        }
        self.cursor = at;
    }

    /// Up or Down by a line of `lines`, keeping the cursor's x.
    pub fn step_line(&mut self, lines: &[(usize, usize)], down: bool, measure: &dyn Fn(&str) -> f32) {
        let (line, x) = self.cursor_in(lines, measure);
        let to = if down { Some(line + 1).filter(|&l| l < lines.len()) } else { line.checked_sub(1) };
        if let Some(to) = to {
            self.place_in(lines, to, x, measure);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measure(text: &str) -> f32 {
        text.chars().count() as f32 * 6.0
    }

    #[test]
    fn typing_and_deleting_count_characters_not_bytes() {
        let mut field = TextField::new("été", 10);
        assert_eq!(field.cursor, 3);
        field.key(Key::Left);
        field.type_char('x');
        assert_eq!(field.value, "étxé");
        field.key(Key::Backspace);
        field.key(Key::Home);
        field.key(Key::Delete);
        assert_eq!((field.value.as_str(), field.cursor), ("té", 0));
        field.type_char('\u{7}');
        assert_eq!(field.value, "té");
    }

    #[test]
    fn a_field_stops_at_its_length_and_one_line_fields_give_enter_back() {
        let mut field = TextField::new("abcdef", 4);
        assert_eq!(field.value, "abcd");
        field.type_char('e');
        assert_eq!(field.value, "abcd");
        assert!(!field.key(Key::Enter));
        let mut lines = TextField::lines("ab", 10);
        assert!(lines.key(Key::Enter));
        lines.type_char('c');
        assert_eq!(lines.value, "ab\nc");
    }

    #[test]
    fn up_and_down_keep_the_column() {
        let mut field = TextField::lines("hello\nhi\nworld", 100);
        assert_eq!(field.cursor, 14);
        field.key(Key::Up);
        assert_eq!(field.cursor, 8);
        field.key(Key::Up);
        assert_eq!(field.cursor, 2);
        field.key(Key::End);
        assert_eq!(field.cursor, 5);
        // Down twice: "hi" has two columns, so the cursor ends at column 2.
        field.key(Key::Down);
        field.key(Key::Down);
        assert_eq!(field.cursor, 11);
        field.key(Key::Home);
        assert_eq!(field.cursor, 9);
    }

    #[test]
    fn a_long_line_scrolls_to_the_cursor_and_clicks_land_between_characters() {
        let mut field = TextField::new("abcdefghij", 100);
        let (shown, start, x) = field.shown(0, 30.0, &measure);
        assert_eq!((shown.as_str(), start, x), ("fghij", 5, Some(30.0)));
        field.key(Key::Home);
        assert_eq!(field.shown(0, 30.0, &measure), ("abcde".to_owned(), 0, Some(0.0)));
        field.click(0, 14.0, 0, &measure);
        assert_eq!(field.cursor, 2);
        field.click(0, 1000.0, 0, &measure);
        assert_eq!(field.cursor, 10);
    }

    #[test]
    fn long_lines_wrap_after_spaces_and_the_cursor_follows() {
        // 6 pixels a character, 30 wide: 5 characters a line.
        let mut field = TextField::lines("ab cd efgh\nijklmnop", 100);
        let lines = field.wrapped(30.0, &measure);
        assert_eq!(lines, [(0, 3), (3, 6), (6, 10), (11, 16), (16, 19)]);
        assert_eq!(field.cursor_in(&lines, &measure), (4, 18.0));
        field.step_line(&lines, false, &measure);
        assert_eq!(field.cursor, 14);
        field.step_line(&lines, false, &measure);
        assert_eq!(field.cursor_in(&lines, &measure).0, 2);
        field.place_in(&lines, 0, 100.0, &measure);
        assert_eq!(field.cursor, 2);
        assert_eq!(TextField::lines("", 10).wrapped(30.0, &measure), [(0, 0)]);
    }

    #[test]
    fn key_names() {
        assert_eq!(Key::named("key.keyboard.backspace"), Some(Key::Backspace));
        assert_eq!(Key::named("key.keyboard.keypad.enter"), Some(Key::Enter));
        assert_eq!(Key::named("key.keyboard.e"), None);
    }
}
