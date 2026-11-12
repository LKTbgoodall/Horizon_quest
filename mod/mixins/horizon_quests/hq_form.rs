// Edit mode's quest form: a window over the book with the quest's fields
// (title, subtitle, description, icon, shape, size, visibility, what it
// needs, flags). Text fields are hq_text's; choices cycle on a click; flags
// toggle. Save turns the form into an `Edit::SetQuestInfo` for the server.
// Pure: it lays itself out as hq_book draws and click zones.
use super::hq_book::{Draw, fit_text};
use super::hq_canvas as canvas;
use super::hq_edit::QuestInfo;
use super::hq_model::{DependencyMode, Quest, Shape, Visibility, resource_id};
use super::hq_text::{Key, TextField};
use std::collections::HashMap;

/// The form's text fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Title,
    Subtitle,
    Description,
    Icon,
    Size,
    MinRequired,
    Cooldown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Shape,
    Visibility,
    Needs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flag {
    Optional,
    Sequential,
    Repeatable,
}

/// What a click on the form does.
#[derive(Clone, Debug, PartialEq)]
pub enum FormAction {
    /// Put the cursor in a field, on line `1`, `2` pixels into it.
    Focus(Field, usize, f32),
    Cycle(Choice),
    Toggle(Flag),
    /// Take this item as the quest's icon.
    PickIcon(String),
    Save,
    Cancel,
    /// A click on the form that does nothing.
    Stay,
}

/// One row of the form, top to bottom.
enum Row {
    Text(&'static str, Field),
    Choice(&'static str, Choice),
    Flag(&'static str, Flag),
}

const ROWS: [Row; 13] = [
    Row::Text("Title", Field::Title),
    Row::Text("Subtitle", Field::Subtitle),
    Row::Text("Description", Field::Description),
    Row::Text("Icon (item id)", Field::Icon),
    Row::Choice("Shape", Choice::Shape),
    Row::Text("Size (1 = normal)", Field::Size),
    Row::Choice("Shown", Choice::Visibility),
    Row::Choice("Needs", Choice::Needs),
    Row::Text("How many needed", Field::MinRequired),
    Row::Flag("Optional", Flag::Optional),
    Row::Flag("Tasks in order", Flag::Sequential),
    Row::Flag("Repeatable", Flag::Repeatable),
    Row::Text("Cooldown (seconds)", Field::Cooldown),
];

const SHAPES: [(Option<Shape>, &str); 5] = [
    (None, "the chapter's"),
    (Some(Shape::Square), "square"),
    (Some(Shape::Circle), "circle"),
    (Some(Shape::Diamond), "diamond"),
    (Some(Shape::Hexagon), "hexagon"),
];
const VISIBILITIES: [(Visibility, &str); 4] = [
    (Visibility::Always, "always"),
    (Visibility::UntilDepsVisible, "once a quest it needs shows"),
    (Visibility::UntilDepsComplete, "once the quests it needs are done"),
    (Visibility::Secret, "secret (once done)"),
];
const MODES: [(DependencyMode, &str); 4] = [
    (DependencyMode::AllCompleted, "every quest done"),
    (DependencyMode::OneCompleted, "one quest done"),
    (DependencyMode::AllStarted, "every quest started"),
    (DependencyMode::OneStarted, "one quest started"),
];

/// The form's size and look.
const WIDTH: f32 = 360.0;
const BAR: f32 = 14.0;
const ROW: f32 = 15.0;
const LINE: f32 = 10.0;
const LABEL_W: f32 = 104.0;
const PAD: f32 = 4.0;
const DESCRIPTION_LINES: usize = 4;
const SUGGESTIONS: usize = 6;
const BACK: u32 = 0xFF1C_2230;
const EDGE: u32 = 0xFF3A_4458;
const BAR_BACK: u32 = 0xFF2A_3044;
const BOX: u32 = 0xFF0C_0E14;
const BOX_EDGE: u32 = 0xFF3A_4458;
const BOX_FOCUS: u32 = 0xFF80_90C0;
const BUTTON: u32 = 0xFF3A_5A8A;
const BUTTON_HOVER: u32 = 0xFF4A_74B0;
const HOVER: u32 = 0xFF2A_2A3A;
const INK: u32 = 0xFF_FFFF;
const DIM_INK: u32 = 0xA0_A0B0;
const LABEL_INK: u32 = 0xC8_C8D0;
const ERROR_INK: u32 = 0xFF_6A5A;
const CURSOR: u32 = 0xFFE0_E0E8;
const ON: u32 = 0xFF5C_C83C;

#[derive(Clone, Debug, PartialEq)]
pub struct QuestForm {
    /// The quest's full id.
    pub quest: String,
    title: TextField,
    subtitle: TextField,
    description: TextField,
    icon: TextField,
    size: TextField,
    min_required: TextField,
    cooldown: TextField,
    shape: usize,
    visibility: usize,
    needs: usize,
    optional: bool,
    sequential: bool,
    repeatable: bool,
    pub focus: Option<Field>,
    pub scroll: f32,
    /// Why the last Save did not go.
    pub error: String,
}

impl QuestForm {
    pub fn new(id: &str, quest: &Quest) -> Self {
        let info = QuestInfo::of(quest);
        let number = |n: Option<String>| TextField::new(&n.unwrap_or_default(), 8);
        Self {
            quest: id.to_owned(),
            title: TextField::new(&info.title, 100),
            subtitle: TextField::new(info.subtitle.as_deref().unwrap_or(""), 100),
            description: TextField::lines(&info.description.join("\n"), 2000),
            icon: TextField::new(info.icon.as_deref().unwrap_or(""), 100),
            size: number(info.size.map(|s| s.to_string())),
            min_required: number(info.min_required.map(|n| n.to_string())),
            cooldown: number(Some(info.repeat_cooldown).filter(|&c| c > 0).map(|c| c.to_string())),
            shape: SHAPES.iter().position(|(shape, _)| *shape == info.shape).unwrap_or(0),
            visibility: VISIBILITIES.iter().position(|(v, _)| *v == info.visibility).unwrap_or(0),
            needs: MODES.iter().position(|(m, _)| *m == info.dependency_mode).unwrap_or(0),
            optional: info.optional,
            sequential: info.sequential_tasks,
            repeatable: info.repeatable,
            focus: Some(Field::Title),
            scroll: 0.0,
            error: String::new(),
        }
    }

    fn field(&mut self, field: Field) -> &mut TextField {
        match field {
            Field::Title => &mut self.title,
            Field::Subtitle => &mut self.subtitle,
            Field::Description => &mut self.description,
            Field::Icon => &mut self.icon,
            Field::Size => &mut self.size,
            Field::MinRequired => &mut self.min_required,
            Field::Cooldown => &mut self.cooldown,
        }
    }

    fn field_ref(&self, field: Field) -> &TextField {
        match field {
            Field::Title => &self.title,
            Field::Subtitle => &self.subtitle,
            Field::Description => &self.description,
            Field::Icon => &self.icon,
            Field::Size => &self.size,
            Field::MinRequired => &self.min_required,
            Field::Cooldown => &self.cooldown,
        }
    }

    /// The form's fields as the quest's new info, or what is wrong.
    pub fn info(&self) -> Result<QuestInfo, String> {
        let text = |field: &TextField| field.value.trim().to_owned();
        let optional = |field: &TextField| Some(text(field)).filter(|t| !t.is_empty());
        let number = |field: &TextField, name: &str| -> Result<Option<u32>, String> {
            optional(field).map(|t| t.parse::<u32>().map_err(|_| format!("{name}: a whole number, or nothing."))).transpose()
        };
        let size = optional(&self.size)
            .map(|t| t.parse::<f32>().ok().filter(|s| (0.25..=4.0).contains(s)).ok_or("Size: a number from 0.25 to 4, or nothing."))
            .transpose()?;
        let icon = optional(&self.icon)
            .map(|t| resource_id(&t).ok_or_else(|| format!("Icon: `{t}` is not an item id.")))
            .transpose()?;
        let mut description: Vec<String> = self.description.value.split('\n').map(|line| line.trim_end().to_owned()).collect();
        while description.last().is_some_and(String::is_empty) {
            description.pop();
        }
        Ok(QuestInfo {
            title: text(&self.title),
            subtitle: optional(&self.subtitle),
            description,
            icon,
            shape: SHAPES[self.shape].0,
            size,
            visibility: VISIBILITIES[self.visibility].0,
            dependency_mode: MODES[self.needs].0,
            min_required: number(&self.min_required, "How many needed")?.filter(|&n| n > 0),
            optional: self.optional,
            sequential_tasks: self.sequential,
            repeatable: self.repeatable,
            repeat_cooldown: number(&self.cooldown, "Cooldown")?.unwrap_or(0),
        })
    }

    /// A click on the form. Save and Cancel are the book's to carry out.
    pub fn click(&mut self, action: &FormAction, measure: &dyn Fn(&str) -> f32) {
        match action {
            FormAction::Focus(field, line, x) => {
                self.focus = Some(*field);
                let scroll = self.field_ref(*field).shown(*line, f32::MAX, measure).1;
                self.field(*field).click(*line, *x, scroll, measure);
            }
            FormAction::Cycle(choice) => {
                let (at, len) = match choice {
                    Choice::Shape => (&mut self.shape, SHAPES.len()),
                    Choice::Visibility => (&mut self.visibility, VISIBILITIES.len()),
                    Choice::Needs => (&mut self.needs, MODES.len()),
                };
                *at = (*at + 1) % len;
            }
            FormAction::Toggle(flag) => {
                let at = match flag {
                    Flag::Optional => &mut self.optional,
                    Flag::Sequential => &mut self.sequential,
                    Flag::Repeatable => &mut self.repeatable,
                };
                *at = !*at;
            }
            FormAction::PickIcon(item) => {
                self.icon = TextField::new(item, 100);
                self.focus = None;
            }
            FormAction::Save | FormAction::Cancel | FormAction::Stay => {}
        }
    }

    /// A character typed: into the field with the cursor.
    pub fn typed(&mut self, c: char) {
        if let Some(field) = self.focus {
            self.field(field).type_char(c);
        }
    }

    /// A key pressed while the form is up. Escape leaves the field first,
    /// then the form (`false`: close it). Tab goes to the next field.
    pub fn key(&mut self, name: &str) -> bool {
        let fields: Vec<Field> = ROWS.iter().filter_map(|row| if let Row::Text(_, f) = row { Some(*f) } else { None }).collect();
        match name {
            "key.keyboard.escape" => return self.focus.take().is_some(),
            "key.keyboard.tab" => {
                let next = self.focus.and_then(|f| fields.iter().position(|g| *g == f)).map_or(0, |i| (i + 1) % fields.len());
                self.focus = Some(fields[next]);
            }
            _ => {
                if let (Some(field), Some(key)) = (self.focus, Key::named(name))
                    && !self.field(field).key(key)
                {
                    self.focus = None;
                }
            }
        }
        true
    }

    /// The icon suggestions: items whose id or name holds what is typed,
    /// those starting with it first.
    fn suggestions(&self, names: &HashMap<String, String>) -> Vec<(String, String)> {
        let typed = self.icon.value.trim().to_lowercase();
        if self.focus != Some(Field::Icon) || typed.len() < 2 {
            return Vec::new();
        }
        let mut found: Vec<(bool, &String, &String)> = names
            .iter()
            .filter(|(id, name)| id.contains(&typed) || name.to_lowercase().contains(&typed))
            .map(|(id, name)| {
                let path = id.split_once(':').map_or(id.as_str(), |(_, p)| p);
                (!(path.starts_with(&typed) || name.to_lowercase().starts_with(&typed)), id, name)
            })
            .collect();
        found.sort_by(|a, b| (a.0, a.1.len(), a.1).cmp(&(b.0, b.1.len(), b.1)));
        found.into_iter().take(SUGGESTIONS).map(|(_, id, name)| (id.clone(), name.clone())).collect()
    }

    /// A row's height.
    fn row_h(row: &Row) -> f32 {
        match row {
            Row::Text(_, Field::Description) => DESCRIPTION_LINES as f32 * LINE + 5.0,
            _ => ROW,
        }
    }

    /// How tall the rows are together.
    fn content_h() -> f32 {
        ROWS.iter().map(Self::row_h).sum::<f32>() + 2.0 * PAD
    }

    /// Where the form goes over `area` (the canvas).
    pub fn rect(area: [f32; 4]) -> [f32; 4] {
        let w = WIDTH.min(area[2] - 8.0).max(0.0);
        let h = (Self::content_h() + 2.0 * BAR).min(area[3] - 8.0).max(0.0);
        let [cx, cy] = canvas::centre(area);
        [cx - w / 2.0, cy - h / 2.0, w, h]
    }

    /// How far the rows can scroll in `area`.
    pub fn max_scroll(area: [f32; 4]) -> f32 {
        (Self::content_h() - (Self::rect(area)[3] - 2.0 * BAR)).max(0.0)
    }

    /// The form over `area`: what to draw and where clicks land (later
    /// zones on top). `clock` blinks the cursor.
    pub fn layout(
        &self,
        area: [f32; 4],
        mouse: [f32; 2],
        clock: u32,
        names: &HashMap<String, String>,
        measure: &dyn Fn(&str) -> f32,
    ) -> (Vec<Draw>, Vec<([f32; 4], FormAction)>) {
        let mut draws = Vec::new();
        let mut zones = Vec::new();
        let fill = |draws: &mut Vec<Draw>, rect: [f32; 4], colour: u32| draws.push(Draw::Fill { rect, colour });
        let text = |draws: &mut Vec<Draw>, text: String, at: [f32; 2], colour: u32| {
            if !text.is_empty() {
                draws.push(Draw::Text { text, at, colour, shadow: false });
            }
        };
        let frame = |draws: &mut Vec<Draw>, [x, y, w, h]: [f32; 4], colour: u32| {
            for edge in [[x, y, w, 1.0], [x, y + h - 1.0, w, 1.0], [x, y, 1.0, h], [x + w - 1.0, y, 1.0, h]] {
                draws.push(Draw::Fill { rect: edge, colour });
            }
        };
        let rect @ [x, y, w, h] = Self::rect(area);
        fill(&mut draws, rect, BACK);
        frame(&mut draws, rect, EDGE);
        zones.push((rect, FormAction::Stay));
        fill(&mut draws, [x + 1.0, y + 1.0, w - 2.0, BAR - 1.0], BAR_BACK);
        text(&mut draws, fit_text("Edit quest", w - 2.0 * PAD, measure), [x + PAD, y + 3.0], INK);

        let body = [x, y + BAR, w, h - 2.0 * BAR];
        let field_x = x + PAD + LABEL_W;
        let field_w = (x + w - PAD - field_x).max(0.0);
        let mut row_y = body[1] + PAD - self.scroll;
        let mut icon_box = None;
        for row in &ROWS {
            let row_h = Self::row_h(row);
            let shown = row_y >= body[1] && row_y + row_h <= body[1] + body[3];
            if shown {
                let label = match row {
                    Row::Text(label, _) | Row::Choice(label, _) | Row::Flag(label, _) => *label,
                };
                text(&mut draws, fit_text(label, LABEL_W - PAD, measure), [x + PAD, row_y + 3.0], LABEL_INK);
                let control = [field_x, row_y + 1.0, field_w, row_h - 2.0];
                match row {
                    Row::Text(_, field) => {
                        let value = self.field_ref(*field);
                        let focused = self.focus == Some(*field);
                        // The icon field leaves room for the icon itself.
                        let boxed = if *field == Field::Icon { [control[0], control[1], control[2] - 16.0, control[3]] } else { control };
                        fill(&mut draws, boxed, BOX);
                        frame(&mut draws, boxed, if focused { BOX_FOCUS } else { BOX_EDGE });
                        let inner = boxed[2] - 6.0;
                        let lines = if value.multiline { DESCRIPTION_LINES } else { 1 };
                        // A long description scrolls to keep the cursor's line in sight.
                        let first = value.cursor_line().saturating_sub(lines - 1).min(value.line_count().saturating_sub(1));
                        let first = if focused { first } else { 0 };
                        if value.multiline {
                            // Under the lines (they are on top): the end of the text.
                            zones.push((boxed, FormAction::Focus(*field, value.line_count() - 1, f32::MAX)));
                        }
                        for k in 0..lines {
                            let line = first + k;
                            if line >= value.line_count() {
                                break;
                            }
                            let (part, _, cursor_x) = value.shown(line, inner, measure);
                            let at = [boxed[0] + 3.0, boxed[1] + 2.5 + k as f32 * LINE];
                            text(&mut draws, part, at, INK);
                            if let Some(cx) = cursor_x.filter(|_| focused && (clock / 10) % 2 == 0) {
                                fill(&mut draws, [at[0] + cx, at[1] - 1.0, 1.0, 9.0], CURSOR);
                            }
                            let zone = [boxed[0], boxed[1] + k as f32 * LINE, boxed[2], LINE + 1.0];
                            zones.push((zone, FormAction::Focus(*field, line, mouse[0] - at[0])));
                        }
                        if *field == Field::Icon {
                            icon_box = Some(boxed);
                            if let Some(item) = resource_id(value.value.trim()) {
                                draws.push(Draw::Item { item, at: [boxed[0] + boxed[2] + 2.0, row_y - 0.5], size: 14.0 });
                            }
                        }
                    }
                    Row::Choice(_, choice) => {
                        let label = match choice {
                            Choice::Shape => SHAPES[self.shape].1,
                            Choice::Visibility => VISIBILITIES[self.visibility].1,
                            Choice::Needs => MODES[self.needs].1,
                        };
                        let hovered = canvas::inside(control, mouse);
                        fill(&mut draws, control, if hovered { BUTTON_HOVER } else { BUTTON });
                        text(&mut draws, fit_text(&format!("< {label} >"), control[2] - 6.0, measure), [control[0] + 3.0, control[1] + 3.0], INK);
                        zones.push((control, FormAction::Cycle(*choice)));
                    }
                    Row::Flag(_, flag) => {
                        let on = match flag {
                            Flag::Optional => self.optional,
                            Flag::Sequential => self.sequential,
                            Flag::Repeatable => self.repeatable,
                        };
                        let square = [control[0], control[1] + 1.0, 10.0, 10.0];
                        fill(&mut draws, square, BOX);
                        frame(&mut draws, square, if canvas::inside(square, mouse) { BOX_FOCUS } else { BOX_EDGE });
                        if on {
                            fill(&mut draws, [square[0] + 2.0, square[1] + 2.0, 6.0, 6.0], ON);
                        }
                        text(&mut draws, (if on { "yes" } else { "no" }).to_owned(), [square[0] + 14.0, control[1] + 2.0], if on { INK } else { DIM_INK });
                        zones.push(([control[0], control[1], 40.0, control[3]], FormAction::Toggle(*flag)));
                    }
                }
            }
            row_y += row_h;
        }

        // Bottom bar: the last error, Cancel and Save.
        let bar_y = y + h - BAR;
        fill(&mut draws, [x + 1.0, bar_y, w - 2.0, BAR - 1.0], BAR_BACK);
        let mut right = x + w - PAD;
        for (label, action) in [("Save", FormAction::Save), ("Cancel", FormAction::Cancel)] {
            let bw = measure(label) + 8.0;
            let button = [right - bw, bar_y + 2.0, bw, BAR - 4.0];
            fill(&mut draws, button, if canvas::inside(button, mouse) { BUTTON_HOVER } else { BUTTON });
            text(&mut draws, label.to_owned(), [button[0] + 4.0, button[1] + 1.0], INK);
            zones.push((button, action));
            right = button[0] - PAD;
        }
        let hint = if self.error.is_empty() { "Tab: next field | Esc: leave the field, then the form" } else { &self.error };
        let ink = if self.error.is_empty() { DIM_INK } else { ERROR_INK };
        text(&mut draws, fit_text(hint, right - x - 2.0 * PAD, measure), [x + PAD, bar_y + 3.0], ink);

        // The icon suggestions drop down over the rows below the icon field.
        if let Some(icon_box) = icon_box {
            let found = self.suggestions(names);
            let list_y = icon_box[1] + icon_box[3];
            for (k, (id, name)) in found.iter().enumerate() {
                let row = [icon_box[0], list_y + k as f32 * 13.0, icon_box[2], 13.0];
                fill(&mut draws, row, if canvas::inside(row, mouse) { HOVER } else { BOX });
                draws.push(Draw::Item { item: id.clone(), at: [row[0] + 1.0, row[1] + 0.5], size: 12.0 });
                let label = fit_text(&format!("{name}  {id}"), row[2] - 18.0, measure);
                text(&mut draws, label, [row[0] + 16.0, row[1] + 2.5], INK);
                zones.push((row, FormAction::PickIcon(id.clone())));
            }
            if !found.is_empty() {
                frame(&mut draws, [icon_box[0], list_y, icon_box[2], found.len() as f32 * 13.0], BOX_EDGE);
            }
        }
        (draws, zones)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measure(text: &str) -> f32 {
        text.chars().count() as f32 * 6.0
    }

    fn quest() -> Quest {
        Quest {
            id: "q".into(),
            title: "Old".into(),
            description: vec!["Line one".into(), "Line two".into()],
            dependency_mode: DependencyMode::OneCompleted,
            ..Quest::default()
        }
    }

    #[test]
    fn an_untouched_form_gives_the_quest_back() {
        let q = quest();
        assert_eq!(QuestForm::new("a/q", &q).info(), Ok(QuestInfo::of(&q)));
    }

    #[test]
    fn typing_tab_cycles_and_save_reads_every_field() {
        let mut form = QuestForm::new("a/q", &quest());
        for _ in 0..3 {
            form.key("key.keyboard.backspace");
        }
        "New".chars().for_each(|c| form.typed(c));
        form.key("key.keyboard.tab");
        assert_eq!(form.focus, Some(Field::Subtitle));
        "Sub".chars().for_each(|c| form.typed(c));
        form.key("key.keyboard.tab");
        form.key("key.keyboard.enter");
        form.key("key.keyboard.enter");
        form.key("key.keyboard.tab");
        "stone".chars().for_each(|c| form.typed(c));
        form.click(&FormAction::Cycle(Choice::Shape), &measure);
        form.click(&FormAction::Cycle(Choice::Needs), &measure);
        form.click(&FormAction::Toggle(Flag::Repeatable), &measure);
        let info = form.info().unwrap();
        assert_eq!(info.title, "New");
        assert_eq!(info.subtitle.as_deref(), Some("Sub"));
        // Two new empty lines at the end are dropped.
        assert_eq!(info.description, vec!["Line one", "Line two"]);
        assert_eq!(info.icon.as_deref(), Some("minecraft:stone"));
        assert_eq!(info.shape, Some(Shape::Square));
        assert_eq!(info.dependency_mode, DependencyMode::AllStarted);
        assert!(info.repeatable);
    }

    #[test]
    fn wrong_numbers_and_icons_are_named() {
        let mut form = QuestForm::new("a/q", &quest());
        form.click(&FormAction::Focus(Field::Size, 0, 0.0), &measure);
        form.typed('9');
        assert!(form.info().unwrap_err().starts_with("Size"));
        form.key("key.keyboard.backspace");
        form.click(&FormAction::Focus(Field::Icon, 0, 0.0), &measure);
        "Bad Item".chars().for_each(|c| form.typed(c));
        assert!(form.info().unwrap_err().starts_with("Icon"));
    }

    #[test]
    fn escape_leaves_the_field_then_the_form() {
        let mut form = QuestForm::new("a/q", &quest());
        assert!(form.key("key.keyboard.escape"));
        assert!(!form.key("key.keyboard.escape"));
    }

    #[test]
    fn icon_suggestions_put_names_that_start_with_the_text_first() {
        let names: HashMap<String, String> = [("minecraft:oak_log", "Oak Log"), ("minecraft:stripped_oak_log", "Stripped Oak Log"), ("minecraft:stone", "Stone")]
            .into_iter()
            .map(|(a, b)| (a.to_owned(), b.to_owned()))
            .collect();
        let mut form = QuestForm::new("a/q", &quest());
        form.click(&FormAction::Focus(Field::Icon, 0, 0.0), &measure);
        "oak".chars().for_each(|c| form.typed(c));
        let ids: Vec<String> = form.suggestions(&names).into_iter().map(|(id, _)| id).collect();
        assert_eq!(ids, ["minecraft:oak_log", "minecraft:stripped_oak_log"]);
        let (_, zones) = form.layout([0.0, 0.0, 480.0, 400.0], [-1.0, -1.0], 0, &names, &measure);
        assert!(zones.iter().any(|(_, a)| *a == FormAction::PickIcon("minecraft:oak_log".into())));
        form.click(&FormAction::PickIcon("minecraft:oak_log".into()), &measure);
        assert_eq!(form.info().unwrap().icon.as_deref(), Some("minecraft:oak_log"));
    }
}
