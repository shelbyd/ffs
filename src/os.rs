use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Linux,
    Mac,
    Windows,
}

impl FromStr for Os {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = None
            .or(s.strip_suffix("_os"))
            .or(s.strip_suffix("os"))
            .unwrap_or(s);

        Ok(match s {
            "linux" => Os::Linux,
            "mac" => Os::Mac,
            "windows" => Os::Windows,

            s => eyre::bail!("Unknown OS: {s:?}"),
        })
    }
}

#[cfg(target_os = "linux")]
pub fn host() -> Os {
    Os::Linux
}

#[cfg(target_os = "macos")]
pub fn host() -> Os {
    Os::Mac
}

#[cfg(target_os = "windows")]
pub fn host() -> Os {
    Os::Windows
}
