use crate::clipboard::Clipboard;
use crate::{synclip_client, Content, Empty};
use color_eyre::Result;
use tonic::transport::Channel;
use tracing::{error, info};

#[derive(Clone)]
pub struct SynclipClient {
    client: synclip_client::SynclipClient<Channel>,
    clipboard: Clipboard,
    shutdown_sender: tokio::sync::broadcast::Sender<()>,
}

impl SynclipClient {
    pub async fn new(address: impl AsRef<str>, clipboard: Clipboard) -> Result<Self> {
        let client = synclip_client::SynclipClient::connect(address.as_ref().to_owned()).await?;
        let (shutdown_sender, _) = tokio::sync::broadcast::channel::<()>(1);
        let client = Self {
            client,
            clipboard,
            shutdown_sender,
        };
        Ok(client)
    }

    pub async fn polling_clipboard(&mut self) -> Result<()> {
        let mut signal_rx = self.shutdown_sender.subscribe();
        loop {
            tokio::select! {
                _ = signal_rx.recv() => {
                    break;
                }
                _ = async {
                    match self.clipboard.get().await {
                        Ok(content) => {
                            info!("Get clipboard from [Local]: {:?}", content);
                            let request = tonic::Request::new(Content { text: content });
                            if let Err(e) = self.client.set_clipboard(request).await {
                                error!("Send clipboard to [Remote] error: {:?}", e);
                            }
                        }
                        Err(e) => {
                            error!("Get clipboard from [Local] error: {:?}", e);
                        }
                    }
                } => {}
            }
        }
        Ok(())
    }

    pub async fn polling_server(&mut self) -> Result<()> {
        let mut signal_rx = self.shutdown_sender.subscribe();
        let request = tonic::Request::new(Empty::default());
        let mut stream = self.client.polling_clipboard(request).await?.into_inner();
        loop {
            tokio::select! {
                _ = signal_rx.recv() => {
                    break;
                }
                _ = async {
                    match stream.message().await {
                        Ok(content) => {
                            if let Some(Content { text }) = content {
                                info!("Get clipboard from [Remote]: {:?}", text);
                                if let Err(e) = self.clipboard.set(text) {
                                    error!("set clipboard error: {:?}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("polling server error: {:?}", e);
                        }
                    }
                } => {}
            }
        }
        Ok(())
    }

    pub fn shutdown(&self) -> Result<()> {
        self.shutdown_sender.send(())?;
        Ok(())
    }
}
