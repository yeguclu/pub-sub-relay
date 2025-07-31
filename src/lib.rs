use tokio::sync::mpsc;
use serde::Deserialize;

pub struct Consumer {
    pub tx: mpsc::UnboundedSender<String>,
    pub topic: String, // the topic this consumer subscribed to
}

#[derive(Deserialize)]
pub struct Message {
    pub topic: String,
    pub payload: String,
}