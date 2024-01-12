pub mod client;
pub mod clipboard;
pub mod server;

mod proto {
    tonic::include_proto!("synclip");
}

pub use proto::*;
