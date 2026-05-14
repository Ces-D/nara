use crate::printer::{AnyPrinter, Justification, RongtaPrinter, TextSize};
use chrono::{DateTime, Utc};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct BoxOutline {
    date: Option<DateTime<Utc>>,
    banner: Option<String>,
    rows: u32,
    lined: bool,
}

impl BoxOutline {
    pub fn rows(&self) -> u32 {
        self.rows
    }

    pub fn lined(&self) -> bool {
        self.lined
    }

    pub fn banner(&self) -> Option<String> {
        self.banner.clone()
    }

    pub fn date(&self) -> Option<DateTime<Utc>> {
        self.date
    }

    pub fn set_date_banner(&mut self, date: Option<DateTime<Utc>>) -> &mut Self {
        self.date = date;
        self
    }

    pub fn set_banner(&mut self, message: Option<String>) -> &mut Self {
        self.banner = message;
        self
    }

    pub fn set_lined(&mut self, lined: bool) -> &mut Self {
        self.lined = lined;
        self
    }

    pub fn set_rows(&mut self, rows: u32) -> &mut Self {
        self.rows = rows;
        self
    }

    /// AKA build
    pub fn print(
        &self,
        printer: &mut RongtaPrinter,
        driver: AnyPrinter,
    ) -> escpos::errors::Result<()> {
        with_text_banner(printer, &self.banner)?;
        with_date_banner(printer, &self.date)?;
        with_body(printer, self.rows, self.lined)?;
        printer.print(None, driver)?;
        log::info!("Printed box template");
        Ok(())
    }
}

impl Default for BoxOutline {
    fn default() -> Self {
        Self {
            rows: 30,
            date: None,
            banner: None,
            lined: false,
        }
    }
}

// Add a centered banner with the date
fn with_date_banner(
    printer: &mut RongtaPrinter,
    date: &Option<DateTime<Utc>>,
) -> escpos::errors::Result<()> {
    printer.reset_cached();
    printer.set_line_justification(Justification::Center);
    printer.set_cached_bold(true);

    match date {
        Some(d) => {
            let str_date = d.format("%A, %B %d, %Y").to_string();
            printer.add_content(&str_date);
            printer.add_new_line();
            Ok(())
        }
        None => Ok(()),
    }
}

fn with_text_banner(
    printer: &mut RongtaPrinter,
    banner: &Option<String>,
) -> escpos::errors::Result<()> {
    printer.reset_cached();
    match banner {
        Some(b) => {
            printer.set_line_justification(Justification::Center);
            printer.set_cached_bold(true);
            printer.set_cached_text_size(TextSize::Large);
            printer.add_content(b);
            printer.add_new_line();
            printer.add_new_line();
            Ok(())
        }
        None => Ok(()),
    }
}

fn with_body(printer: &mut RongtaPrinter, rows: u32, lined: bool) -> escpos::errors::Result<()> {
    let pattern = super::get_random_box_pattern()?;
    printer.reset_cached();
    printer.set_cached_bold(true);

    printer.add_content(&pattern.top);
    printer.add_new_line();

    for i in 0..rows {
        if lined && i % 2 == 0 {
            printer.add_content(&pattern.row.replace(" ", "."));
        } else {
            printer.add_content(&pattern.row);
        }
        printer.add_new_line();
    }

    printer.add_content(&pattern.bottom);
    printer.add_new_line();
    Ok(())
}
