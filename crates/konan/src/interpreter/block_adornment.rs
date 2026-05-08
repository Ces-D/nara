use crate::printer::{Justification, RongtaPrinter, TextSize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum OrderedListType {
    LowerCaseLetter,
    UpperCaseLetter,
    LowerCaseRoman,
    UpperCaseRoman,
    #[default]
    Number,
}
impl From<&str> for OrderedListType {
    fn from(value: &str) -> Self {
        match value {
            "a" => Self::LowerCaseLetter,
            "A" => Self::UpperCaseLetter,
            "i" => Self::LowerCaseRoman,
            "I" => Self::UpperCaseRoman,
            _ => Self::Number,
        }
    }
}

/// Style the ListItem ::before pseudoelement
#[derive(Debug, Clone)]
pub struct ListItemBefore {
    ordinal: Option<OrderedListType>,
    content: String,
    text_size: TextSize,
    bold: bool,
}
impl ListItemBefore {
    pub fn new_ordered(ordinal: Option<OrderedListType>) -> Self {
        Self {
            content: "".to_string(),
            ordinal,
            text_size: TextSize::Medium,
            bold: true,
        }
    }
    pub fn new_unordered() -> Self {
        Self {
            content: "∙ ".to_string(),
            text_size: TextSize::Medium,
            bold: true,
            ordinal: None,
        }
    }
    fn ordered_before_content(index: u64, ordinal: &Option<OrderedListType>) -> String {
        let value = match ordinal.clone().unwrap_or_default() {
            OrderedListType::LowerCaseLetter => Self::letter_for_index(index, false),
            OrderedListType::UpperCaseLetter => Self::letter_for_index(index, true),
            OrderedListType::LowerCaseRoman => Self::roman_numeral(index, false),
            OrderedListType::UpperCaseRoman => Self::roman_numeral(index, true),
            OrderedListType::Number => index.to_string(),
        };
        format!("{}. ", value)
    }
    pub fn next_index(&mut self, index: u64) {
        self.content = Self::ordered_before_content(index, &self.ordinal);
    }
    /// Returns the alphabetic label for a 1-based index.
    /// Examples: 1 -> "a"/"A", 26 -> "z"/"Z", 27 -> "aa"/"AA".
    fn letter_for_index(index: u64, uppercase: bool) -> String {
        if index == 0 {
            return String::new();
        }
        let mut n = index;
        let mut s = String::new();
        while n > 0 {
            let rem = ((n - 1) % 26) as u8;
            let base = if uppercase { b'A' } else { b'a' };
            s.insert(0, (base + rem) as char);
            n = (n - 1) / 26;
        }
        s
    }
    /// Returns the Roman numeral for a positive integer (1..=3999).
    /// Set `uppercase` to control casing (e.g., 4 -> "iv" or "IV").
    fn roman_numeral(value: u64, uppercase: bool) -> String {
        if value == 0 || value > 3999 {
            return String::new();
        }
        let mut n = value;
        let vals: [u64; 13] = [1000, 900, 500, 400, 100, 90, 50, 40, 10, 9, 5, 4, 1];
        let syms: [&str; 13] = [
            "M", "CM", "D", "CD", "C", "XC", "L", "XL", "X", "IX", "V", "IV", "I",
        ];
        let mut out = String::new();
        for (i, &v) in vals.iter().enumerate() {
            while n >= v {
                out.push_str(syms[i]);
                n -= v;
            }
        }
        if uppercase { out } else { out.to_lowercase() }
    }
}

pub fn list_item_before_print_command(item: ListItemBefore, printer: &mut RongtaPrinter) {
    printer.set_line_justification(Justification::Start);
    printer.set_cached_text_size(item.text_size);
    printer.set_cached_bold(item.bold);
    printer.add_content(&item.content)
}

pub struct TaskListBefore {
    content: String,
    text_size: TextSize,
    bold: bool,
}
impl TaskListBefore {
    pub fn new(checked: bool) -> Self {
        let content = if checked {
            "[■] ".to_string()
        } else {
            "[ ] ".to_string()
        };
        Self {
            content,
            text_size: TextSize::Medium,
            bold: true,
        }
    }
}

pub fn task_list_before_print_command(item: TaskListBefore, printer: &mut RongtaPrinter) {
    printer.set_cached_text_size(item.text_size);
    printer.set_cached_bold(item.bold);
    printer.add_content(&item.content)
}

pub struct HorizontalRule {
    content: String,
    text_size: TextSize,
    bold: bool,
}
impl HorizontalRule {
    pub fn new() -> Self {
        Self {
            content: "-".repeat(12),
            text_size: TextSize::Large,
            bold: true,
        }
    }
}

pub fn horizontal_rule_print_command(item: HorizontalRule, printer: &mut RongtaPrinter) {
    printer.add_new_line();
    printer.set_cached_text_size(item.text_size);
    printer.set_cached_bold(item.bold);
    printer.set_line_justification(Justification::Center);
    printer.add_content(&item.content);
    printer.add_new_line();
}

pub fn set_heading_style(level: u8, printer: &mut RongtaPrinter) {
    match level {
        1 => {
            printer.set_cached_text_size(TextSize::ExtraLarge);
            printer.set_cached_bold(true);
        }
        2 => {
            printer.set_cached_text_size(TextSize::Large);
            printer.set_cached_bold(true);
        }
        3 => {
            printer.set_cached_text_size(TextSize::Large);
            printer.set_cached_bold(false);
        }
        _ => {
            printer.set_cached_text_size(TextSize::Medium);
            printer.set_cached_bold(true);
        }
    };
}
