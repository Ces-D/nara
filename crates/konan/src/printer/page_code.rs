use std::{collections::HashSet, sync::LazyLock};

/// Extended CP437 characters (non-ASCII) for O(1) lookup
static EXTENDED_CP437: LazyLock<HashSet<char>> = LazyLock::new(|| {
    CP437_CHARS
        .iter()
        .copied()
        .filter(|ch| !ch.is_ascii())
        .collect()
});

/// Extended PC850 characters (non-ASCII) for O(1) lookup
static EXTENDED_PC850: LazyLock<HashSet<char>> = LazyLock::new(|| {
    PC850_CHARS
        .iter()
        .copied()
        .filter(|ch| !ch.is_ascii())
        .collect()
});

#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum CharPageCode {
    #[default]
    Pc437,
    Pc850,
}

impl From<CharPageCode> for escpos::utils::PageCode {
    fn from(value: CharPageCode) -> Self {
        match value {
            CharPageCode::Pc437 => escpos::utils::PageCode::PC437,
            CharPageCode::Pc850 => escpos::utils::PageCode::PC850,
        }
    }
}

/// Returns the page code required to print `ch`, preferring PC437 for characters
/// that exist in both tables. Returns `None` if the character is not supported.
pub fn char_page_code(ch: char) -> Option<CharPageCode> {
    if ch.is_ascii() {
        return Some(CharPageCode::Pc437);
    }
    if EXTENDED_CP437.contains(&ch) {
        return Some(CharPageCode::Pc437);
    }
    if EXTENDED_PC850.contains(&ch) {
        return Some(CharPageCode::Pc850);
    }
    None
}

/// All valid PC850 characters (extended range 0x80–0xFF)
pub const PC850_CHARS: [char; 128] = [
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å', 'É', 'æ', 'Æ',
    'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', 'ø', '£', 'Ø', '×', 'ƒ', 'á', 'í', 'ó', 'ú', 'ñ', 'Ñ',
    'ª', 'º', '¿', '®', '¬', '½', '¼', '¡', '«', '»', '░', '▒', '▓', '│', '┤', 'Á', 'Â', 'À', '©',
    '╣', '║', '╗', '╝', '¢', '¥', '┐', '└', '┴', '┬', '├', '─', '┼', 'ã', 'Ã', '╚', '╔', '╩', '╦',
    '╠', '═', '╬', '¤', 'ð', 'Ð', 'Ê', 'Ë', 'È', 'ı', 'Í', 'Î', 'Ï', '┘', '┌', '█', '▄', '¦', 'Ì',
    '▀', 'Ó', 'ß', 'Ô', 'Ò', 'õ', 'Õ', 'µ', 'þ', 'Þ', 'Ú', 'Û', 'Ù', 'ý', 'Ý', '¯', '´', '-', '±',
    '‗', '¾', '¶', '§', '÷', '¸', '°', '¨', '·', '¹', '³', '²', '■', '\u{00A0}',
];

/// All valid CP437 characters mapped to their Unicode equivalents
pub const CP437_CHARS: [char; 128] = [
    // 0x20-0x2F (standard ASCII)
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å', 'É', 'æ', 'Æ',
    'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', '¢', '£', '¥', '₧', 'ƒ', 'á', 'í', 'ó', 'ú', 'ñ', 'Ñ',
    'ª', 'º', '¿', '⌐', '¬', '½', '¼', '¡', '«', '»', '░', '▒', '▓', '│', '┤', '╡', '╢', '╖', '╕',
    '╣', '║', '╗', '╝', '╜', '╛', '┐', '└', '┴', '┬', '├', '─', '┼', '╞', '╟', '╚', '╔', '╩', '╦',
    '╠', '═', '╬', '╧', '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '┘', '┌', '█', '▄', '▌', '▐',
    '▀', 'α', 'ß', 'Γ', 'π', 'Σ', 'σ', 'µ', 'τ', 'Φ', 'Θ', 'Ω', 'δ', '∞', 'φ', 'ε', '∩', '≡', '±',
    '≥', '≤', '⌠', '⌡', '÷', '≈', '°', '∙', '·', '√', 'ⁿ', '²', '■', '\u{00A0}',
];

/// Normalize a single Unicode typographic character to its ASCII equivalent.
/// Returns the ASCII equivalent if applicable, otherwise returns None.
pub fn normalize_char(ch: char) -> Option<char> {
    match ch {
        // Curly apostrophes → straight apostrophe
        '\u{2018}' | '\u{2019}' | '\u{02BC}' => Some('\''),
        // Curly double quotes → straight double quote
        '\u{201C}' | '\u{201D}' => Some('"'),
        // En-dash, em-dash → hyphen-minus
        '\u{2013}' | '\u{2014}' => Some('-'),
        // Left arrow → less-than
        '\u{2190}' => Some('<'),
        // Right arrow → greater-than
        '\u{2192}' => Some('>'),
        _ => None,
    }
}
