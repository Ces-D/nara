use std::path::Path;

use konan_core::{
    print_ops::{CreateSchedule, PrintFileTask, Schedule},
    template::{BoxOutline, HabitTracker},
};
use reqwest::{
    StatusCode,
    blocking::{Client, Response, multipart},
};

use crate::error::CliError;

pub const BASE_URL_ENV: &str = "NARA_SERVER_URL";

pub struct TitansTowerClient {
    http: Client,
    base_url: String,
}

impl TitansTowerClient {
    pub fn from_env() -> Result<Self, CliError> {
        let raw = std::env::var(BASE_URL_ENV).map_err(|_| CliError::MissingEnv(BASE_URL_ENV))?;
        let trimmed = raw.trim().trim_end_matches('/');
        if trimmed.is_empty() {
            return Err(CliError::MissingEnv(BASE_URL_ENV));
        }
        Ok(Self {
            http: Client::new(),
            base_url: trimmed.to_string(),
        })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    pub fn print_outline(&self, payload: &BoxOutline) -> Result<(), CliError> {
        let resp = self
            .http
            .post(self.url("/konan/print/outline"))
            .json(payload)
            .send()?;
        expect_success(resp)?;
        Ok(())
    }

    pub fn print_tracker(&self, payload: &HabitTracker) -> Result<(), CliError> {
        let resp = self
            .http
            .post(self.url("/konan/print/tracker"))
            .json(payload)
            .send()?;
        expect_success(resp)?;
        Ok(())
    }

    pub fn print_file(&self, payload: &PrintFileTask) -> Result<(), CliError> {
        let resp = self
            .http
            .post(self.url("/konan/print/file"))
            .json(payload)
            .send()?;
        expect_success(resp)?;
        Ok(())
    }

    pub fn upload_file(&self, local_path: &Path) -> Result<(), CliError> {
        let basename = local_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| CliError::InvalidPath(local_path.display().to_string()))?
            .to_string();
        let bytes = std::fs::read(local_path)?;
        let part = multipart::Part::bytes(bytes).file_name(basename);
        let form = multipart::Form::new().part("file", part);

        let resp = self
            .http
            .post(self.url("/konan/upload"))
            .multipart(form)
            .send()?;
        expect_success(resp)?;
        Ok(())
    }

    pub fn create_schedule(&self, payload: &CreateSchedule) -> Result<i64, CliError> {
        let resp = self
            .http
            .post(self.url("/konan/schedules"))
            .json(payload)
            .send()?;
        let resp = expect_success(resp)?;
        let id = resp.json::<i64>()?;
        Ok(id)
    }

    pub fn list_schedules(&self) -> Result<Vec<Schedule>, CliError> {
        let resp = self.http.get(self.url("/konan/schedules")).send()?;
        let resp = expect_success(resp)?;
        let schedules = resp.json::<Vec<Schedule>>()?;
        Ok(schedules)
    }

    pub fn delete_schedule(&self, id: i64) -> Result<bool, CliError> {
        let resp = self
            .http
            .delete(self.url(&format!("/konan/schedules/{id}")))
            .send()?;
        match resp.status() {
            StatusCode::NO_CONTENT => Ok(true),
            StatusCode::NOT_FOUND => Ok(false),
            status => {
                let body = resp.text().unwrap_or_default();
                Err(CliError::Server { status, body })
            }
        }
    }
}

fn expect_success(resp: Response) -> Result<Response, CliError> {
    let status = resp.status();
    if status.is_success() {
        Ok(resp)
    } else {
        let body = resp.text().unwrap_or_default();
        Err(CliError::Server { status, body })
    }
}
