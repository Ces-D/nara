use crate::interpreter::MarkdownInterpreter;
use crate::print_ops::{CHANNEL_PRINT, MIME_OUTLINE, MIME_TRACKER};
use crate::printer::{RongtaPrinter, configured_printer};
use crate::template::{BoxOutline, HabitTracker};
use async_trait::async_trait;
use cadence_core::{
    channels::{Artifact, DeliveryChannel},
    error::CadenceError,
};

pub struct KonanPrintChannel;

#[async_trait]
impl DeliveryChannel for KonanPrintChannel {
    fn name(&self) -> &'static str {
        CHANNEL_PRINT
    }

    fn accepts(&self, artifact: &Artifact) -> bool {
        match artifact {
            Artifact::MarkdownFile(_) => true,
            Artifact::Bytes { mime, .. } => mime == MIME_OUTLINE || mime == MIME_TRACKER,
            Artifact::PlainText(_) => false,
        }
    }

    async fn deliver(&self, artifact: Artifact) -> Result<(), CadenceError> {
        tokio::task::spawn_blocking(move || print_artifact_blocking(artifact)).await?
    }
}

fn print_artifact_blocking(artifact: Artifact) -> Result<(), CadenceError> {
    match artifact {
        Artifact::Bytes { mime, data } if mime == MIME_OUTLINE => {
            let outline: BoxOutline =
                serde_json::from_slice(&data).map_err(|e| CadenceError::Channel(e.to_string()))?;
            let driver = configured_printer();
            let mut printer = RongtaPrinter::new(true);
            outline
                .print(&mut printer, driver)
                .map_err(|e| CadenceError::Channel(e.to_string()))
        }
        Artifact::Bytes { mime, data } if mime == MIME_TRACKER => {
            let tracker: HabitTracker =
                serde_json::from_slice(&data).map_err(|e| CadenceError::Channel(e.to_string()))?;
            let driver = configured_printer();
            let mut printer = RongtaPrinter::new(true);
            tracker
                .print(&mut printer, driver)
                .map_err(|e| CadenceError::Channel(e.to_string()))
        }
        Artifact::MarkdownFile(path) => {
            let bytes = std::fs::read(&path).map_err(|e| CadenceError::Channel(e.to_string()))?;
            let content = String::from_utf8_lossy(&bytes);
            let printer = RongtaPrinter::new(true);
            let mut interp = MarkdownInterpreter::new(printer);
            interp.render_content(&content);
            let driver = configured_printer();
            interp
                .print(None, driver)
                .map_err(|e| CadenceError::Channel(e.to_string()))
        }
        _ => Err(CadenceError::ArtifactNotAccepted),
    }
}
