mod synclip_rpc;

use crate::clipboard::Clipboard;
use crate::server::synclip_rpc::SynclipRpc;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use tokio::task::JoinHandle;
use tracing::info;

pub struct SynclipServer {
    shutdown_sender: tokio::sync::oneshot::Sender<()>,
    server_handle: JoinHandle<Result<()>>,
}

impl SynclipServer {
    pub fn new(port: u16, clipboard: Clipboard) -> Result<Self> {
        let rpc = SynclipRpc::new(clipboard);
        let addr = format!("0.0.0.0:{}", port).parse()?;
        let mut server = tonic::transport::Server::default();
        let router = server.add_service(synclip_proto::synclip_server::SynclipServer::new(rpc));

        let (shutdown_sender, shutdown_receiver) = tokio::sync::oneshot::channel::<()>();
        let server_handle: JoinHandle<Result<()>> = tokio::spawn(async move {
            router
                .serve_with_shutdown(addr, async move {
                    info!("Listening on: {}", addr);
                    let signal = shutdown_receiver.await;
                    info!("Received shutdown signal: {:?}", signal);
                })
                .await?;
            info!("Server shutdown");
            Ok(())
        });

        let server = Self {
            shutdown_sender,
            server_handle,
        };
        Ok(server)
    }

    pub async fn shutdown(self) -> Result<()> {
        self.shutdown_sender
            .send(())
            .map_err(|_| eyre!("Send shutdown signal error"))?;
        self.server_handle.await??;
        Ok(())
    }
}
