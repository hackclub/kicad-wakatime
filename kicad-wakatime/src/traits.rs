use log::debug;
use sysinfo::{Pid, Process, System};

// type ProcessSearchResult = Option<(&Pid, &Process)>;

pub trait FindProcess {
  fn find_process(&self, name: &str) -> Option<(&Pid, &Process)>;
}

impl FindProcess for System {
  fn find_process(&self, name: &str) -> Option<(&Pid, &Process)> {
    self.processes()
      .iter()
      .filter(|(_pid, process)| process.exe().is_some_and(
        |e| e.ends_with(name) || e.ends_with(format!("{name}.exe"))
      ))
      .collect::<Vec<_>>()
      .first()
      .cloned()
  }
}

pub trait DebugProcesses {
  fn debug_processes(&self);
}

impl DebugProcesses for System {
  fn debug_processes(&self) {
    debug!("eeschema -> {:?}", self.find_process("eeschema"));
    debug!("pcbnew -> {:?}", self.find_process("pcbnew"));
  }
}