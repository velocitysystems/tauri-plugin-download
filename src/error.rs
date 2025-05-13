use serde::{ser::Serializer, Serialize};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
   #[error("Invalid State")]
   InvalidState,

   #[error("Store Error: {0}")]
   Store(String),

   #[error("File Error: {0}")]
   File(String),

   #[error("HTTP Error: {0}")]
   Http(String),

   #[error(transparent)]
   Io(#[from] std::io::Error),

   #[cfg(mobile)]
   #[error(transparent)]
   PluginInvoke(#[from] tauri::plugin::mobile::PluginInvokeError),
}

impl Serialize for Error {
   fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
   where
      S: Serializer,
   {
      serializer.serialize_str(self.to_string().as_ref())
   }
}
