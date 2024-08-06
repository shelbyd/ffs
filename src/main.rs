use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
};

use clap::{Parser, Subcommand};
use serde::Deserialize;

#[derive(Parser, Debug)]
struct Options {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Test,
    // Build,
    // Run,
}

fn main() -> eyre::Result<()> {
    let options = Options::parse();

    match &options.command {
        Command::Test => {
            run_tests()?;
        }
    }

    Ok(())
}

fn run_tests() -> eyre::Result<()> {
    let mut count = 0;

    for entry in ignore::Walk::new(".") {
        let entry = entry?;

        let is_ffs_file = entry.path().file_name().is_some_and(|f| f == "FFS");
        if !is_ffs_file {
            continue;
        }

        let file: FfsFile = toml::from_str(&std::fs::read_to_string(entry.path())?)?;
        for test in file.tests() {
            test.execute(entry.path().parent().expect("entry is file"))?;
            count += 1;
        }
    }

    eprintln!("{count} tests passed");

    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FfsFile {
    #[serde(default)]
    tests: BTreeMap<String, Test>,

    #[serde(default)]
    #[allow(unused)]
    tools: BTreeMap<String, Tool>,
}

impl FfsFile {
    fn tests(&self) -> impl Iterator<Item = &Test> {
        self.tests.values()
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Test {
    cmd: String,
}

impl Test {
    fn execute(&self, pwd: &Path) -> eyre::Result<()> {
        let output = std::process::Command::new("sh")
            .current_dir(pwd)
            .arg("-c")
            .arg(&self.cmd)
            .output()?;

        if output.status.success() {
            return Ok(());
        }

        std::io::stderr().lock().write_all(&output.stderr)?;
        std::io::stdout().lock().write_all(&output.stdout)?;

        eyre::bail!("Test command failed");
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Tool {
    #[serde(default, rename = "pub")]
    #[allow(unused)]
    pub_: bool,

    #[allow(unused)]
    source: ToolSource,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ToolSource {
    #[allow(unused)]
    System(PathBuf),
}
