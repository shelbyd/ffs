use std::{
    collections::{BTreeMap, HashMap, HashSet},
    ops::Deref,
    path::PathBuf,
};

mod output;
mod relative;
mod selector;
mod target;

pub use output::*;
pub use selector::*;
pub use target::*;

use crate::{command::Command, os::Os};

#[derive(Debug, Default)]
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
    pub runs_on: Option<Os>,

    pub common: Common,
}

#[derive(Debug)]
pub struct Common {
    pub cmd: Command,
    pub prereqs: HashSet<TargetPath>,
    pub tags: HashSet<String>,
    pub outs: HashMap<String, PathBuf>,
}

#[derive(Debug)]
pub enum Target {
    Task(Task),
    Build(Build),
}

impl Target {
    pub(crate) fn as_build(&self) -> Option<&Build> {
        match self {
            Target::Build(b) => Some(b),
            Target::Task(_) => None,
        }
    }
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
