use std::{
    cell::RefCell,
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context as _;
use dashmap::DashMap;
use starlark::{
    any::ProvidesStaticType,
    environment::{FrozenModule, GlobalsBuilder, Module},
    eval::Evaluator,
    syntax::{AstModule, Dialect},
    values::{list::UnpackList, none::NoneType},
};

use crate::target::{Build, Common, Target, TargetSet, Task};

pub struct Reader {
    root: PathBuf,
    cache: DashMap<PathBuf, Arc<TargetSet>>,
}

impl Reader {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self {
            root: root.into(),
            cache: Default::default(),
        }
    }

    pub fn read(&self, path: impl AsRef<Path>) -> eyre::Result<Arc<TargetSet>> {
        let v = match self.cache.entry(path.as_ref().to_path_buf()) {
            dashmap::Entry::Occupied(o) => return Ok(Arc::clone(o.get())),
            dashmap::Entry::Vacant(v) => v,
        };

        let tasks: TargetSet = self.load(path.as_ref())?;
        let f = v.insert(Arc::new(tasks));
        Ok(Arc::clone(&f))
    }

    fn load(&self, path: impl AsRef<Path>) -> eyre::Result<TargetSet> {
        let path = path.as_ref();
        let contents = std::fs::read_to_string(path)?;

        let (_, result) = self
            .exec_starlark(&path.display().to_string(), contents)
            .map_err(|e| eyre::eyre!(e))?;

        Ok(result)
    }

    fn exec_starlark<'s>(
        &'s self,
        path: &str,
        contents: String,
    ) -> anyhow::Result<(Module, TargetSet)> {
        let ast =
            AstModule::parse(path, contents, &Dialect::Standard).map_err(|e| e.into_anyhow())?;

        // TODO(shelbyd): Do all invocations of this have the task_definer?
        let globals = GlobalsBuilder::standard().with(task_definer).build();
        let module = Module::new();

        let context = Context {
            path,
            task_out: RefCell::new(TargetSet::default()),
        };
        {
            let mut eval = Evaluator::new(&module);
            eval.extra = Some(&context);
            eval.set_loader(self);

            eval.eval_module(ast, &globals)
                .map_err(|e| e.into_anyhow())?;
        }

        Ok((module, context.task_out.into_inner()))
    }
}

impl starlark::eval::FileLoader for Reader {
    fn load(&self, path: &str) -> anyhow::Result<FrozenModule> {
        let source = if let Some(path) = path.strip_prefix("//") {
            let path = self.root.join(path);
            std::fs::read_to_string(&path).context(format!("Reading: {}", path.display()))?
        } else {
            anyhow::bail!("Unknown load schema: {path:?}");
        };

        let (module, _) = self.exec_starlark(path, source)?;
        Ok(module.freeze()?)
    }
}

#[derive(ProvidesStaticType)]
struct Context<'c> {
    path: &'c str,
    task_out: RefCell<TargetSet>,
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
        let context = eval.extra.unwrap().downcast_ref::<Context>().unwrap();
        let mut set = context.task_out.borrow_mut();

        set.targets.insert(
            name.to_string(),
            Target::Task(Task {
                common: Common {
                    cmd,
                    prereqs: prereqs.into_iter().flatten().collect(),
                    tags: tags.into_iter().flatten().collect(),
                    outs: outs
                        .into_iter()
                        .flatten()
                        .map(|(k, v)| (k, PathBuf::from(v)))
                        .collect(),
                },
            }),
        );

        Ok(NoneType)
    }

    fn build(
        name: String,
        cmd: String,
        srcs: UnpackList<String>,
        outs: BTreeMap<String, String>,
        runs_on: Option<String>,

        #[starlark(require = named)] prereqs: Option<UnpackList<String>>,
        #[starlark(require = named)] tags: Option<UnpackList<String>>,

        eval: &mut Evaluator,
    ) -> anyhow::Result<NoneType> {
        let context = eval.extra.unwrap().downcast_ref::<Context>().unwrap();
        let mut set = context.task_out.borrow_mut();

        set.targets.insert(
            name.to_string(),
            Target::Build(Build {
                common: Common {
                    cmd,
                    prereqs: prereqs.into_iter().flatten().collect(),
                    tags: tags.into_iter().flatten().collect(),
                    outs: outs
                        .into_iter()
                        .map(|(k, v)| (k, PathBuf::from(v)))
                        .collect(),
                },
                srcs: srcs.into_iter().collect(),
                runs_on: runs_on
                    .map(|s| s.parse())
                    .transpose()
                    .map_err(|e: eyre::Report| anyhow::anyhow!(e))?,
            }),
        );

        Ok(NoneType)
    }

    fn local_file(source: String, file: String) -> anyhow::Result<String> {
        let source_dir = source.rsplit_once("/").unwrap().0;
        Ok(format!("{source_dir}/{file}"))
    }

    fn get_source(eval: &mut Evaluator) -> anyhow::Result<String> {
        let context = eval.extra.unwrap().downcast_ref::<Context>().unwrap();
        Ok(context.path.to_string())
    }
}
