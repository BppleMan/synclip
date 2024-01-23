use crate::clipboard::remote_clipboard::RemoteClipboard;
use crate::clipboard::VirtualClipboard;
use crate::{synclip_client, Content, Empty};
use color_eyre::eyre::{eyre, Context};
use color_eyre::Result;
use std::sync::{Arc, Mutex};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tonic::transport::Channel;
use tracing::{error, info};

#[derive(Clone)]
pub struct SynclipClient {
    remote: RemoteClipboard,
    handle: Arc<Mutex<Option<std::thread::JoinHandle<Result<()>>>>>,
}

impl SynclipClient {
    pub async fn new(
        address: impl AsRef<str>,
        initial: String,
        cancel_token: CancellationToken,
    ) -> Result<Self> {
        let client = synclip_client::SynclipClient::connect(address.as_ref().to_owned()).await?;
        let (sender_1, receiver_1) = watch::channel(initial.clone());
        let (sender_2, receiver_2) = watch::channel(initial);

        let handle = std::thread::spawn(move || {
            let handle1 = Self::polling_local(client.clone(), receiver_1, cancel_token.clone());
            let handle2 = Self::polling_server(client, sender_2, cancel_token);
            handle1.join().unwrap();
            handle2.join().unwrap()
        });

        let client = Self {
            remote: RemoteClipboard::new(sender_1, receiver_2),
            handle: Arc::new(Mutex::new(Some(handle))),
        };

        Ok(client)
    }

    pub fn polling_server(
        mut client: synclip_client::SynclipClient<Channel>,
        sender: watch::Sender<String>,
        cancel_token: CancellationToken,
    ) -> std::thread::JoinHandle<Result<()>> {
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;

            runtime.block_on(async move {
                let request = tonic::Request::new(Empty::default());
                let response = client.polling_clipboard(request).await?;
                let mut stream = response.into_inner();

                loop {
                    tokio::select! {
                        _ = cancel_token.cancelled() => {
                            info!("Polling [Client-Server] shutdown");
                            break;
                        }
                        result = async {
                            match stream.message().await {
                                Ok(Some(Content { text })) => {
                                    let _replaced = sender.send_if_modified(|prev| {
                                        if prev != &text {
                                            *prev = text;
                                            true
                                        } else {
                                            false
                                        }
                                    });
                                    Ok(())
                                }
                                Ok(None) => Err(eyre!("Get clipboard from [Remote] None")),
                                Err(e) => Err(eyre!(e))
                            }
                        } => {
                            if let Err(e) = result {
                                error!("Polling [Client-Server] error: {:?}", e);
                                break;
                            }
                        }
                    }
                }
                cancel_token.cancel();
                info!("End polling [Client-Server]");
                Ok(())
            })
        })
    }

    pub fn polling_local(
        mut client: synclip_client::SynclipClient<Channel>,
        mut receiver: watch::Receiver<String>,
        cancel_token: CancellationToken,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async move {
                loop {
                    tokio::select! {
                        _ = cancel_token.cancelled() => {
                            info!("Polling [Client-Local] shutdown");
                            break;
                        }
                        _ = receiver.changed() => {
                            let content = receiver.borrow_and_update().clone();
                            let request = tonic::Request::new(Content { text: content.clone() });
                            let response = client
                                .set_clipboard(request)
                                .await
                                .with_context(|| "Send clipboard to [Remote]");
                            match response {
                                Ok(response) => {
                                    let replaced = response.into_inner().replaced;
                                    if replaced {
                                        info!("Set [Remote] with: [{replaced}] {:?}", content);
                                    }
                                }
                                Err(e) => {
                                    error!("Set [Remote] error: {:?}", e);
                                    break;
                                }
                            }
                        }
                    }
                }
                cancel_token.cancel();
                info!("End polling [Client-Local]");
            });
        })
    }

    pub fn shutdown(self) -> Result<()> {
        info!("Shutdown [Client]");
        self.handle
            .lock()
            .unwrap()
            .take()
            .unwrap()
            .join()
            .unwrap()?;
        Ok(())
    }
}

impl VirtualClipboard for SynclipClient {
    fn remote(&self) -> &RemoteClipboard {
        &self.remote
    }

    fn shutdown(self) -> Result<()> {
        self.shutdown()
    }
}
