use std::path::PathBuf;
use url::Host;

#[derive(Debug, Clone)]
pub struct Cookie {
    pub host: Host,
    pub path: PathBuf,
}
