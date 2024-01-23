mod synclip_rpc;

use crate::clipboard::remote_clipboard::RemoteClipboard;
use crate::clipboard::VirtualClipboard;
use crate::server::synclip_rpc::SynclipRpc;
use crate::synclip_server;
use color_eyre::Result;
use std::sync::{Arc, Mutex};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::info;

#[derive(Clone)]
pub struct SynclipServer {
    remote: RemoteClipboard,
    handle: Arc<Mutex<Option<JoinHandle<Result<()>>>>>,
}

impl SynclipServer {
    pub async fn new(port: u16, initial: String, cancel_token: CancellationToken) -> Result<Self> {
        let (sender_1, receiver_1) = watch::channel(initial.clone());
        let (sender_2, receiver_2) = watch::channel(initial);
        let rpc = SynclipRpc::new(sender_2, receiver_1);

        let addr = format!("0.0.0.0:{}", port).parse()?;
        let mut server = tonic::transport::Server::default();
        let router = server.add_service(synclip_server::SynclipServer::new(rpc));

        let handle = tokio::spawn(async move {
            router
                .serve_with_shutdown(addr, async move {
                    info!("Listening on: {}", addr);
                    cancel_token.cancelled().await;
                    info!("Received shutdown signal");
                })
                .await?;
            info!("End [Server]");
            Ok(())
        });

        let server = Self {
            remote: RemoteClipboard::new(sender_1, receiver_2),
            handle: Arc::new(Mutex::new(Some(handle))),
        };

        Ok(server)
    }

    pub fn shutdown(self) -> Result<()> {
        info!("Shutdown [Server]");
        self.handle.lock().unwrap().take().unwrap().abort();
        Ok(())
    }
}

impl VirtualClipboard for SynclipServer {
    fn remote(&self) -> &RemoteClipboard {
        &self.remote
    }

    fn shutdown(self) -> Result<()> {
        self.shutdown()
    }
}
