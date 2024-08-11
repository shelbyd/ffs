use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

use clap::{Parser, Subcommand};
use command::ParseError;
use dashmap::DashMap;
use eyre::{Context, OptionExt};
use serde::Deserialize;
use target::Selector;

mod command;
mod target;

#[derive(Parser, Debug)]
struct Options {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Run { selector: Selector },
}

fn main() -> eyre::Result<()> {
    let options = Options::parse();

    match &options.command {
        Command::Run { selector: _ } => {
            run()?;
        }
    }

    Ok(())
}

fn run() -> eyre::Result<()> {
    // TODO(shelbyd): Search for root.
    let reader = Arc::new(Reader::new());

    let root = std::env::current_dir()?;
    let mut builder = Builder::new(Arc::clone(&reader), &root);

    let mut count = 0;

    for entry in ignore::Walk::new(".") {
        let entry = entry?;

        let is_ffs_file = entry.path().file_name().is_some_and(|f| f == "FFS");
        if !is_ffs_file {
            continue;
        }

        let file = reader.read(entry.path())?;
        for (name, test) in file.tests() {
            // TODO(shelbyd): Actual target name.
            let message = format!("Executing {}/{name}", entry.path().display());
            test.execute(entry.path().parent().expect("entry is file"), &mut builder)
                .context(message)?;
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
    fn tests(&self) -> impl Iterator<Item = (&String, &Test)> {
        self.tests.iter()
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Test {
    cmd: String,
}

impl Test {
    fn execute(&self, pwd: &Path, builder: &mut Builder) -> eyre::Result<()> {
        let command = builder.parse_command(&self.cmd)?;

        let output = std::process::Command::new("sh")
            .current_dir(pwd)
            .env_clear()
            .arg("-c")
            .arg(command)
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
    source: ToolSource,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ToolSource {
    System(PathBuf),
    Target { cmd: String, bin: PathBuf },
}

struct Builder {
    reader: Arc<Reader>,

    root: PathBuf,
    targets: DashMap<String, PathBuf>,
}

impl Builder {
    fn new(reader: Arc<Reader>, root: impl AsRef<Path>) -> Self {
        Self {
            reader,

            root: root.as_ref().to_path_buf(),
            targets: Default::default(),
        }
    }

    fn parse_command(&mut self, c: &str) -> eyre::Result<String> {
        loop {
            match command::parse_command(c, &self.targets) {
                Ok(c) => return Ok(c),
                Err(ParseError::UnknownTarget(t)) => {
                    self.build(&t).context(format!("Building {t:?}"))?
                }
            }
        }
    }

    fn build(&mut self, target: &str) -> eyre::Result<()> {
        let definition = self.root.join(target::path_to_definition(target)?);
        let file = self.reader.read(&definition)?;

        let name = target::name(target)?;

        let tool = file
            .tools
            .get(name)
            .ok_or_eyre(format!("Unknown tool: {target:?}"))?;

        match &tool.source {
            ToolSource::System(name) => {
                // TODO(shelbyd): Allow actual paths.
                let path = String::from_utf8(
                    std::process::Command::new("which")
                        .arg(name)
                        .output()?
                        .stdout,
                )?;
                self.targets.insert(target.to_string(), path.trim().into());
            }

            ToolSource::Target { cmd, bin } => {
                let dir = definition.parent().unwrap();
                let command = self.parse_command(cmd)?;
                let output = std::process::Command::new("sh")
                    .current_dir(dir)
                    .env_clear()
                    .arg("-c")
                    .arg(command)
                    .output()?;

                eyre::ensure!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );

                self.targets.insert(target.to_string(), dir.join(bin));
            }
        }

        Ok(())
    }
}

struct Reader {
    cache: DashMap<PathBuf, Arc<FfsFile>>,
}

impl Reader {
    fn new() -> Self {
        Self {
            cache: Default::default(),
        }
    }

    fn read(&self, path: impl AsRef<Path>) -> eyre::Result<Arc<FfsFile>> {
        match self.cache.entry(path.as_ref().to_path_buf()) {
            dashmap::Entry::Occupied(o) => Ok(Arc::clone(o.get())),
            dashmap::Entry::Vacant(v) => {
                let file: FfsFile = toml::from_str(&std::fs::read_to_string(path)?)?;
                let f = v.insert(Arc::new(file));
                Ok(Arc::clone(&f))
            }
        }
    }
}
