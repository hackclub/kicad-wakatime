use std::rc::Rc;
use std::sync::{Mutex, RwLock};
use std::{collections::HashMap, sync::Arc};
use std::fs;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;
use ini::Ini;
use kicad::{KiCad, KiCadConnectionConfig, board::{Board, BoardItem}};
use kicad::protos::enums::KiCadObjectType;
use log::debug;
use log::info;
use log::error;
// use mouse_position::mouse_position::Mouse;
use sysinfo::{Pid, Process, System};
use thiserror::Error;

#[derive(Default)]
// pub struct WakaKicad<'a> {
pub struct WakaKicad {
  pub kicad: Option<KiCad>,
  // TODO: get somebody way smarter than me to help me uncomment this field
  // pub board: Option<Board<'a>>,
  pub items: HashMap<KiCadObjectType, Vec<BoardItem>>,
  // pub mouse_position: Mouse,
  pub active: bool,
}

// TODO: heartbeat - a new file is being focused on
// TODO: heartbeat - the currently focused file has been saved

// impl<'a> WakaKicad<'a> {
impl<'a> WakaKicad {
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
  pub fn get_api_key(&mut self) -> Result<(), anyhow::Error> {
    let cfg_path = self.cfg_path();
    // TODO: remove expects
    // TODO: prompt for and store API key if not found
    let cfg = Ini::load_from_file(cfg_path).expect("Could not get ~/.wakatime.cfg!");
    let cfg_settings = cfg.section(Some("settings")).expect("Could not get settings from ~/.wakatime.cfg!");
    let api_key = cfg_settings.get("api_key").expect("Could not get API key!");
    debug!("api_key = {api_key}");
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
    info!("Connected to KiCAD! (v{})", self.kicad.as_ref().unwrap().get_version().unwrap());
    // debug!("{:?}", k);
    Ok(())
  }
  pub fn await_get_open_board<'b>(&'b mut self) -> Result<Option<Board<'a>>, anyhow::Error> where 'b: 'a {
  // pub fn await_get_open_board(&mut self) -> Result<(), anyhow::Error> {
    let mut times = 0;
    let k = self.kicad.as_ref().unwrap();
    let mut board: Option<Board>;
    loop {
      // if times == 6 {
      //   error!("Could not find open board! (30s)");
      //   error!("Ensure that a board is open in the Schematic Editor or PCB Editor");
      //   return Err(PluginError::NoOpenBoard.into())
      // }
      info!("Waiting for open board... ({times})");
      board = k.get_open_board().ok();
      if board.is_some() {
        break;
      }
      sleep(Duration::from_secs(5));
      times += 1;
    }
    info!("Found open board!");
    debug!("{:?}", board);
    Ok(board)
  }
  pub fn set_active(&mut self, active: bool) {
    if active != self.active {
      debug!("self.active = {active}");
    }
    self.active = active;
  }
  pub fn set_many_items(&mut self) -> Result<(), anyhow::Error> {
    let mut items_new: HashMap<KiCadObjectType, Vec<BoardItem>> = HashMap::new();
    info!("Setting board items...");
    // TODO: safety
    // let board = self.board.as_ref().unwrap();
    let board = self.await_get_open_board()?.unwrap();
    let tracks = board.get_items(&[KiCadObjectType::KOT_PCB_TRACE])?;
    // TODO: is this the right variant?
    let arc_tracks = board.get_items(&[KiCadObjectType::KOT_PCB_ARC])?;
    let vias = board.get_items(&[KiCadObjectType::KOT_PCB_VIA])?;
    let footprint_instances = board.get_items(&[KiCadObjectType::KOT_PCB_FOOTPRINT])?;
    let pads = board.get_items(&[KiCadObjectType::KOT_PCB_PAD])?;
    items_new.insert(KiCadObjectType::KOT_PCB_TRACE, tracks);
    items_new.insert(KiCadObjectType::KOT_PCB_ARC, arc_tracks);
    items_new.insert(KiCadObjectType::KOT_PCB_VIA, vias);
    items_new.insert(KiCadObjectType::KOT_PCB_FOOTPRINT, footprint_instances);
    items_new.insert(KiCadObjectType::KOT_PCB_PAD, pads);
    // check
    // if self.items.iter().count() > 0 && self.items != items_new {
    if self.items != items_new {
      info!("Old items differ from new items!");
      for (kot, vec) in items_new.iter() {
        debug!("{:?} = [{}]", kot, vec.len());
      }
      self.set_active(true);
    }
    // set
    self.items = items_new;
    info!("Set board items!");
    Ok(())
  }
  pub fn cfg_path(&self) -> PathBuf {
    let home_dir = home::home_dir().expect("Unable to get your home directory!");
    home_dir.join(".wakatime.cfg")
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