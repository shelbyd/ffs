use std::{path::Path, process::Output, sync::Arc, time::Instant};

use crate::{os::Os, reporting::Reporter};

pub struct Executor {
    reporter: Arc<dyn Reporter>,
}

impl Executor {
    pub(crate) fn new(reporter: Arc<dyn Reporter>) -> Self {
        Self { reporter }
    }

    pub fn execute(&self, e: Execution) -> eyre::Result<Output> {
        if let Some(runs_on) = e.runs_on {
            let host = crate::os::host();
            eyre::ensure!(
                runs_on == &host,
                "Cannot run job requiring {runs_on:?} on {host:?}"
            );
        }

        self.reporter.begin_execute(e.path);
        let start = Instant::now();
        let output = std::process::Command::new("sh")
            .current_dir(e.dir)
            .arg("-e")
            .arg("-c")
            .arg(e.command)
            .output()?;
        self.reporter.finish_execute(e.path, start.elapsed());

        Ok(output)
    }
}

pub struct Execution<'l> {
    pub path: &'l str,
    pub command: &'l str,
    pub dir: &'l Path,
    pub runs_on: Option<&'l Os>,
}
