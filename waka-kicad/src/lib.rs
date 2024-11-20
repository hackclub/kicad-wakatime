use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
// use std::rc::Rc;
// use std::sync::{Arc, Mutex, RwLock};
use std::thread::{sleep, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use ini::Ini;
use kicad::KiCadError;
use kicad::{KiCad, KiCadConnectionConfig, board::{Board, BoardItem}};
use kicad::protos::base_types::{DocumentSpecifier, document_specifier::Identifier};
use kicad::protos::enums::KiCadObjectType;
use log::debug;
use log::info;
use log::error;
// use mouse_position::mouse_position::Mouse;
use notify::{Watcher, RecommendedWatcher, RecursiveMode};
use thiserror::Error;

pub mod traits;

#[derive(Default)]
// pub struct WakaKicad<'a> {
pub struct WakaKicad {
  pub tx: Option<Sender<notify::Result<notify::Event>>>,
  pub rx: Option<Receiver<notify::Result<notify::Event>>>,
  pub kicad: Option<KiCad>,
  // TODO: open a waka-kicad issue for help uncommenting this field
  // pub board: Option<Board<'a>>,
  // filename of currently focused file
  pub filename: String,
  // path of currently focused file
  pub path: PathBuf,
  pub file_watcher: Option<RecommendedWatcher>,
  pub items: HashMap<KiCadObjectType, Vec<BoardItem>>,
  // pub mouse_position: Mouse,
  pub time: Duration,
  // the last time a heartbeat was sent
  pub last_sent_time: Duration,
  // the last file that was sent
  pub last_sent_file: String,
  pub first_iteration_finished: bool,
}

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
  // TODO: plugin only works in PCB editor - get open documents, not open board
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
      debug!("Waiting for open board... ({times})");
      board = k.get_open_board().ok();
      if board.is_some() {
        break;
      }
      sleep(Duration::from_secs(5));
      times += 1;
    }
    debug!("Found open board!");
    debug!("{:?}", board);
    Ok(board)
  }
  // pub fn set_current_file_from_identifier(&mut self, identifier: Identifier) -> Result<(), anyhow::Error> {
  pub fn set_current_file_from_document_specifier(
    &mut self,
    specifier: DocumentSpecifier,
  ) -> Result<(), anyhow::Error> {
    // filename
    // TODO: other variants
    let Some(Identifier::BoardFilename(board_filename)) = specifier.identifier else { unreachable!(); };
    // path
    let path = PathBuf::from(specifier.project.unwrap().path).join(board_filename.clone());
    debug!("path = {:?}", path);
    // debug!("board_filename = {board_filename}");
    if self.filename != board_filename {
      info!("Identifier changed!");
      // since the focused file changed, it might be time to send a heartbeat.
      // self.filename and self.path are not actually updated here unless they
      // were empty before, so self.maybe_send_heartbeat() can use the difference
      // as a condition in its check
      if self.filename != String::new() {
        self.maybe_send_heartbeat(board_filename.clone(), false);
      } else {
        self.filename = board_filename.clone();
      }
      debug!("filename = {:?}", board_filename.clone());
      // also begin watching the focused file for changes
      self.watch_file(path)?;
    }
    Ok(())
  }
  pub fn create_file_watcher(&mut self) -> Result<(), anyhow::Error> {
    let (tx, rx) = std::sync::mpsc::channel::<Result<notify::Event, notify::Error>>();
    self.tx = Some(tx.clone());
    self.rx = Some(rx);
    self.file_watcher = Some(notify::recommended_watcher(tx)?);
    Ok(())
  }
  pub fn watch_file(&mut self, path: PathBuf) -> Result<(), anyhow::Error> {
    // let path = PathBuf::from("/Users/lux/file.txt");
    info!("Watching {:?} for changes...", path);
    self.file_watcher.as_mut().unwrap().watch(path.as_path(), RecursiveMode::NonRecursive).unwrap();
    info!("Watcher set up to watch {:?} for changes", path);
    Ok(())
  }
  pub fn try_recv(&mut self) -> Result<(), anyhow::Error> {
    let Some(ref rx) = self.rx else { unreachable!(); };
    let recv = rx.try_recv();
    if recv.is_ok() { // watched file was saved
      // skip duplicate
      // TODO: use debouncer instead
      let _ = rx.try_recv();
      // TODO: variant check?
      info!("File saved!");
      self.maybe_send_heartbeat(self.filename.clone(), true);
    }
    Ok(())
  }
  pub fn current_time(&self) -> Duration {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards!")
  }
  pub fn set_current_time(&mut self, t: Duration) {
    self.time = t;
  }
  /// Return the amount of time passed since the last heartbeat.
  pub fn time_passed(&self) -> Duration {
    self.current_time() - self.last_sent_time
  }
  /// Returns `true` if more than 2 minutes have passed since the last heartbeat.
  pub fn enough_time_passed(&self) -> bool {
    // self.current_time() > self.last_sent_time + Duration::from_secs(120)
    self.time_passed() > Duration::from_secs(120)
  }
  // TODO: change sig
  pub fn set_many_items(&mut self) -> Result<(), anyhow::Error> {
    let mut items_new: HashMap<KiCadObjectType, Vec<BoardItem>> = HashMap::new();
    // TODO: safety
    let board = self.await_get_open_board()?.unwrap();
    // TODO: still fails sometimes!
    while let Err(KiCadError::ApiError(_e)) = board.get_items(&[KiCadObjectType::KOT_PCB_TRACE]) {};
    let tracks = board.get_items(&[KiCadObjectType::KOT_PCB_TRACE])?;
    // TODO: is this the right variant?
    while let Err(KiCadError::ApiError(_e)) = board.get_items(&[KiCadObjectType::KOT_PCB_ARC]) {};
    let arc_tracks = board.get_items(&[KiCadObjectType::KOT_PCB_ARC])?;
    while let Err(KiCadError::ApiError(_e)) = board.get_items(&[KiCadObjectType::KOT_PCB_VIA]) {};
    let vias = board.get_items(&[KiCadObjectType::KOT_PCB_VIA])?;
    while let Err(KiCadError::ApiError(_e)) = board.get_items(&[KiCadObjectType::KOT_PCB_FOOTPRINT]) {};
    let footprint_instances = board.get_items(&[KiCadObjectType::KOT_PCB_FOOTPRINT])?;
    while let Err(KiCadError::ApiError(_e)) = board.get_items(&[KiCadObjectType::KOT_PCB_PAD]) {};
    let pads = board.get_items(&[KiCadObjectType::KOT_PCB_PAD])?;
    items_new.insert(KiCadObjectType::KOT_PCB_TRACE, tracks);
    items_new.insert(KiCadObjectType::KOT_PCB_ARC, arc_tracks);
    items_new.insert(KiCadObjectType::KOT_PCB_VIA, vias);
    items_new.insert(KiCadObjectType::KOT_PCB_FOOTPRINT, footprint_instances);
    items_new.insert(KiCadObjectType::KOT_PCB_PAD, pads);
    // if self.items.iter().count() > 0 && self.items != items_new {
    if self.items != items_new {
      debug!("Board items changed!");
      self.items = items_new;
      // since the items changed, it might be time to send a heartbeat
      self.maybe_send_heartbeat(self.filename.clone(), false);
      for (kot, vec) in self.items.iter() {
        debug!("{:?} = [{}]", kot, vec.len());
      }
    } else {
      debug!("Board items did not change!");
    }
    // set
    Ok(())
  }
  /// Send a heartbeat if conditions are met.
  /// This is an analog of vscode-wakatime's `private onEvent(isWrite)`.
  pub fn maybe_send_heartbeat(
    &mut self,
    filename: String,
    is_file_saved: bool
  ) {
    // on the first iteration of the main loop, multiple values used to determine
    // whether a heartbeat should be sent are updated from their defaults, so any
    // heartbeats that would be sent are false positives that should be ignored
    if self.first_iteration_finished == false {
      return;
    }
    if self.last_sent_time == Duration::ZERO {
      debug!("No heartbeats have been sent since the plugin opened");
    } else {
      debug!("It has been {:?} since the last heartbeat", self.time_passed());
    }
    // TODO: the currently focused file has been saved
    if is_file_saved ||
    self.enough_time_passed() ||
    self.filename != filename {
      self.filename = filename;
      self.send_heartbeat(is_file_saved);
    }
  }
  pub fn send_heartbeat(&mut self, is_file_saved: bool) {
    info!("Sending heartbeat...");
    info!("last_sent_time = {:?}", self.last_sent_time);
    info!("last_sent_file = {:?}", self.last_sent_file);
    self.last_sent_time = self.current_time();
    // TODO
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