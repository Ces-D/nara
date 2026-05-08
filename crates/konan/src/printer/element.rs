use crate::printer::AnyPrinter;
use escpos::utils::JustifyMode;

/// characters per line
pub const CPL: u8 = 48;

pub trait PrintCommand {
    fn command(&self, printer: &mut AnyPrinter) -> escpos::errors::Result<()>;
}

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub enum TextSize {
    #[default]
    Medium,
    Large,
    ExtraLarge,
}
impl TextSize {
    pub fn char_width(&self) -> usize {
        match self {
            TextSize::Medium => 1,
            TextSize::Large => 2,
            TextSize::ExtraLarge => 3,
        }
    }
}
impl PrintCommand for TextSize {
    fn command(&self, printer: &mut AnyPrinter) -> escpos::errors::Result<()> {
        match self {
            TextSize::Medium => {
                printer.reset_size()?;
                printer.reset_line_spacing()?;
            }
            TextSize::Large => {
                printer.size(2, 2)?;
                printer.line_spacing(2)?;
            }
            TextSize::ExtraLarge => {
                printer.size(3, 3)?;
                printer.line_spacing(3)?;
            }
        }
        Ok(())
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum Justification {
    #[default]
    Start,
    Center,
    End,
}
impl PrintCommand for Justification {
    fn command(&self, printer: &mut AnyPrinter) -> escpos::errors::Result<()> {
        match self {
            Justification::Start => printer.justify(JustifyMode::LEFT)?,
            Justification::Center => printer.justify(JustifyMode::CENTER)?,
            Justification::End => printer.justify(JustifyMode::RIGHT)?,
        };
        Ok(())
    }
}

#[derive(Default, Debug)]
pub struct StyledChar {
    pub ch: char,
    pub text_size: TextSize,
    pub bold: bool,
}

#[derive(Default, Debug)]
pub struct Line {
    pub chars: Vec<StyledChar>,
    pub justification: Justification,
    cached_width: usize,
}
impl Line {
    pub fn new(chars: Vec<StyledChar>, justification: Justification) -> Self {
        let cached_width = chars.iter().map(|sc| sc.text_size.char_width()).sum();
        Self {
            chars,
            justification,
            cached_width,
        }
    }

    /// Find the character index where we should soft-wrap (at whitespace).
    /// Returns None if the line fits within CPL or no whitespace is found.
    fn find_wrap_point(&self) -> Option<usize> {
        log::debug!(
            "Finding wrap point for {}",
            self.chars.iter().map(|sc| sc.ch).collect::<String>()
        );
        let mut width = 0;
        let mut last_whitespace_idx: Option<usize> = None;

        for (i, sc) in self.chars.iter().enumerate() {
            if sc.ch.is_whitespace() && width <= CPL as usize {
                last_whitespace_idx = Some(i)
            }
            width += sc.text_size.char_width();
            if width > CPL as usize {
                break;
            }
        }
        last_whitespace_idx
    }

    /// Add a character to the line, and return a new line if the line is full.
    /// Uses visual width (accounting for text size) to determine when to wrap.
    pub fn add_char(&mut self, styled_char: StyledChar) -> Option<Line> {
        let char_width = styled_char.text_size.char_width();
        self.cached_width += char_width;
        self.chars.push(styled_char);
        if self.cached_width <= CPL as usize {
            return None;
        }
        let remainder = if let Some(wrap_point) = self.find_wrap_point() {
            log::debug!(
                "Wrapping line at {} for {:?}",
                wrap_point,
                self.chars[wrap_point]
            );
            let mut remainder = self.chars.split_off(wrap_point);
            if !remainder.is_empty() {
                remainder.remove(0); // Remove whitespace at wrap point
            }
            remainder
        } else {
            log::trace!("No whitespace found, hard wrap for {:?}", self.chars.last());
            self.chars.split_off(self.chars.len() - 1)
        };
        (!remainder.is_empty()).then_some(Line::new(remainder, self.justification))
    }
}
