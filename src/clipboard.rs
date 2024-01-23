pub mod local_clipboard;
pub mod remote_clipboard;

use crate::clipboard::local_clipboard::LocalClipboard;
use crate::clipboard::remote_clipboard::RemoteClipboard;
use color_eyre::Result;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, watch};
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

pub type ClipboardSender = broadcast::Sender<String>;
pub type ClipboardReceiver = watch::Receiver<String>;

#[derive(Clone)]
pub struct Clipboard<T: VirtualClipboard> {
    remote: T,
    local: LocalClipboard,
    frequency: Arc<AtomicU64>,
    cancel_token: CancellationToken,
}

pub enum ClipboardEvent {
    SetLocal(String),
    Shutdown,
}

impl<T: VirtualClipboard + 'static> Clipboard<T> {
    pub fn new(
        local: LocalClipboard,
        remote: T,
        frequency: u64,
        cancel_token: CancellationToken,
    ) -> Self {
        let frequency = Arc::new(AtomicU64::new(frequency));
        Self {
            remote,
            local,
            frequency,
            cancel_token,
        }
    }

    pub fn start(&mut self) -> std::thread::JoinHandle<()> {
        let this = self.clone();
        let handle1 = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(this.polling_local());
        });
        let this = self.clone();
        let handle2 = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(this.listen_remote());
        });
        std::thread::spawn(move || {
            handle1.join().unwrap();
            handle2.join().unwrap();
        })
    }

    pub async fn shutdown(self) -> Result<()> {
        info!("Shutdown [Remote]");
        self.remote.shutdown()?;
        Ok(())
    }

    async fn polling_local(&self) {
        let mut interval = interval(tokio::time::Duration::from_millis(
            self.frequency.load(Ordering::Relaxed),
        ));
        loop {
            tokio::select! {
                _ = self.cancel_token.cancelled() => {
                    info!("Polling [Local] shutdown");
                    break;
                }
                _ = interval.tick() => {
                    match self.local.get().await {
                        Ok(content) => {
                            match self.remote.remote().set(content.clone()).await {
                                Ok(replaced) => {
                                    if replaced {
                                        info!("Set [Remote] with: [{replaced}] {:?}", content);
                                    }
                                }
                                Err(e) => {
                                    error!("Set [Remote] error: {:?}", e);
                                    break;
                                },
                            }
                        }
                        Err(e) => {
                            error!("Get [Local] error: {:?}", e);
                            break;
                        }
                    }
                }
            }
        }
        self.cancel_token.cancel();
        info!("End polling [Local]");
    }

    async fn listen_remote(&self) {
        loop {
            tokio::select! {
                _ = self.cancel_token.cancelled() => {
                    info!("Listen [Remote] shutdown");
                    break;
                }
                result =  async {
                    match self.remote.remote().get_new().await {
                        Ok(content) => {
                            info!("Get [Remote] with: {:?}", content);
                            match self.local.set(content.clone()).await {
                                Ok(replaced) => {
                                    if replaced {
                                        info!("Set [Local] with: [{replaced}] {:?}", content);
                                    }
                                    Ok(())
                                }
                                Err(e) => {
                                    error!("Set [Local] error: {:?}", e);
                                    Err(())
                                },
                            }
                        }
                        Err(e) => {
                            error!("Get [Remote] error: {:?}", e);
                            Err(())
                        }
                    }
                } => {
                    if result.is_err() {
                        break;
                    }
                }
            }
        }
        info!("End listen [Remote]");
    }
}

pub trait VirtualClipboard: Clone + Send + Sync {
    fn remote(&self) -> &RemoteClipboard;

    fn shutdown(self) -> Result<()>;
}
