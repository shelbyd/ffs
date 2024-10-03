use std::str::FromStr;

use super::{ident, TargetPath};

pub struct Output {
    target: TargetPath,
    name: String,
}

impl Output {
    pub fn target(&self) -> &TargetPath {
        &self.target
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl FromStr for Output {
    type Err = eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (target, name) = s.split_once(":").unwrap_or((s, "default"));

        let name = ident(name)?;

        Ok(Output {
            target: target.parse()?,
            name: name.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_parses() {
        let cases = ["//target:output"];

        for t in cases {
            if let Err(e) = t.parse::<Output>() {
                panic!("{t:?} failed parsing as Output: {e}");
            }
        }
    }

    #[test]
    fn invalid_parses() {
        let cases = ["//target@output", "//target:path/bad"];

        for t in cases {
            assert!(
                t.parse::<Output>().is_err(),
                "{t:?} parsed as Output, but should have failed"
            );
        }
    }

    #[test]
    fn provides_various_fields() {
        let output = "//path/to/target:output".parse::<Output>().unwrap();

        assert_eq!(output.target().to_string(), "//path/to/target");
        assert_eq!(output.name(), "output");
    }

    #[test]
    fn missing_name_is_default() {
        let output = "//path/to/target".parse::<Output>().unwrap();

        assert_eq!(output.target().to_string(), "//path/to/target");
        assert_eq!(output.name(), "default");
    }
}
