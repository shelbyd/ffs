use std::{
    cell::RefCell,
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
use starlark::{
    environment::{GlobalsBuilder, Module},
    eval::Evaluator,
    syntax::{AstModule, Dialect},
    values::{list::UnpackList, none::NoneType},
};
use target::{task_path, Selector};

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
        if !selector.matches_file(&entry.path()) {
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

#[derive(Debug, Default, starlark::any::ProvidesStaticType)]
struct TaskSet(BTreeMap<String, Task>);

impl TaskSet {
    fn tasks(&self) -> impl Iterator<Item = (&String, &Task)> {
        self.0.iter()
    }
}

#[derive(Debug)]
struct Task {
    cmd: String,

    prereqs: Vec<String>,

    tags: HashSet<String>,

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
                    let task_path = t.split_once(":").map(|(t, _file)| t).unwrap_or(&t);
                    self.build(task_path)
                        .context(format!("Building {task_path:?}"))?;
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

    fn execute(&mut self, task: &Task, dir: &Path) -> eyre::Result<Output> {
        for prereq in &task.prereqs {
            self.build(&prereq)?;
        }

        let command = self.parse_command(&task.cmd)?;
        Ok(std::process::Command::new("sh")
            .current_dir(dir)
            .arg("-c")
            .arg(command)
            .output()?)
    }
}

struct Reader {
    cache: DashMap<PathBuf, Arc<TaskSet>>,
}

impl Reader {
    fn new() -> Self {
        Self {
            cache: Default::default(),
        }
    }

    fn read(&self, path: impl AsRef<Path>) -> eyre::Result<Arc<TaskSet>> {
        let v = match self.cache.entry(path.as_ref().to_path_buf()) {
            dashmap::Entry::Occupied(o) => return Ok(Arc::clone(o.get())),
            dashmap::Entry::Vacant(v) => v,
        };

        let tasks: TaskSet = self.load(path.as_ref())?;
        let f = v.insert(Arc::new(tasks));
        Ok(Arc::clone(&f))
    }

    fn load(&self, path: impl AsRef<Path>) -> eyre::Result<TaskSet> {
        let path = path.as_ref();
        let contents = std::fs::read_to_string(path)?;

        let ast = AstModule::parse(&path.display().to_string(), contents, &Dialect::Standard)
            .map_err(|e| eyre::eyre!(e.into_anyhow()))?;

        let globals = GlobalsBuilder::standard().with(task_definer).build();
        let module = Module::new();

        let result = RefCell::new(TaskSet::default());
        {
            let mut eval = Evaluator::new(&module);
            eval.extra = Some(&result);

            eval.eval_module(ast, &globals)
                .map_err(|e| eyre::eyre!(e.into_anyhow()))?;
        }

        Ok(result.into_inner())
    }
}

#[starlark::starlark_module]
fn task_definer(builder: &mut GlobalsBuilder) {
    // TODO(shelbyd): Return path to task.
    fn task(
        name: String,
        cmd: String,

        #[starlark(require = named)] prereqs: Option<UnpackList<String>>,
        #[starlark(require = named)] tags: Option<UnpackList<String>>,
        #[starlark(require = named)] outs: Option<BTreeMap<String, String>>,

        eval: &mut Evaluator,
    ) -> starlark::Result<NoneType> {
        let mut set = eval
            .extra
            .unwrap()
            .downcast_ref::<RefCell<TaskSet>>()
            .unwrap()
            .borrow_mut();

        set.0.insert(
            name.to_string(),
            Task {
                cmd,
                prereqs: prereqs.into_iter().flatten().collect(),
                tags: tags.into_iter().flatten().collect(),
                outs: outs
                    .into_iter()
                    .flatten()
                    .map(|(k, v)| (k, PathBuf::from(v)))
                    .collect(),
            },
        );

        Ok(NoneType)
    }
}
