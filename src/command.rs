use std::path::PathBuf;

use dashmap::DashMap;

pub fn parse_command(
    c: &str,
    target_locations: &DashMap<String, PathBuf>,
) -> Result<String, ParseError> {
    Ok(c.split(" ")
        .map(|target| {
            if !target.starts_with("//") {
                return Ok(target.to_string());
            }

            let Some(t) = target_locations.get(target) else {
                return Err(ParseError::UnknownTarget(target.to_string()));
            };

            Ok(t.to_str().expect("path is not utf-8").to_string())
        })
        .collect::<Result<Vec<_>, _>>()?
        .join(" "))
}

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum ParseError {
    #[error("Unknown target {0}")]
    UnknownTarget(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_command() {
        let command = parse_command("echo 'foo'", &Default::default()).unwrap();
        assert_eq!(command, "echo 'foo'");
    }

    #[test]
    fn command_includes_tool() {
        let command = parse_command(
            "//some_tool foo",
            &[("//some_tool", "some/tool/location")]
                .into_iter()
                .map(|(a, b)| (a.to_string(), b.into()))
                .collect(),
        )
        .unwrap();
        assert_eq!(command, "some/tool/location foo");
    }

    #[test]
    fn just_command_no_args() {
        let command = parse_command(
            "//some_tool",
            &[("//some_tool", "some/tool/location")]
                .into_iter()
                .map(|(a, b)| (a.to_string(), b.into()))
                .collect(),
        )
        .unwrap();

        assert_eq!(command, "some/tool/location");
    }

    #[test]
    fn arg_uses_reference() {
        let command = parse_command(
            "system_tool //another/target",
            &[("//another/target", "target/location")]
                .into_iter()
                .map(|(a, b)| (a.to_string(), b.into()))
                .collect(),
        )
        .unwrap();

        assert_eq!(command, "system_tool target/location");
    }

    #[test]
    fn missing_target() {
        let result = parse_command("system_tool //another/target", &Default::default());
        assert_eq!(
            result,
            Err(ParseError::UnknownTarget("//another/target".to_string()))
        );
    }

    #[test]
    fn does_not_replace_non_target() {
        let result = parse_command(
            "not_a_target arg",
            &[("not_a_target", "should_not_be_here")]
                .into_iter()
                .map(|(a, b)| (a.to_string(), b.into()))
                .collect(),
        )
        .unwrap();

        assert_eq!(result, "not_a_target arg");
    }
}
