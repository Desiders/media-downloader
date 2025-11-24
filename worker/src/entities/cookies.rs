use std::{fmt::Display, path::PathBuf};
use url::Host;

#[derive(Debug, Clone)]
pub struct Cookie {
    pub host: Host,
    pub path: PathBuf,
}

#[derive(Debug, Default, Clone)]
pub struct Cookies {
    inner: Vec<Cookie>,
}

impl Cookies {
    pub fn add(&mut self, host: Host, path: PathBuf) {
        self.inner.push(Cookie { host, path });
    }

    pub fn get_by_host(&self, host: impl Display) -> Option<&Cookie> {
        let host = host.to_string();
        let host_stripped = if let Some(stripped) = host.strip_prefix("www.") {
            stripped
        } else {
            host.as_str()
        };
        self.inner
            .iter()
            .find(|cookie| cookie.host.to_string() == host_stripped)
    }

    pub fn get_by_optional_host(&self, host: Option<&Host<&str>>) -> Option<&Cookie> {
        host.and_then(|h| self.get_by_host(h))
    }

    pub fn get_all(&self) -> Vec<&Host> {
        self.inner.iter().map(|cookie| &cookie.host).collect()
    }
}
