use clap::Parser;
use color_eyre::Result;
use tokio_util::sync::CancellationToken;
use tracing::info;

use synclip::clipboard::local_clipboard::LocalClipboard;
use synclip::clipboard::Clipboard;
use synclip::{client, server};

#[derive(Parser)]
#[command(
author,
version,
about,
long_about = None
)]
pub enum Cli {
    /// Run as a server
    Server {
        /// The address to connect to
        port: u16,
    },
    /// Run as a client
    Client {
        /// The address to connect to (lke http://[remote]:[port])
        address: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();
    info!("pid: {}", std::process::id());

    let cli = Cli::parse();

    let local_clipboard = LocalClipboard::new()?;
    let initial = local_clipboard.get().await?;
    let cancel_token = CancellationToken::new();

    match cli {
        Cli::Server { port } => {
            let server = server::SynclipServer::new(port, initial, cancel_token.clone()).await?;
            let mut clipboard = Clipboard::new(local_clipboard, server, 500, cancel_token.clone());
            let handle = clipboard.start();
            tokio::select! {
                _ = cancel_token.cancelled() => {}
                _ = tokio::signal::ctrl_c() => {
                    cancel_token.cancel();
                }
            }
            clipboard.shutdown().await?;
            info!("wait for server shutdown");
            handle.join().unwrap();
        }
        Cli::Client { address } => {
            let client = client::SynclipClient::new(address, initial, cancel_token.clone()).await?;
            let mut clipboard = Clipboard::new(local_clipboard, client, 500, cancel_token.clone());
            let handle = clipboard.start();
            tokio::select! {
                _ = cancel_token.cancelled() => {}
                _ = tokio::signal::ctrl_c() => {
                    cancel_token.cancel();
                }
            }
            clipboard.shutdown().await?;
            info!("wait for client shutdown");
            handle.join().unwrap();
        }
    }

    Ok(())
}
