use clipboard::ClipboardProvider;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::error;

type ClipboardContext = Arc<Mutex<clipboard::ClipboardContext>>;

#[derive(Clone)]
pub struct LocalClipboard {
    context: ClipboardContext,
}

impl LocalClipboard {
    pub fn new() -> Result<Self> {
        let context = ClipboardProvider::new().map_err(|e| eyre!("{:?}", e))?;
        let context = Arc::new(Mutex::new(context));
        Ok(Self { context })
    }

    pub async fn set(&self, content: impl AsRef<str>) -> Result<bool> {
        let mut context = self.context.lock().await;
        let current = context.get_contents();
        let replaced = current.map(|c| c != content.as_ref()).unwrap_or(true);
        if let (false, Err(e)) = (replaced, context.set_contents(content.as_ref().to_string())) {
            error!("Set [Local] error: {:?}", e);
        }
        Ok(replaced)
    }

    pub async fn get(&self) -> Result<String> {
        let mut context = self.context.lock().await;
        context.get_contents().map_err(|e| eyre!("{:?}", e))
    }
}
