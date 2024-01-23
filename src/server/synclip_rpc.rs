use std::pin::Pin;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use tokio_stream::StreamExt;

use tonic::codegen::tokio_stream::Stream;
use tonic::{Request, Response, Status};

use crate::proto::synclip_server::Synclip;
use crate::proto::{Content, Empty};
use crate::Replaced;

pub type ContentResult = Result<Content, Status>;
type ContentStream = Pin<Box<dyn Stream<Item = ContentResult> + Send>>;

pub struct SynclipRpc {
    sender: watch::Sender<String>,
    receiver: watch::Receiver<String>,
}

impl SynclipRpc {
    pub fn new(sender: watch::Sender<String>, receiver: watch::Receiver<String>) -> Self {
        Self { sender, receiver }
    }
}

#[tonic::async_trait]
impl Synclip for SynclipRpc {
    type PollingClipboardStream = ContentStream;

    async fn polling_clipboard(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::PollingClipboardStream>, Status> {
        let stream = Box::pin(
            WatchStream::new(self.receiver.clone()).map(|content| Ok(Content { text: content })),
        );
        Ok(Response::new(stream))
    }

    async fn set_clipboard(&self, request: Request<Content>) -> Result<Response<Replaced>, Status> {
        let content = request.into_inner().text;
        let replaced = self.sender.send_if_modified(|prev| {
            if prev != &content {
                *prev = content;
                true
            } else {
                false
            }
        });
        Ok(Response::new(Replaced { replaced }))
    }
}
