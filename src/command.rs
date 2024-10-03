use std::{borrow::Borrow, path::PathBuf, str::FromStr};

use dashmap::DashMap;
use eyre::OptionExt;

use crate::target::{Output, TargetPath};

pub struct Command {
    words: Vec<Word>,
}

impl Command {
    pub fn targets(&self) -> impl Iterator<Item = impl Borrow<TargetPath> + '_> {
        self.words
            .iter()
            .filter_map(|s| match s {
                Word::Output(o) => Some(o),
                _ => None,
            })
            .map(|o| o.target())
    }

    pub fn as_sh(&self, outputs: &DashMap<Output, PathBuf>) -> eyre::Result<String> {
        Ok(self
            .words
            .iter()
            .map(|w| {
                let output = match w {
                    Word::Lit(s) => return Ok(s.to_string()),
                    Word::Output(o) => o,
                };

                let path = outputs
                    .get(&output)
                    .ok_or_eyre(format!("Missing output {output}"))?;

                Ok(path
                    .to_str()
                    .ok_or_eyre(format!("Path not utf8 {}", path.display()))?
                    .to_string())
            })
            .collect::<eyre::Result<Vec<_>>>()?
            .join(""))
    }
}

impl FromStr for Command {
    type Err = eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut words = Vec::new();

        let pat = &[' ', '\n'];

        for s in s.split_inclusive(pat) {
            let trimmed = s.trim_end_matches(pat);
            match trimmed.parse() {
                Ok(o) => {
                    words.push(Word::Output(o));
                    words.push(Word::Lit(s[trimmed.len()..].to_string()));
                }
                Err(_) => words.push(Word::Lit(s.to_string())),
            }
        }

        Ok(Command { words })
    }
}

enum Word {
    Lit(String),
    Output(Output),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target_strings(c: &Command) -> Vec<String> {
        c.targets().map(|t| t.borrow().to_string()).collect()
    }

    fn map<'s>(i: impl IntoIterator<Item = (&'s str, &'s str)>) -> DashMap<Output, PathBuf> {
        i.into_iter()
            .map(|(out, path)| (out.parse().unwrap(), PathBuf::from(path)))
            .collect()
    }

    #[test]
    fn simple_command() {
        let c = "echo 'foo'".parse::<Command>().unwrap();

        assert_eq!(target_strings(&c), &[] as &[&str]);
        assert_eq!(c.as_sh(&map([])).unwrap(), "echo 'foo'");
    }

    #[test]
    fn output_as_arg() {
        let c = "cat //path/to/target:output".parse::<Command>().unwrap();

        assert_eq!(target_strings(&c), &["//path/to/target"]);
        assert_eq!(
            c.as_sh(&map([("//path/to/target:output", "path/to/file")]))
                .unwrap(),
            "cat path/to/file",
        );
    }

    #[test]
    fn output_as_command() {
        let c = "//path/to/target:cmd arg1 arg2".parse::<Command>().unwrap();

        assert_eq!(target_strings(&c), &["//path/to/target"]);
        assert_eq!(
            c.as_sh(&map([("//path/to/target:cmd", "path/to/file")]))
                .unwrap(),
            "path/to/file arg1 arg2",
        );
    }

    #[test]
    fn multiple_lines() {
        let c = "echo foo\n//some/target bar".parse::<Command>().unwrap();

        assert_eq!(
            c.as_sh(&map([("//some/target", "some/target")])).unwrap(),
            "echo foo\nsome/target bar",
        );
    }
}
