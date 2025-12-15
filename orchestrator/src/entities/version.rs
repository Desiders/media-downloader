use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
