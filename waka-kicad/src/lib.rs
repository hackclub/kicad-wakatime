use std::path::PathBuf;

use kicad::board::Board;
// use kicad::{DocumentType, KiCad};
use log::info;
// use sysinfo::{Pid, Process, System};

#[derive(Default)]
pub struct WakaKicad<'a> {
  pub board: Option<Board<'a>>,
}

impl<'a> WakaKicad<'a> {
  pub fn set_board(&mut self, board: Option<Board<'a>>) {
    self.board = board;
  }
  pub fn cli_path(&self, consts: (&'static str, &'static str)) -> PathBuf {
    let (os, arch) = consts;
    let home_dir = home::home_dir().expect("Unable to get your home directory!");
    let cli_name = match os {
      "windows" => format!("wakatime-cli-windows-{arch}.exe"),
      _o => format!("wakatime-cli-{os}-{arch}"),
    };
    home_dir.join(".wakatime").join(cli_name)
  }
}

/// Return the current OS and ARCH.
/// Values are changed to match those used in wakatime-cli release names.
pub fn env_consts() -> (&'static str, &'static str) {
  // os
  let os = match std::env::consts::OS {
    "macos" => "darwin",
    a => a,
  };
  // arch
  let arch = match std::env::consts::ARCH {
    "x86" => "386",
    "x86_64" => "amd64",
    "aarch64" => "arm64", // e.g. Apple Silicon
    a => a,
  };
  (os, arch)
}

// pub trait FindProcess {
//   fn find_process(&self, name: &str) -> Option<(&Pid, &Process)>;
// }

// impl FindProcess for System {
//   fn find_process(&self, name: &str) -> Option<(&Pid, &Process)> {
//     self.processes()
//       .iter()
//       .filter(|(_pid, process)| process.exe().is_some_and(|e| e.ends_with(name)))
//       .collect::<Vec<_>>()
//       .first()
//       .cloned()
//   }
// }