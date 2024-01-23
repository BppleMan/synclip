use color_eyre::Result;
use std::sync::Arc;
use tokio::sync::{watch, Mutex};

#[derive(Clone)]
pub struct RemoteClipboard {
    sender: Arc<Mutex<watch::Sender<String>>>,
    receiver: Arc<Mutex<watch::Receiver<String>>>,
}

impl RemoteClipboard {
    pub fn new(sender: watch::Sender<String>, receiver: watch::Receiver<String>) -> Self {
        Self {
            sender: Arc::new(Mutex::new(sender)),
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    pub async fn set(&self, content: String) -> Result<bool> {
        let replaced = self.sender.lock().await.send_if_modified(|prev| {
            if prev != &content {
                *prev = content;
                true
            } else {
                false
            }
        });
        Ok(replaced)
    }

    pub async fn current(&self) -> Result<String> {
        let content = self.receiver.lock().await.borrow().clone();
        Ok(content)
    }

    pub async fn get_new(&self) -> Result<String> {
        let mut receiver = self.receiver.lock().await;
        receiver.changed().await?;
        let content = receiver.borrow_and_update().clone();
        Ok(content)
    }
}
