use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Сеть: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Файловая система: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Zip: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Хранилище учётных данных: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("{0}")]
    Msg(String),
}

impl Error {
    pub fn msg(s: impl Into<String>) -> Self {
        Error::Msg(s.into())
    }
}

impl Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;