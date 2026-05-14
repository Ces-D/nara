use crate::printer::{
    AnyPrinter,
    element::{Justification, Line, PrintCommand, StyledChar, TextSize},
    page_code,
};

#[derive(Default)]
pub struct RongtaPrinter {
    lines: Vec<Line>,
    cut: bool,
    cached_text_size: TextSize,
    cached_bold: bool,
}
impl RongtaPrinter {
    pub fn new(cut: bool) -> Self {
        Self {
            cut,
            ..Default::default()
        }
    }

    /// Add content to lines using cached text size and bold setting
    pub fn add_content(&mut self, content: &str) {
        if self.lines.is_empty() {
            self.add_new_line();
        }
        for char in content.chars() {
            let new_line = {
                let current_line = self.lines.last_mut().expect("Default new line not added");
                current_line.add_char(StyledChar {
                    ch: char,
                    text_size: self.cached_text_size,
                    bold: self.cached_bold,
                })
            };
            if let Some(new_line) = new_line {
                self.lines.push(new_line);
            }
        }
    }

    pub fn add_new_line(&mut self) {
        self.lines.push(Line::default());
    }

    pub fn set_line_justification(&mut self, justification: Justification) {
        if self.lines.is_empty() {
            self.add_new_line();
        }
        self.lines.last_mut().unwrap().justification = justification;
    }

    pub fn set_cached_text_size(&mut self, text_size: TextSize) {
        self.cached_text_size = text_size
    }

    pub fn set_cached_bold(&mut self, bold: bool) {
        self.cached_bold = bold
    }

    pub fn reset_cached(&mut self) {
        self.cached_bold = false;
        self.cached_text_size = TextSize::default();
    }

    pub fn print(&self, rows: Option<u32>, mut printer: AnyPrinter) -> escpos::errors::Result<()> {
        let mut state = PrintState::default();
        if let Some(rows_per_page) = rows {
            let mut line_count = 0;
            for line in &self.lines {
                print_line(line, &mut printer, &mut state)?;
                line_count += 1;
                if line_count >= rows_per_page {
                    printer.print_cut()?;
                    line_count = 0;
                    state = PrintState::default();
                }
            }
            if line_count > 0 {
                while line_count < rows_per_page {
                    printer.feed()?;
                    line_count += 1;
                }
                printer.print_cut()?;
            }
        } else {
            for line in &self.lines {
                print_line(line, &mut printer, &mut state)?;
            }
            match self.cut {
                true => printer.print_cut()?,
                false => printer.print()?,
            };
        }
        Ok(())
    }
}

#[derive(Default)]
struct PrintState {
    justification: Justification,
    text_size: TextSize,
    bold: bool,
    page_code: page_code::CharPageCode,
}

fn print_line(
    line: &Line,
    printer: &mut AnyPrinter,
    state: &mut PrintState,
) -> escpos::errors::Result<()> {
    if state.justification != line.justification {
        line.justification.command(printer)?;
        state.justification = line.justification;
    }
    for styled_char in line.chars.iter() {
        if state.text_size != styled_char.text_size {
            styled_char.text_size.command(printer)?;
            state.text_size = styled_char.text_size;
        }
        if state.bold != styled_char.bold {
            printer.bold(styled_char.bold)?;
            state.bold = styled_char.bold;
        }
        let normalized = page_code::normalize_char(styled_char.ch).unwrap_or(styled_char.ch);
        let required = page_code::char_page_code(normalized).ok_or_else(|| {
            escpos::errors::PrinterError::Input("Unable to locate char's page code".to_string())
        })?;
        if state.page_code != required {
            printer.page_code(required.into())?;
            state.page_code = required;
        }
        printer.write(&normalized.to_string())?;
    }

    printer.feed()
}
