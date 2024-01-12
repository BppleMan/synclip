use crate::clipboard::Clipboard;
use color_eyre::Result;
use synclip_proto::Content;
use tonic::transport::Channel;
use tracing::error;

#[derive(Clone)]
pub struct SynclipClient {
    client: synclip_proto::synclip_client::SynclipClient<Channel>,
    clipboard: Clipboard,
    shutdown_sender: tokio::sync::broadcast::Sender<()>,
}

impl SynclipClient {
    pub async fn new(address: impl AsRef<str>, clipboard: Clipboard) -> Result<Self> {
        let client = synclip_proto::synclip_client::SynclipClient::connect(address.as_ref().to_owned()).await?;
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
                            let request = tonic::Request::new(Content { text: content });
                            if let Err(e) = self.client.set_clipboard(request).await {
                                error!("set clipboard to server error: {:?}", e);
                            }
                        }
                        Err(e) => {
                            error!("get clipboard error: {:?}", e);
                        }
                    }
                } => {}
            }
        }
        Ok(())
    }

    pub async fn polling_server(&mut self) -> Result<()> {
        let mut signal_rx = self.shutdown_sender.subscribe();
        let request = tonic::Request::new(synclip_proto::Empty::default());
        let mut stream = self.client.polling_clipboard(request).await?.into_inner();
        loop {
            tokio::select! {
                _ = signal_rx.recv() => {
                    break;
                }
                _ = async {
                    match stream.message().await {
                        Ok(content) => {
                            if let Some(Content { text: Some(content) }) = content {
                                if let Err(e) = self.clipboard.set(content) {
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
