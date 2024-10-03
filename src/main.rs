use std::{
    io::Write,
    path::{Path, PathBuf},
    process::Output,
    sync::Arc,
};

use clap::{Parser, Subcommand};
use command::ParseError;
use dashmap::DashMap;
use executor::{Execution, Executor};
use eyre::OptionExt;
use reporting::{build_reporter, Reporter};
use starlark::Reader;
use target::{task_path, Selector, Target, TargetPath};

mod command;
mod executor;
mod os;
mod reporting;
mod starlark;
mod target;

#[derive(Parser, Debug)]
struct Options {
    #[command(flatten)]
    reporting: reporting::Options,

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
            let reporter = build_reporter(&options.reporting);
            run(&selector, reporter)?;
        }
    }

    Ok(())
}

fn run(selector: &Selector, reporter: Arc<dyn Reporter>) -> eyre::Result<()> {
    let executor = Arc::new(Executor::new(Arc::clone(&reporter)));

    // TODO(shelbyd): Search for root.
    let root = std::env::current_dir()?;
    let reader = Arc::new(Reader::new(&root));

    let mut builder = Builder::new(Arc::clone(&reader), Arc::clone(&executor), &root);

    let mut count = 0;
    for entry in ignore::Walk::new(".") {
        let entry = entry?;

        let is_ffs_file = entry.path().file_name().is_some_and(|f| f == "FFS");
        if !is_ffs_file {
            continue;
        }
        if !selector.matches_file(&entry.path()) {
            continue;
        }

        let file = reader.read(entry.path())?;
        for (name, task) in file.targets() {
            let task_path = task_path(entry.path(), name);

            if !selector.matches(&task_path, &task.tags) {
                continue;
            }

            let output = builder.execute(
                &task_path,
                task,
                entry.path().parent().expect("entry is file"),
            )?;

            if !output.status.success() {
                std::io::stdout().lock().write_all(&output.stdout)?;
                std::io::stderr().lock().write_all(&output.stderr)?;
                eyre::bail!("Task failed: {task_path}");
            }
            count += 1;
        }
    }

    eyre::ensure!(count > 0, "No targets found matching {selector}");
    reporter.finish_top_level();

    Ok(())
}

struct Builder {
    reader: Arc<Reader>,
    executor: Arc<Executor>,

    root: PathBuf,
    target_outs: DashMap<String, PathBuf>,
}

impl Builder {
    fn new(reader: Arc<Reader>, executor: Arc<Executor>, root: impl AsRef<Path>) -> Self {
        Self {
            reader,
            executor,

            root: root.as_ref().to_path_buf(),
            target_outs: Default::default(),
        }
    }

    #[context_attr::eyre(format!("Parsing command for {task_path}"))]
    fn parse_command(&mut self, task_path: &str, c: &str) -> eyre::Result<String> {
        loop {
            // TODO(shelbyd): One pass parse.
            match command::parse_command(c, &self.target_outs) {
                Ok(c) => return eyre::Ok(c),
                Err(ParseError::UnknownTarget(t)) => {
                    let task_path = t.split_once(":").map(|(t, _file)| t).unwrap_or(&t);
                    self.build(&task_path.parse()?)?;
                }
            }
        }
    }

    #[context_attr::eyre(format!("Building {target}"))]
    fn build(&mut self, target: &TargetPath) -> eyre::Result<()> {
        let definition = self.root.join(target.definition());
        let targets = self.reader.read(&definition)?;

        let name = target.name();

        let task = targets
            .targets
            .get(name)
            .ok_or_eyre(format!("Unknown task: {target}"))?;

        let dir = definition.parent().unwrap();
        let relative_dir = dir.strip_prefix(&self.root).unwrap();

        let task_path = task_path(&relative_dir, name);

        let output = self.execute(&task_path, task, &dir)?;

        if !output.status.success() {
            eyre::bail!("Command exited with code: {:?}", output.status.code())
        }

        for (name, path) in &task.outs {
            let file = dir.join(path);
            eyre::ensure!(
                file.exists(),
                "Missing output file: {name} @ {}",
                file.display()
            );

            if name == "default" {
                self.target_outs.insert(task_path.to_string(), file);
            } else {
                self.target_outs.insert(format!("{task_path}:{name}"), file);
            }
        }

        Ok(())
    }

    fn execute(&mut self, path: &str, task: &Target, dir: &Path) -> eyre::Result<Output> {
        for prereq in &task.prereqs {
            self.build(&prereq.parse()?)?;
        }
        let command = self.parse_command(path, &task.cmd)?;

        let execution = Execution {
            path,
            command: &command,
            dir,
            runs_on: task.as_build().and_then(|b| b.runs_on.as_ref()),
        };
        Ok(self.executor.execute(execution)?)
    }
}
