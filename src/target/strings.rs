use std::{collections::HashSet, path::Path, str::FromStr};

pub fn path_to_definition(target: &str) -> eyre::Result<String> {
    let target = target
        .strip_prefix("//")
        .ok_or_else(|| eyre::eyre!("Expected {target:?} to start with //"))?;

    let Some((pre, _)) = target.rsplit_once("/") else {
        return Ok(String::from("FFS"));
    };

    Ok(format!("{pre}/FFS"))
}

pub fn name(target: &str) -> eyre::Result<&str> {
    Ok(target
        .rsplit_once("/")
        .ok_or_else(|| eyre::eyre!("Expected {target:?} to contain a /"))?
        .1)
}

#[derive(Clone, Debug, Default)]
pub struct Selector {
    target: String,
    allow_children: bool,
    required_tags: HashSet<String>,
}

impl Selector {
    pub fn matches<T>(&self, task_id: &str, tags: &HashSet<T>) -> bool
    where
        T: std::borrow::Borrow<str> + Eq + std::hash::Hash,
    {
        for req in &self.required_tags {
            if !tags.contains(req.as_str()) {
                return false;
            }
        }

        let Some(child) = task_id.strip_prefix(&self.target) else {
            return false;
        };

        if child.is_empty() {
            return true;
        }

        if self.allow_children {
            return child.starts_with("/");
        }

        false
    }

    pub(crate) fn matches_file(&self, path: impl AsRef<Path>) -> bool {
        let path = std_to_ffs(path);

        if self.allow_children {
            return path.starts_with(&self.target);
        }

        let (target_parent, _) = self.target.rsplit_once("/").unwrap();
        path == target_parent || (path == "//" && target_parent == "/")
    }
}

impl FromStr for Selector {
    type Err = eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut result = Selector::default();

        let s = if let Some((s, tags)) = s.split_once("@") {
            result.required_tags = tags.split(",").map(ToString::to_string).collect();
            s
        } else {
            s
        };

        if matches!(s, "*" | "") {
            result.target = "/".to_string();
            result.allow_children = true;
            return Ok(result);
        }

        eyre::ensure!(s.starts_with("//"));

        if let Some(parent) = s.strip_suffix("/...") {
            result.target = parent.to_string();
            result.allow_children = true;
            return Ok(result);
        }

        result.target = s.to_string();
        Ok(result)
    }
}

pub fn task_path(file_or_dir: impl AsRef<Path>, name: &str) -> String {
    let mut result = std_to_ffs(file_or_dir);

    if !result.ends_with("/") {
        result += "/";
    }
    result += name;

    result
}

fn std_to_ffs(file_or_dir: impl AsRef<Path>) -> String {
    let file_or_dir = file_or_dir.as_ref();
    assert!(
        file_or_dir.is_relative(),
        "Expected {} to be relative",
        file_or_dir.display()
    );

    let without_ffs = if file_or_dir.file_name().is_some_and(|f| f == "FFS") {
        file_or_dir.parent().unwrap()
    } else {
        file_or_dir
    };

    let path = without_ffs.strip_prefix("./").unwrap_or(without_ffs);

    format!("//{}", path.display()).replace("///", "//")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root() {
        assert_eq!(path_to_definition("//target").unwrap(), "FFS");
    }

    #[test]
    fn subdir() {
        assert_eq!(
            path_to_definition("//path/to/target").unwrap(),
            "path/to/FFS"
        );
    }

    fn selector_matches<'a>(
        sel: &str,
        target: &str,
        tags: impl IntoIterator<Item = &'a str>,
    ) -> bool {
        let sel = sel.parse::<Selector>().unwrap();
        sel.matches(target, &tags.into_iter().collect())
    }

    #[test]
    fn selector_star_matches_everything() {
        assert!(selector_matches("*", "//some/target", []));
    }

    #[test]
    fn selector_exact_does_not_match_other() {
        assert!(!selector_matches("//a/target", "//another/target", []));
    }

    #[test]
    fn selector_matches_exact() {
        assert!(selector_matches("//a/target", "//a/target", []));
    }

    #[test]
    fn glob_matches_children() {
        assert!(selector_matches(
            "//some/path/...",
            "//some/path/actual_target",
            []
        ));
    }

    #[test]
    fn glob_does_not_match_sibling_directory() {
        assert!(!selector_matches(
            "//some/path/...",
            "//some/path_suffix/actual_target",
            []
        ));
    }

    #[test]
    fn matches_with_tags() {
        assert!(selector_matches("@test", "//some/target", ["test"]));
    }

    #[test]
    fn does_not_match_without_tags() {
        assert!(!selector_matches("@test", "//some/target", ["deploy"]));
    }

    #[test]
    fn matches_with_all_tags() {
        assert!(selector_matches(
            "@test,deploy",
            "//some/target",
            ["deploy", "test"]
        ));
    }

    #[test]
    fn does_not_match_with_some_tags() {
        assert!(!selector_matches(
            "@test,deploy",
            "//some/target",
            ["deploy"]
        ));
    }

    #[test]
    fn exact_does_not_match_child() {
        assert!(!selector_matches("//a/target", "//a/target/child", []));
    }

    #[test]
    fn bad_target_specifier() {
        assert!("bad/target".parse::<Selector>().is_err());
    }

    fn selector_matches_file<'a>(sel: &str, file: &str) -> bool {
        let sel = sel.parse::<Selector>().unwrap();
        sel.matches_file(file)
    }

    #[test]
    fn exact_matches_file() {
        assert!(selector_matches_file("//path/to/target", "./path/to/FFS"));
    }

    #[test]
    fn exact_but_different_file() {
        assert!(!selector_matches_file(
            "//path/to/target",
            "./path/elsewhere/FFS"
        ));
    }

    #[test]
    fn child_file_match() {
        assert!(selector_matches_file(
            "//path/to/...",
            "./path/to/some/child/FFS"
        ));
    }

    #[test]
    fn poorly_named_sibling() {
        assert!(!selector_matches_file(
            "//path/to_elsewhere/target",
            "./path/to/FFS"
        ));
    }

    #[test]
    fn root_file() {
        assert!(selector_matches_file("//root_target", "./FFS"));
    }

    #[test]
    fn task_path_() {
        assert_eq!(task_path("./FFS", "task"), "//task");
        assert_eq!(task_path("path/to", "task"), "//path/to/task");
        assert_eq!(task_path("path/to/", "task"), "//path/to/task");
        assert_eq!(task_path("path/to/FFS", "task"), "//path/to/task");
        assert_eq!(task_path("./path/to/FFS", "task"), "//path/to/task");
    }
}
