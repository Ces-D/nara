use std::path::PathBuf;

pub mod interpreter;
pub mod print_ops;
pub mod printer;
pub mod template;

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

fn print_job_database() -> PathBuf {
    application_storage().join("konan.db")
}

fn print_file_directory() -> PathBuf {
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
