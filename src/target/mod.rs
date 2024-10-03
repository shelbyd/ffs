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
    pub targets: BTreeMap<String, TargetDef>,
}

impl TargetSet {
    pub fn targets(&self) -> impl Iterator<Item = (&String, &TargetDef)> {
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
pub enum TargetDef {
    Task(Task),
    Build(Build),
}

impl TargetDef {
    pub(crate) fn as_build(&self) -> Option<&Build> {
        match self {
            TargetDef::Build(b) => Some(b),
            TargetDef::Task(_) => None,
        }
    }
}

impl Deref for TargetDef {
    type Target = Common;

    fn deref(&self) -> &Self::Target {
        match self {
            TargetDef::Task(t) => &t.common,
            TargetDef::Build(b) => &b.common,
        }
    }
}
