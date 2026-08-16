use std::net::SocketAddr;

pub const DEFAULT_ADDR: &str = "127.0.0.1:7432";

#[derive(Debug)]
pub enum BindError {
    Parse(std::net::AddrParseError),
    NotLoopback(SocketAddr),
}

impl std::fmt::Display for BindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(err) => write!(f, "invalid listen address: {err}"),
            Self::NotLoopback(addr) => {
                write!(f, "daemon must bind loopback only, refused {addr}")
            }
        }
    }
}

impl std::error::Error for BindError {}

impl From<std::net::AddrParseError> for BindError {
    fn from(value: std::net::AddrParseError) -> Self {
        Self::Parse(value)
    }
}

pub fn listen_addr(raw: Option<&str>) -> Result<SocketAddr, BindError> {
    let addr: SocketAddr = raw.unwrap_or(DEFAULT_ADDR).parse()?;
    if !addr.ip().is_loopback() {
        return Err(BindError::NotLoopback(addr));
    }
    Ok(addr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_loopback() {
        let addr = listen_addr(None).unwrap();
        assert!(addr.ip().is_loopback());
        assert_eq!(addr.port(), 7432);
    }

    #[test]
    fn rejects_unspecified() {
        let err = listen_addr(Some("0.0.0.0:7432")).unwrap_err();
        assert!(matches!(err, BindError::NotLoopback(_)));
    }
}
