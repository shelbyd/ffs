use std::{fmt::Display, str::FromStr};

pub struct TargetPath {
    dir: Option<String>,
    name: String,
}

impl TargetPath {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn definition(&self) -> String {
        match &self.dir {
            Some(d) => format!("{d}/FFS"),
            None => "FFS".to_string(),
        }
    }
}

impl FromStr for TargetPath {
    type Err = eyre::Report;

    #[context_attr::eyre("Parsing {s:?} as Target")]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some(pre) = s.strip_prefix("//") else {
            eyre::bail!("Target must start with //");
        };
        eyre::ensure!(!pre.contains("//"));

        let invalid_char = pre.chars().find(|c| !(c.is_alphanumeric() || *c == '/'));
        if let Some(c) = invalid_char {
            eyre::bail!("Invalid character: {c:?}");
        }

        let (dir, name) = match pre.rsplit_once("/") {
            Some((dir, name)) => (Some(dir), name),
            None => (None, pre),
        };

        eyre::ensure!(!name.is_empty());

        Ok(TargetPath {
            dir: dir.map(ToString::to_string),
            name: name.to_string(),
        })
    }
}

impl Display for TargetPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.dir {
            Some(d) => write!(f, "//{d}/{}", self.name),
            None => write!(f, "//{}", self.name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_parsing() {
        let cases = ["//target", "//path/to/target"];

        for t in cases {
            if let Err(e) = t.parse::<TargetPath>() {
                panic!("{t:?} failed parsing as Target: {e}");
            }
        }
    }

    #[test]
    fn invalid_parsing() {
        let cases = [
            "/target",
            "//path:other",
            "//path@tag",
            "//trailing/slash/",
            "//empty//dir",
        ];

        for t in cases {
            if let Ok(_) = t.parse::<TargetPath>() {
                panic!("{t:?} parsed as Target but should have failed");
            }
        }
    }

    #[test]
    fn name() {
        assert_eq!("//target".parse::<TargetPath>().unwrap().name(), "target");
        assert_eq!(
            "//path/to/target".parse::<TargetPath>().unwrap().name(),
            "target"
        );
    }

    #[test]
    fn definition() {
        assert_eq!(
            "//target".parse::<TargetPath>().unwrap().definition(),
            "FFS"
        );
        assert_eq!(
            "//path/to/target"
                .parse::<TargetPath>()
                .unwrap()
                .definition(),
            "path/to/FFS"
        );
    }
}
