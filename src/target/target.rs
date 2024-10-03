use std::{fmt::Display, path::Path, str::FromStr};

pub fn ident(s: &str) -> eyre::Result<&str> {
    let invalid_char = s
        .chars()
        .find(|c| !(c.is_alphanumeric() || matches!(c, '_' | '-')));
    if let Some(c) = invalid_char {
        eyre::bail!("Invalid ident char {c:?}");
    }
    Ok(s)
}

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

    #[context_attr::eyre("Constructing path from {path:?} + {name}")]
    pub fn from_path_name(path: &Path, name: &str) -> eyre::Result<TargetPath> {
        let mut path = path.strip_prefix("./").unwrap_or(path);
        if path.ends_with("FFS") {
            path = path.parent().unwrap();
        }

        let Some(path) = path.to_str() else {
            eyre::bail!("Path not utf-8");
        };

        let path = path.strip_suffix("/").unwrap_or(path);

        Ok(TargetPath {
            dir: if path.is_empty() {
                None
            } else {
                Some(path.to_string())
            },
            name: name.to_string(),
        })
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

        let path = pre
            .split("/")
            .map(ident)
            .collect::<Result<Vec<_>, _>>()?
            .join("/");

        let (dir, name) = match path.rsplit_once("/") {
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
        let cases = ["//target", "//path/to/target", "//allowed/characters_-"];

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

    #[test]
    fn from_path_name() {
        fn target_path(p: &str, name: &str) -> String {
            TargetPath::from_path_name(Path::new(p), name)
                .unwrap()
                .to_string()
        }

        assert_eq!(target_path("./FFS", "task"), "//task");
        assert_eq!(target_path("path/to", "task"), "//path/to/task");
        assert_eq!(target_path("path/to/", "task"), "//path/to/task");
        assert_eq!(target_path("path/to/FFS", "task"), "//path/to/task");
        assert_eq!(target_path("./path/to/FFS", "task"), "//path/to/task");
        assert_eq!(
            target_path("./path/to/fakeFFS", "task"),
            "//path/to/fakeFFS/task"
        );
    }
}
