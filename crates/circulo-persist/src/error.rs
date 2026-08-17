use circulo_core::DomainError;

#[derive(Debug)]
pub enum PersistError {
    Sqlite(rusqlite::Error),
    Serde(serde_json::Error),
    Domain(DomainError),
    Time(time::error::Parse),
    Io(std::io::Error),
    NotFound,
    InvalidHome,
    AgentBindingLocked,
}

impl std::fmt::Display for PersistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sqlite(err) => write!(f, "sqlite: {err}"),
            Self::Serde(err) => write!(f, "serde: {err}"),
            Self::Domain(err) => write!(f, "domain: {}", err.code()),
            Self::Time(err) => write!(f, "time: {err}"),
            Self::Io(err) => write!(f, "io: {err}"),
            Self::NotFound => write!(f, "not found"),
            Self::InvalidHome => write!(f, "HOME is not set"),
            Self::AgentBindingLocked => write!(f, "agent session binding already set"),
        }
    }
}

impl std::error::Error for PersistError {}

impl From<rusqlite::Error> for PersistError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}

impl From<serde_json::Error> for PersistError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}

impl From<time::error::Parse> for PersistError {
    fn from(value: time::error::Parse) -> Self {
        Self::Time(value)
    }
}

impl From<std::io::Error> for PersistError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
