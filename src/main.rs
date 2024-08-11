use std::{
    collections::{BTreeMap, HashMap, HashSet},
    io::Write,
    path::{Path, PathBuf},
    process::Output,
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
        Command::Run { selector } => {
            run(&selector)?;
        }
    }

    Ok(())
}

fn run(selector: &Selector) -> eyre::Result<()> {
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
        for (name, task) in file.tasks() {
            let task_path = task_path(entry.path(), name);

            if !selector.matches(&task_path, &task.tags) {
                continue;
            }

            let message = format!("Executing {task_path}");
            let output = builder
                .execute(task, entry.path().parent().expect("entry is file"))
                .context(message)?;

            if !output.status.success() {
                std::io::stdout().lock().write_all(&output.stdout)?;
                std::io::stderr().lock().write_all(&output.stderr)?;
                eyre::bail!("Task failed: {task_path}");
            }

            eprintln!("Task finished: {task_path}");

            count += 1;
        }
    }

    eprintln!("Successfully ran {count} tasks");

    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FfsFile(BTreeMap<String, Task>);

impl FfsFile {
    fn tasks(&self) -> impl Iterator<Item = (&String, &Task)> {
        self.0.iter()
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Task {
    cmd: String,

    #[serde(default)]
    tags: HashSet<String>,

    #[serde(default)]
    outs: HashMap<String, PathBuf>,
}

struct Builder {
    reader: Arc<Reader>,

    root: PathBuf,
    target_outs: DashMap<String, PathBuf>,
}

impl Builder {
    fn new(reader: Arc<Reader>, root: impl AsRef<Path>) -> Self {
        Self {
            reader,

            root: root.as_ref().to_path_buf(),
            target_outs: Default::default(),
        }
    }

    fn parse_command(&mut self, c: &str) -> eyre::Result<String> {
        loop {
            match command::parse_command(c, &self.target_outs) {
                Ok(c) => return Ok(c),
                Err(ParseError::UnknownTarget(t)) => {
                    self.build(&t).context(format!("Building {t:?}"))?;
                }
            }
        }
    }

    fn build(&mut self, target: &str) -> eyre::Result<()> {
        let definition = self.root.join(target::path_to_definition(target)?);
        let file = self.reader.read(&definition)?;

        let name = target::name(target)?;

        let task = file
            .0
            .get(name)
            .ok_or_eyre(format!("Unknown task: {target:?}"))?;

        let dir = definition.parent().unwrap();
        let relative_dir = dir.strip_prefix(&self.root).unwrap();

        let task_path = task_path(&relative_dir, name);

        let output = self.execute(task, &dir)?;

        if !output.status.success() {
            eyre::bail!("Build failed: {task_path}")
        }

        if let Some(path) = task.outs.get("default") {
            let file = dir.join(path);
            eyre::ensure!(file.exists());

            self.target_outs.insert(task_path, file);
        }

        Ok(())
    }

    fn execute(&mut self, task: &Task, dir: &Path) -> eyre::Result<Output> {
        let command = self.parse_command(&task.cmd)?;
        Ok(std::process::Command::new("sh")
            .current_dir(dir)
            .arg("-c")
            .arg(command)
            .output()?)
    }
}

fn task_path(file_or_dir: &Path, name: &str) -> String {
    assert!(file_or_dir.is_relative());

    let without_ffs = if file_or_dir.file_name().is_some_and(|f| f == "FFS") {
        file_or_dir.parent().unwrap()
    } else {
        file_or_dir
    };

    let path = without_ffs.strip_prefix("./").unwrap_or(without_ffs);

    format!("//{}/{name}", path.display())
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
