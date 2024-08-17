use std::{
    collections::{BTreeMap, HashMap, HashSet},
    ops::Deref,
    path::PathBuf,
};

mod strings;
pub use strings::*;

#[derive(Debug, Default, starlark::any::ProvidesStaticType)]
pub struct TargetSet {
    pub targets: BTreeMap<String, Target>,
}

impl TargetSet {
    pub fn targets(&self) -> impl Iterator<Item = (&String, &Target)> {
        self.targets.iter()
    }
}

#[derive(Debug)]
pub struct Task {
    pub common: Common,
}

#[derive(Debug)]
pub struct Build {
    #[allow(unused)]
    pub srcs: HashSet<String>,
    #[allow(unused)]
    pub runs_on: Option<String>,

    pub common: Common,
}

#[derive(Debug)]
pub struct Common {
    pub cmd: String,
    pub prereqs: HashSet<String>,
    pub tags: HashSet<String>,
    pub outs: HashMap<String, PathBuf>,
}

#[derive(Debug)]
pub enum Target {
    Task(Task),
    Build(Build),
}

impl Deref for Target {
    type Target = Common;

    fn deref(&self) -> &Self::Target {
        match self {
            Target::Task(t) => &t.common,
            Target::Build(b) => &b.common,
        }
    }
}
