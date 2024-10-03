use std::{io::Write, sync::Arc, time::Duration};

use crate::target::TargetPath;

#[derive(Debug, Clone, clap::Args)]
pub struct Options {
    #[arg(long, short)]
    quiet: bool,
}

pub fn build_reporter(options: &Options) -> Arc<dyn Reporter> {
    if options.quiet {
        return Arc::new(Quiet);
    }

    Arc::new(Stderr(std::io::stderr()))
}

#[allow(unused)]
pub trait Reporter {
    fn begin_execute(&self, task: &TargetPath) {}
    fn finish_execute(&self, task: &TargetPath, took: Duration) {}
    fn finish_top_level(&self) {}
}

struct Quiet;

impl Reporter for Quiet {}

struct Stderr(std::io::Stderr);

impl Reporter for Stderr {
    fn begin_execute(&self, task: &TargetPath) {
        let _ = writeln!(&self.0, "Running {task}");
    }

    fn finish_execute(&self, task: &TargetPath, took: Duration) {
        let _ = writeln!(
            &self.0,
            "Finish  {task} in {}.{}s",
            took.as_secs(),
            took.subsec_millis()
        );
    }
}
