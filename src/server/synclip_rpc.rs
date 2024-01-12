use std::pin::Pin;
use tokio_stream::StreamExt;

use tonic::codegen::tokio_stream::Stream;
use tonic::{Request, Response, Status};

use crate::proto::synclip_server::Synclip;
use crate::proto::{Content, Empty};

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

    async fn polling_clipboard(
        &self,
        _: Request<Empty>,
    ) -> Result<Response<Self::PollingClipboardStream>, Status> {
        let stream = Box::pin(
            self.clipboard
                .as_stream()
                .map(|content| Ok(Content { text: content })),
        );
        Ok(Response::new(stream))
    }

    async fn set_clipboard(&self, request: Request<Content>) -> Result<Response<Empty>, Status> {
        let content = request.into_inner();
        self.clipboard
            .set(content.text)
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::new(tonic::Code::Unavailable, format!("{:?}", e)))
    }
}
