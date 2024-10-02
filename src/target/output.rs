use std::str::FromStr;

pub struct Output {}

impl FromStr for Output {
    type Err = eyre::Report;

    fn from_str(_: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn parsing() {
        let valid = ["//target:output"];

        for t in valid {
            assert!(
                t.parse::<Output>().is_ok(),
                "{t:?} failed parsing as Output"
            );
        }
    }
}
