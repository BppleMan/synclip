use clipboard::ClipboardProvider;
use color_eyre::{eyre::eyre, Result};
use std::sync::Arc;
use tokio::sync::{mpsc, watch, Mutex};
use tokio::task::JoinHandle;
use tokio::time::interval;
use tokio_stream::wrappers::WatchStream;
use tracing::{error, info};

type ClipboardContext = Arc<Mutex<clipboard::ClipboardContext>>;

type MessageSender = mpsc::UnboundedSender<ClipboardEvent>;
// type MessageReceiver = mpsc::UnboundedReceiver<ClipboardEvent>;

pub type ClipboardSender = watch::Sender<Option<String>>;
pub type ClipboardReceiver = watch::Receiver<Option<String>>;

#[derive(Clone)]
pub struct Clipboard {
    pub current: ClipboardReceiver,
    message: MessageSender,
    handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

pub enum ClipboardEvent {
    Set(String),
    Shutdown,
}

impl Clipboard {
    pub fn new(frequency: u64) -> Result<Self> {
        let context = ClipboardProvider::new().map_err(|e| eyre!(format!("{:?}", e)))?;
        let context = Arc::new(Mutex::new(context));
        let (handle, current, message) = Self::start(context.clone(), frequency)?;
        let clipboard = Self {
            message,
            current,
            handle: Arc::new(Mutex::new(Some(handle))),
        };
        Ok(clipboard)
    }

    pub fn start(
        context: ClipboardContext,
        frequency: u64,
    ) -> Result<(JoinHandle<()>, ClipboardReceiver, MessageSender)> {
        let (current_tx, current_rx) = watch::channel(None);
        let (message_tx, mut message_rx) = mpsc::unbounded_channel::<ClipboardEvent>();

        let handle: JoinHandle<()> = tokio::spawn(async move {
            let mut interval = interval(tokio::time::Duration::from_millis(frequency));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = Self::polling(&current_tx, context.clone()).await {
                            error!("polling error: {:?}", e);
                        }
                    }
                    Some(event) = message_rx.recv() => {
                        match &event {
                            ClipboardEvent::Set(content) => {
                                info!("set clipboard: {:?}", content);
                                if let Err(e) = Self::set_clipboard(context.clone(), content.clone()).await {
                                    error!("set clipboard error: {:?}", e);
                                }
                            }
                            ClipboardEvent::Shutdown => {
                                info!("clipboard shutdown");
                                break;
                            }
                        }
                    }
                }
            }
        });

        Ok((handle, current_rx, message_tx))
    }

    async fn polling(current_tx: &ClipboardSender, context: ClipboardContext) -> Result<()> {
        let content = Self::get_clipboard(context).await?;
        current_tx.send(Some(content)).map_err(|e| eyre!(format!("{:?}", e)))?;
        let current_rx = current_tx.subscribe();
        if current_rx.has_changed()? {
            let content = current_rx.borrow();
            info!("set clipboard: {:?}", content);
        }
        Ok(())
    }

    async fn get_clipboard(context: ClipboardContext) -> Result<String> {
        let content = context
            .lock()
            .await
            .get_contents()
            .map_err(|e| eyre!(format!("{:?}", e)))?;
        Ok(content)
    }

    async fn set_clipboard(context: ClipboardContext, content: String) -> Result<()> {
        context
            .lock()
            .await
            .set_contents(content)
            .map_err(|e| eyre!(format!("{:?}", e)))?;
        Ok(())
    }

    pub fn set(&self, content: impl AsRef<str>) -> Result<()> {
        self.message.send(ClipboardEvent::Set(content.as_ref().to_string()))?;
        Ok(())
    }

    pub async fn get(&mut self) -> Result<Option<String>> {
        self.current.changed().await?;
        Ok(self.current.borrow_and_update().clone())
    }

    pub async fn shutdown(self) -> Result<()> {
        self.message.send(ClipboardEvent::Shutdown)?;
        self.handle.lock().await.take().unwrap().await?;
        Ok(())
    }

    pub fn as_stream(&self) -> WatchStream<Option<String>> {
        WatchStream::new(self.current.clone())
    }
}
