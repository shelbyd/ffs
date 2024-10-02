use std::str::FromStr;

pub struct RelativeTarget {}

impl FromStr for RelativeTarget {
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
        let valid = ["%/target"];

        for t in valid {
            assert!(
                t.parse::<RelativeTarget>().is_ok(),
                "{t:?} failed parsing as RelativeTarget"
            );
        }
    }
}
