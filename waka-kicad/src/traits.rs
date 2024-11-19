use sysinfo::{Pid, Process, System};

pub trait FindProcess {
  fn find_process(&self, name: &str) -> Option<(&Pid, &Process)>;
}

impl FindProcess for System {
  fn find_process(&self, name: &str) -> Option<(&Pid, &Process)> {
    self.processes()
      .iter()
      .filter(|(_pid, process)| process.exe().is_some_and(|e| e.ends_with(name)))
      .collect::<Vec<_>>()
      .first()
      .cloned()
  }
}