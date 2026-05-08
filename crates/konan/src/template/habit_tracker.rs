use crate::printer::{AnyPrinter, Justification, RongtaPrinter, TextSize};
use chrono::{DateTime, Datelike, Days, Duration, Utc};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct HabitTracker {
    habit: String,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
}

impl HabitTracker {
    pub fn new(habit: String, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Self {
        Self {
            habit,
            start_date,
            end_date,
        }
    }

    pub fn habit(&self) -> &str {
        &self.habit
    }

    pub fn start_date(&self) -> DateTime<Utc> {
        self.start_date
    }

    pub fn end_date(&self) -> DateTime<Utc> {
        self.end_date
    }

    pub fn print(
        &self,
        printer: &mut RongtaPrinter,
        driver: AnyPrinter,
    ) -> escpos::errors::Result<()> {
        with_time_period(printer, &self.start_date, &self.end_date)?;
        with_body(printer, &self.habit, &self.start_date, &self.end_date)?;
        printer.print(None, driver)?;
        log::info!("Printed habit tracker template");
        Ok(())
    }
}

fn with_time_period(
    printer: &mut RongtaPrinter,
    start_date: &DateTime<Utc>,
    end_date: &DateTime<Utc>,
) -> escpos::errors::Result<()> {
    printer.add_new_line();
    printer.set_line_justification(Justification::Center);
    printer.set_cached_bold(true);
    let start_str = start_date.format("%B %d, %Y").to_string();
    let end_str = end_date.format("%B %d, %Y").to_string();
    printer.add_content(&format!("{} - {}", start_str, end_str));
    printer.add_new_line();
    Ok(())
}

fn with_habit(printer: &mut RongtaPrinter, habit: &str) -> escpos::errors::Result<()> {
    printer.set_line_justification(Justification::Center);
    printer.set_cached_text_size(TextSize::Large);
    printer.add_content(&habit.to_ascii_uppercase());
    printer.add_new_line();
    Ok(())
}

fn with_checkmarks(
    printer: &mut RongtaPrinter,
    start_date: &DateTime<Utc>,
    end_date: &DateTime<Utc>,
) -> escpos::errors::Result<()> {
    printer.set_line_justification(Justification::Center);
    printer.set_cached_bold(true);
    printer.set_cached_text_size(TextSize::Medium);

    const SEGMENTS_PER_LINE: usize = 4;

    let mut current_date = *start_date;
    let mut day_numbers = Vec::new();

    while current_date
        < end_date
            .checked_add_days(Days::new(1))
            .expect("End date overflow")
    {
        day_numbers.push(current_date.day());
        current_date = current_date
            .checked_add_days(Days::new(1))
            .unwrap_or(current_date + Duration::days(1));
    }

    for chunk in day_numbers.chunks(SEGMENTS_PER_LINE) {
        let line = chunk
            .iter()
            .map(|day| format!("( {:02} )", day))
            .collect::<Vec<_>>()
            .join("      ");
        printer.add_content(&line);
        printer.add_new_line();
    }

    Ok(())
}

fn with_body(
    printer: &mut RongtaPrinter,
    habit: &str,
    start_date: &DateTime<Utc>,
    end_date: &DateTime<Utc>,
) -> escpos::errors::Result<()> {
    let pattern = super::get_random_box_pattern()?;

    printer.set_line_justification(Justification::Start);
    printer.set_cached_bold(true);
    printer.set_cached_text_size(TextSize::Medium);
    printer.add_content(&pattern.top);
    printer.add_new_line();

    with_habit(printer, habit)?;

    printer.set_line_justification(Justification::Start);
    printer.set_cached_bold(true);
    printer.set_cached_text_size(TextSize::Medium);
    printer.add_content(&pattern.top);
    printer.add_new_line();

    with_checkmarks(printer, start_date, end_date)?;

    printer.set_line_justification(Justification::Start);
    printer.set_cached_text_size(TextSize::Medium);
    printer.add_content(&pattern.bottom);
    printer.add_new_line();

    Ok(())
}
