use std::{io, path::PathBuf};

mod print;
mod schedulable;

pub use print::KonanPrintChannel;
pub use schedulable::{
    FileBuildHandler, KonanPrintDeliverHandler, OutlineBuildHandler, TrackerBuildHandler,
};

pub const TASK_OUTLINE_BUILD: &str = "konan.outline.build";
pub const TASK_TRACKER_BUILD: &str = "konan.tracker.build";
pub const TASK_FILE_BUILD: &str = "konan.file.build";
pub const TASK_PRINT_DELIVER: &str = "konan.print.deliver";
pub const CHANNEL_PRINT: &str = "konan.print";

pub const MIME_OUTLINE: &str = "application/x-konan-outline";
pub const MIME_TRACKER: &str = "application/x-konan-tracker";

/// Tagged payload handed from a build handler to the deliver handler.
/// Carries enough information for the deliver step to reconstruct the right
/// [`cadence_core::channels::Artifact`] variant for [`KonanPrintChannel`].
#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum KonanDeliverPayload {
    Outline {
        outline: crate::template::BoxOutline,
    },
    Tracker {
        tracker: crate::template::HabitTracker,
    },
    File {
        file_name: String,
        rows: Option<u32>,
    },
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct PrintFileTask {
    pub file_name: String,
    pub rows: Option<u32>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub enum PrintTask {
    Outline(crate::template::BoxOutline),
    Tracker(crate::template::HabitTracker),
    File(PrintFileTask),
}

/// Location of application_storage
fn application_storage() -> PathBuf {
    let home = std::env::home_dir().expect("Unable to find HOME env variable");
    let p = home.join(".local/share/konan");
    if !p.exists() {
        std::fs::create_dir_all(&p).unwrap_or_else(|_| {
            panic!(
                "Unable to create konan storage directory at: {}",
                p.display()
            )
        });
    }
    p
}

pub(crate) fn print_file_directory() -> PathBuf {
    let p = application_storage().join("files");
    if !p.exists() {
        std::fs::create_dir_all(&p).unwrap_or_else(|_| {
            panic!(
                "Unable to create konan file storage directory at: {}",
                p.display()
            )
        })
    }
    p
}

/// Reads a file from the print file directory by name.
pub fn read_print_file(file_name: &str) -> io::Result<Vec<u8>> {
    let path = print_file_directory().join(file_name);
    std::fs::read(&path)
}

/// Writes a markdown file to the print file directory.
/// Returns an error if `file_name` does not end with `.md` or contains any
/// path-component characters. Callers are still expected to validate input,
/// but this check stops path-traversal even if a caller forgets.
pub fn upload_print_file(file_name: &str, content: &[u8]) -> io::Result<()> {
    if !file_name.ends_with(".md") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("file must be a markdown file (.md): {file_name}"),
        ));
    }
    if file_name.is_empty()
        || file_name.contains('/')
        || file_name.contains('\\')
        || file_name.contains('\0')
        || file_name.contains("..")
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid file name: {file_name}"),
        ));
    }
    let dir = print_file_directory();
    std::fs::write(dir.join(file_name), content)
}
