#[derive(Debug, thiserror::Error)]
pub enum TowerError {
    #[error("discord channel config: {0}")]
    ChannelConfig(String),
}
