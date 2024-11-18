use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

use kicad::{KiCad, KiCadConnectionConfig, board::Board};
use kicad::protos::enums::KiCadObjectType;
use log::info;
// use sysinfo::{Pid, Process, System};

#[derive(Default)]
pub struct WakaKicad<'a> {
  pub kicad: Option<KiCad>,
  pub board: Option<Board<'a>>,
}

impl<'a> WakaKicad<'a> {
  pub fn await_connect_to_kicad(&mut self) {
    let mut times = 0;
    let mut k: Option<KiCad>;
    loop {
      info!("Connecting to KiCAD... ({times})");
      k = KiCad::new(KiCadConnectionConfig {
        client_name: String::from("waka-kicad"),
        ..Default::default()
      }).ok();
      if k.is_some() {
        break;
      }
      sleep(Duration::from_secs(5));
      times += 1;
    }
    self.kicad = k;
    let Some(ref k) = self.kicad else { unreachable!(); };
    info!("Connected to KiCAD {}", k.get_version().unwrap());
  }
  pub fn await_get_open_board(&'a mut self) {
    let mut times = 0;
    let mut b: Option<Board>;
    let Some(ref k) = self.kicad else { unreachable!(); };
    loop {
      info!("Waiting for open board... ({times})");
      b = k.get_open_board().ok();
      if b.is_some() {
        break;
      }
      sleep(Duration::from_secs(5));
      times += 1;
    }
    self.board = b;
    let Some(ref b) = self.board else { unreachable!(); };
    info!("Found open board: {:?}", b);
  }
  pub fn get_many_types(&mut self) -> Result<(), anyhow::Error> {
    // TODO: safety
    let board = self.board.as_mut().unwrap();
    let tracks = board.get_items(&[KiCadObjectType::KOT_PCB_TRACE])?;
    info!("Found {} tracks", tracks.len());
    // TODO: is this the right variant?
    let arc_tracks = board.get_items(&[KiCadObjectType::KOT_PCB_ARC])?;
    info!("Found {} arc tracks", arc_tracks.len());
    let vias = board.get_items(&[KiCadObjectType::KOT_PCB_VIA])?;
    info!("Found {} vias", vias.len());
    let footprint_instances = board.get_items(&[KiCadObjectType::KOT_PCB_FOOTPRINT])?;
    info!("Found {} footprint instances", footprint_instances.len());
    let pads = board.get_items(&[KiCadObjectType::KOT_PCB_PAD])?;
    info!("Found {} pads", pads.len());
    Ok(())
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