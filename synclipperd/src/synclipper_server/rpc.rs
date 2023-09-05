use color_eyre::eyre::{eyre, format_err};
use std::pin::Pin;
use synclipper_proto::synclipper::syn_clipper_server::SynClipper;
use synclipper_proto::synclipper::{Content, Empty};
use tonic::codegen::futures_core::Stream;
use tonic::{Request, Response};

pub type ContentResult = Result<Content, tonic::Status>;
pub type ContentStream = Pin<Box<dyn Stream<Item = ContentResult> + Send>>;
pub type ContentSender = tokio::sync::mpsc::Sender<Content>;
pub type ContentReceiver = tokio::sync::watch::Receiver<ContentResult>;

pub struct SynClipperRpc {
    sender: ContentSender,
    receiver: ContentReceiver,
}

#[tonic::async_trait]
impl SynClipper for SynClipperRpc {
    type SyncClipboardStream = ContentStream;

    async fn sync_clipboard(
        &self,
        mut request: tonic::Request<Empty>,
    ) -> Result<tonic::Response<Self::SyncClipboardStream>, tonic::Status> {
        // while let content = request.get_mut().message().await {
        //     match content {
        //         Ok(contentj) => {}
        //         Err(status) => {
        //             eprintln!("Error: {:?}", status);
        //         }
        //     }
        // }
        Err(tonic::Status::unimplemented("Not implemented"))
    }

    async fn set_clipboard(&self, request: Request<Content>) -> Result<Response<Empty>, tonic::Status> {
        let content = request.into_inner();
        self.sender
            .send(content)
            .await
            .map_err(|e| tonic::Status::new(tonic::Code::Unavailable, format!("{:?}", eyre!(e))))?;
        Ok(Response::new(Empty {}))
    }
}
