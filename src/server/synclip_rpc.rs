use std::pin::Pin;
use tokio_stream::StreamExt;

use tonic::codegen::tokio_stream::Stream;
use tonic::{Request, Response, Status};

use synclip_proto::synclip_server::Synclip;
use synclip_proto::{Content, Empty};

use crate::clipboard::Clipboard;

pub type ContentResult = Result<Content, Status>;
type ContentStream = Pin<Box<dyn Stream<Item = ContentResult> + Send>>;

pub struct SynclipRpc {
    clipboard: Clipboard,
}

impl SynclipRpc {
    pub fn new(clipboard: Clipboard) -> Self {
        Self { clipboard }
    }
}

#[tonic::async_trait]
impl Synclip for SynclipRpc {
    type PollingClipboardStream = ContentStream;

    async fn polling_clipboard(&self, _: Request<Empty>) -> Result<Response<Self::PollingClipboardStream>, Status> {
        let stream = Box::pin(self.clipboard.as_stream().map(|content| Ok(Content { text: content })));
        Ok(Response::new(stream))
    }

    async fn set_clipboard(&self, request: Request<Content>) -> Result<Response<Empty>, Status> {
        let content = request.into_inner();
        if let Some(content) = content.text {
            self.clipboard
                .set(content)
                .map(|_| Response::new(Empty {}))
                .map_err(|e| Status::new(tonic::Code::Unavailable, format!("{:?}", e)))
        } else {
            Ok(Response::new(Empty {}))
        }
    }

    // async fn sync_clipboard(
    //     &self,
    //     _request: tonic::Request<Empty>,
    // ) -> Result<tonic::Response<Self::SyncClipboardStream>, tonic::Status> {
    //     Ok(tonic::Response::new(Box::pin(WatchStream::new(self.receiver.clone()))))
    // }
    //
    // async fn send_clipboard(&self, request: tonic::Request<Content>) -> Result<tonic::Response<Empty>, tonic::Status> {
    //     let content = request.into_inner();
    //     self.sender
    //         .send(Some(content))
    //         .map_err(|e| tonic::Status::new(tonic::Code::Unavailable, format!("{:?}", eyre!(e))))?;
    //     Ok(tonic::Response::new(Empty {}))
    // }
}
