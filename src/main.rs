use clap::Parser;
use color_eyre::Result;

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

    let cli = Cli::parse();

    let clipboard = Clipboard::new(500).await?;

    match cli {
        Cli::Server { port } => {
            let server = server::SynclipServer::new(port, clipboard.clone())?;
            tokio::signal::ctrl_c().await?;
            let (r1, r2) = tokio::join!(server.shutdown(), clipboard.shutdown());
            r1?;
            r2?;
        }
        Cli::Client { address } => {
            let client = client::SynclipClient::new(address, clipboard.clone()).await?;
            let mut client_clone = client.clone();
            let handle1 = tokio::spawn(async move { client_clone.polling_clipboard().await });
            let mut client_clone = client.clone();
            let handle2 = tokio::spawn(async move { client_clone.polling_server().await });
            tokio::signal::ctrl_c().await?;
            client.shutdown()?;
            let (r1, r2, r3) = tokio::join!(handle1, handle2, clipboard.shutdown());
            r1??;
            r2??;
            r3?;
        }
    }

    Ok(())
}
