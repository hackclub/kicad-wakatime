use std::fs;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;
use kicad::{KiCad, KiCadConnectionConfig, board::Board};
use kicad::protos::enums::KiCadObjectType;
use log::info;
use log::error;
// use sysinfo::{Pid, Process, System};
use thiserror::Error;

#[derive(Default)]
pub struct WakaKicad<'a> {
  pub kicad: Option<KiCad>,
  pub board: Option<Board<'a>>,
}

impl<'a> WakaKicad<'a> {
  pub fn check_cli_installed(&self) -> Result<(), anyhow::Error> {
    let cli_path = self.cli_path(env_consts());
    info!("WakaTime CLI path: {:?}", cli_path);
    if fs::exists(cli_path)? {
      info!("File exists!");
      // TODO: update to latest version if needed
    } else {
      // TODO: download latest version
      error!("File does not exist!");
      error!("Ensure this file exists before proceeding");
      return Err(PluginError::CliNotFound.into())
    }
    Ok(())
  }
  pub fn await_connect_to_kicad(&mut self) -> Result<(), anyhow::Error> {
    let mut times = 0;
    let mut k: Option<KiCad>;
    loop {
      // if times == 6 {
      //   error!("Could not connect to KiCAD! (30s)");
      //   error!("Ensure KiCAD is open and the KiCAD API is enabled (Preferences -> Plugins -> Enable KiCAD API)");
      //   return Err(PluginError::CouldNotConnect.into())
      // }
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
    info!("Connected to KiCAD! (v{})", k.get_version().unwrap());
    Ok(())
  }
  pub fn await_get_open_board(&'a mut self) -> Result<(), anyhow::Error> {
    let mut times = 0;
    let mut b: Option<Board>;
    let Some(ref k) = self.kicad else { unreachable!(); };
    loop {
      // if times == 6 {
      //   error!("Could not find open board! (30s)");
      //   error!("Ensure that a board is open in the Schematic Editor or PCB Editor");
      //   return Err(PluginError::NoOpenBoard.into())
      // }
      info!("Waiting for open board... ({times})");
      b = k.get_open_board().ok();
      if b.is_some() {
        break;
      }
      sleep(Duration::from_secs(5));
      times += 1;
    }
    self.board = b;
    // let Some(ref b) = self.board else { unreachable!(); };
    // info!("Found open board: {:?}", b);
    info!("Found open board!");
    Ok(())
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

#[derive(Debug, Error)]
pub enum PluginError {
  #[error("Could not find WakaTime CLI!")]
  CliNotFound,
  // #[error("Could not connect to KiCAD!")]
  // CouldNotConnect,
  // #[error("Could not find open board!")]
  // NoOpenBoard,
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