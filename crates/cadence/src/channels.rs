use crate::error::CadenceError;
use async_trait::async_trait;
use std::{collections::HashMap, path::PathBuf, sync::Arc};

pub enum Artifact {
    MarkdownFile(PathBuf),
    PlainText(String),
    Bytes { data: Vec<u8>, mime: String },
}

#[async_trait]
pub trait DeliveryChannel: Send + Sync {
    fn name(&self) -> &'static str;
    fn accepts(&self, artifact: &Artifact) -> bool;
    async fn deliver(&self, artifact: Artifact) -> Result<(), CadenceError>;
}

#[derive(Default, Clone)]
pub struct ChannelRegistry {
    channels: HashMap<&'static str, Arc<dyn DeliveryChannel>>,
}

impl ChannelRegistry {
    pub fn register<C: DeliveryChannel + 'static>(&mut self, channel: C) {
        let name = channel.name();
        if self.channels.insert(name, Arc::new(channel)).is_some() {
            panic!("duplicate channel registration: {name}")
        }
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn DeliveryChannel>> {
        self.channels.get(name).cloned()
    }

    pub async fn deliver(&self, name: &str, artifact: Artifact) -> Result<(), CadenceError> {
        let channel = self.get(name).ok_or_else(|| CadenceError::NoChannel)?;
        if !channel.accepts(&artifact) {
            return Err(CadenceError::ArtifactNotAccepted);
        }
        channel.deliver(artifact).await
    }
}
