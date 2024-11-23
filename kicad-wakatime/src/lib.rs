use core::str;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
// use std::rc::Rc;
// use std::sync::{Arc, Mutex, RwLock};
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use active_win_pos_rs::{get_active_window, ActiveWindow};
use ini::Ini;
use kicad::{KiCad, KiCadConnectionConfig, board::{Board, BoardItem}};
use kicad::protos::base_types::{DocumentSpecifier, document_specifier::Identifier};
use kicad::protos::enums::KiCadObjectType;
use log::debug;
use log::info;
use log::error;
use log::warn;
// use mouse_position::mouse_position::Mouse;
use notify::{Watcher, RecommendedWatcher, RecursiveMode};
use thiserror::Error;

pub mod ui;
pub mod traits;

use ui::Ui;

const PLUGIN_VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[derive(Default)]
pub struct Plugin {
  pub version: &'static str,
  pub disable_heartbeats: bool,
  pub sleepy: bool,
  pub ui: Option<Ui>,
  // pub active_window: ActiveWindow,
  pub tx: Option<Sender<notify::Result<notify::Event>>>,
  pub rx: Option<Receiver<notify::Result<notify::Event>>>,
  pub kicad: Option<KiCad>,
  // TODO: open an issue for help uncommenting this field
  // pub board: Option<Board<'a>>,
  // filename of currently focused file
  pub filename: String,
  // path of currently focused file
  pub full_path: PathBuf,
  pub full_paths: HashMap<String, PathBuf>,
  pub warned_filenames: Vec<String>,
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

impl<'a> Plugin {
  pub fn new(
    disable_heartbeats: bool,
    sleepy: bool,
  ) -> Self {
    Plugin {
      version: PLUGIN_VERSION,
      disable_heartbeats,
      sleepy,
      ..Default::default()
    }
  }
  pub fn get_active_window(&mut self) -> Result<ActiveWindow, ()> {
    let active_window = get_active_window();
    if active_window.clone().is_ok_and(|w| w.title == "") {
      self.dual_error(String::from("Could not get title of active window!"));
      self.dual_error(String::from("If you are on macOS, please give kicad-wakatime Screen Recording permission"));
      self.dual_error(String::from("(System Settings -> Privacy and Security -> Screen Recording)"));
      // process::exit(1);
    }
    active_window
  }
  pub fn check_cli_installed(&mut self) -> Result<(), anyhow::Error> {
    let cli_path = self.cli_path(env_consts());
    self.dual_info(format!("WakaTime CLI path: {:?}", cli_path));
    if fs::exists(cli_path)? {
      self.dual_info(String::from("File exists!"));
      // TODO: update to latest version if needed
    } else {
      // TODO: download latest version
      self.dual_error(String::from("File does not exist!"));
      self.dual_error(String::from("Ensure this file exists before proceeding"));
      return Err(PluginError::CliNotFound.into())
    }
    Ok(())
  }
  pub fn get_api_key(&mut self) -> Result<String, anyhow::Error> {
    let cfg_path = self.cfg_path();
    // TODO: remove expects
    // TODO: prompt for and store API key if not found
    let cfg = Ini::load_from_file(cfg_path).expect("Could not get ~/.wakatime.cfg!");
    let cfg_settings = cfg.section(Some("settings")).expect("Could not get settings from ~/.wakatime.cfg!");
    let api_key = cfg_settings.get("api_key").expect("Could not get API key!");
    // debug!("api_key = {api_key}");
    Ok(api_key.to_string())
  }
  pub fn connect_to_kicad(&mut self) -> Result<(), anyhow::Error> {
    let k = KiCad::new(KiCadConnectionConfig {
      client_name: String::from("kicad-wakatime"),
      ..Default::default()
    }).ok();
    if k.is_some() {
      self.kicad = k;
      self.dual_info(format!("Connected to KiCAD! (v{})", self.kicad.as_ref().unwrap().get_version().unwrap()));
      debug!("self.kicad = {:?}", self.kicad);
    } else {
      self.dual_error(String::from("Could not connect to KiCAD!"));
      self.dual_error(String::from("Please open KiCAD before opening kicad-wakatime!"));
      // process::exit(1);
    }
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
      // debug!("Waiting for open board... ({times})");
      board = k.get_open_board().ok();
      if board.is_some() {
        break;
      }
      sleep(Duration::from_secs(5));
      times += 1;
    }
    // debug!("Found open board!");
    // debug!("{:?}", board);
    Ok(board)
  }
  pub fn get_full_path(&self, filename: String) -> Option<&PathBuf> {
    self.full_paths.get(&filename)
  }
  pub fn get_filename_from_document_specifier(
    &self,
    specifier: &DocumentSpecifier,
  ) -> String {
    let Some(Identifier::BoardFilename(ref filename)) = specifier.identifier else { unreachable!(); };
    filename.to_string()
  }
  // pub fn set_current_file_from_identifier(&mut self, identifier: Identifier) -> Result<(), anyhow::Error> {
  pub fn set_current_file_from_document_specifier(
    &mut self,
    specifier: DocumentSpecifier,
  ) -> Result<(), anyhow::Error> {
    debug!("Updating current file...");
    // filename
    // TODO: other variants
    let filename = self.get_filename_from_document_specifier(&specifier);
    // info!("filename = {}", filename.clone());
    // info!("Current file updated to {}", filename.clone());
    // full path
    let project = specifier.project.0;
    let full_path = match project {
      // in the PCB editor, the specifier's project field is populated
      Some(project) => {
        let full_path = PathBuf::from(project.path).join(filename.clone());
        let file_stem = full_path.file_stem().unwrap().to_str().unwrap();
        self.full_paths.insert(
          format!("{file_stem}.kicad_sch"),
          full_path.parent().unwrap().join(format!("{file_stem}.kicad_sch"))
        );
        self.full_paths.insert(
          format!("{file_stem}.kicad_pcb"),
          full_path.parent().unwrap().join(format!("{file_stem}.kicad_pcb"))
        );
        full_path
      },
      // in the schematic editor, the specifier's project field is not populated.
      // ask the user to switch to the PCB editor for this schematic so that the
      // full path can be stored
      None => {
        if self.get_full_path(filename.clone()).is_none() {
          if !self.warned_filenames.contains(&filename) {
            self.dual_warn(format!("Schematic \"{}\" cannot be tracked yet!", filename.clone()));
            self.dual_warn(String::from("Please switch to the PCB editor first!"));
            self.dual_warn(String::from("You can then track time spent in both the schematic editor and PCB editor for this project"));
            self.warned_filenames.push(filename.clone());
          }
          return Ok(())
        }
        self.get_full_path(filename.clone()).unwrap().to_path_buf()
      }
    };
    // debug!("board_filename = {board_filename}");
    if self.filename != filename {
      self.dual_info(String::from("Focused file changed!"));
      // since the focused file changed, it might be time to send a heartbeat.
      // self.filename and self.path are not actually updated here unless they
      // were empty before, so self.maybe_send_heartbeat() can use the difference
      // as a condition in its check
      self.dual_info(format!("Filename: {}", filename.clone()));
      if self.filename != String::new() {
        self.maybe_send_heartbeat(filename.clone(), false)?;
      } else {
        self.filename = filename.clone();
        self.full_path = full_path.clone();
      }
      debug!("filename = {:?}", self.filename.clone());
      debug!("full_path = {:?}", self.full_path.clone());
      // also begin watching the focused file for changes
      self.watch_file(self.get_full_path(filename.clone()).unwrap().to_path_buf())?;
    } else {
      debug!("Focused file did not change!")
    }
    Ok(())
  }
  pub fn create_file_watcher(&mut self) -> Result<(), anyhow::Error> {
    self.file_watcher = Some(notify::recommended_watcher(self.tx.clone().unwrap())?);
    Ok(())
  }
  pub fn watch_file(&mut self, path: PathBuf) -> Result<(), anyhow::Error> {
    // let path = PathBuf::from("/Users/lux/file.txt");
    // info!("Watching {:?} for changes...", path);
    self.create_file_watcher()?;
    self.file_watcher.as_mut().unwrap().watch(path.as_path(), RecursiveMode::NonRecursive).unwrap();
    self.dual_info(format!("Watcher set up to watch {:?} for changes", path));
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
      self.dual_info(String::from("File saved!"));
      self.maybe_send_heartbeat(self.filename.clone(), true)?;
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
    debug!("Updating board items...");
    let mut items_new: HashMap<KiCadObjectType, Vec<BoardItem>> = HashMap::new();
    // TODO: safety
    let board = self.await_get_open_board()?.unwrap();
    // TODO: write cooler function
    let tracks = loop {
      if let Ok(tracks) = board.get_items(&[KiCadObjectType::KOT_PCB_TRACE]) { break tracks; }
    };
    let arc_tracks = loop {
      // TODO: is this the right variant?
      if let Ok(arc_tracks) = board.get_items(&[KiCadObjectType::KOT_PCB_ARC]) { break arc_tracks; }
    };
    let vias = loop {
      if let Ok(vias) = board.get_items(&[KiCadObjectType::KOT_PCB_VIA]) { break vias; }
    };
    let footprint_instances = loop {
      if let Ok(footprint_instances) = board.get_items(&[KiCadObjectType::KOT_PCB_FOOTPRINT]) { break footprint_instances; }
    };
    let pads = loop {
      if let Ok(pads) = board.get_items(&[KiCadObjectType::KOT_PCB_PAD]) { break pads; }
    };
    items_new.insert(KiCadObjectType::KOT_PCB_TRACE, tracks);
    items_new.insert(KiCadObjectType::KOT_PCB_ARC, arc_tracks);
    items_new.insert(KiCadObjectType::KOT_PCB_VIA, vias);
    items_new.insert(KiCadObjectType::KOT_PCB_FOOTPRINT, footprint_instances);
    items_new.insert(KiCadObjectType::KOT_PCB_PAD, pads);
    // if self.items.iter().count() > 0 && self.items != items_new {
    if self.items != items_new {
      debug!("Board items changed!");
      self.items = items_new;
      for (kot, vec) in self.items.iter() {
        debug!("{:?} = [{}]", kot, vec.len());
      }
      // since the items changed, it might be time to send a heartbeat
      self.maybe_send_heartbeat(self.filename.clone(), false)?;
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
  ) -> Result<(), anyhow::Error> {
    debug!("Determining whether to send heartbeat...");
    // on the first iteration of the main loop, multiple values used to determine
    // whether a heartbeat should be sent are updated from their defaults, so any
    // heartbeats that would be sent are false positives that should be ignored
    if !self.first_iteration_finished {
      debug!("Not sending heartbeat (first iteration)");
      return Ok(());
    }
    if self.last_sent_time == Duration::ZERO {
      debug!("No heartbeats have been sent since the plugin opened");
    } else {
      debug!("It has been {:?} since the last heartbeat", self.time_passed());
    }
    if is_file_saved ||
    self.enough_time_passed() ||
    self.filename != filename {
      self.filename = filename;
      self.send_heartbeat(is_file_saved)?;
    } else {
      debug!("Not sending heartbeat (no conditions met)");
    }
    Ok(())
  }
  pub fn send_heartbeat(&mut self, is_file_saved: bool) -> Result<(), anyhow::Error> {
    self.dual_info(String::from("Sending heartbeat..."));
    if self.disable_heartbeats {
      self.dual_warn(String::from("Heartbeats are disabled (using --disable-heartbeats)"));
      return Ok(())
    }
    let full_path = self.full_path.clone().into_os_string().into_string().unwrap();
    let quoted_full_path = format!("\"{full_path}\"");
    let plugin_version = self.version;
    let kicad_version = self.kicad.as_ref().unwrap().get_version().unwrap();
    let quoted_user_agent = format!("\"kicad/{kicad_version} kicad-wakatime/{plugin_version}\"");
    let api_key = self.get_api_key()?;
    let quoted_api_key = format!("\"{api_key}\"");
    // TODO: metrics?
    // TODO: api_url?
    // TODO: is_unsaved_entity
    // create process
    let cli_path = self.cli_path(env_consts());
    let mut cli = std::process::Command::new(cli_path);
    // cli.args(cli_args.split(' ').collect::<Vec<&str>>());
    cli.args(&["--entity", &quoted_full_path]);
    cli.args(&["--plugin", &quoted_user_agent]);
    cli.args(&["--key", &quoted_api_key]);
    if is_file_saved {
      cli.arg("--write");
    }
    self.dual_info(String::from("Executing WakaTime CLI..."));
    // cli.spawn().expect("Could not spawn WakaTime CLI!");
    let cli_output = cli.output()
      .expect("Could not execute WakaTime CLI!");
    let cli_status = cli_output.status;
    let cli_stdout = cli_output.stdout;
    let cli_stderr = cli_output.stderr;
    // TODO: handle failing statuses
    debug!("cli_status = {cli_status}");
    debug!("cli_stdout = {:?}", str::from_utf8(&cli_stdout).unwrap());
    debug!("cli_stderr = {:?}", str::from_utf8(&cli_stderr).unwrap());
    // heartbeat should have been sent at this point
    self.dual_info(String::from("Finished!"));
    self.last_sent_time = self.current_time();
    self.last_sent_file = full_path;
    debug!("last_sent_time = {:?}", self.last_sent_time);
    debug!("last_sent_file = {:?}", self.last_sent_file);
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
  pub fn dual_info(&mut self, s: String) {
    let Some(ref mut ui) = self.ui else { todo!(); };
    info!("{}", s);
    ui.main_window_ui.log_window.append(format!("\x1b[32m[info]\x1b[0m  {s}\n").as_str());
  }
  pub fn dual_warn(&mut self, s: String) {
    let Some(ref mut ui) = self.ui else { todo!(); };
    warn!("{}", s);
    ui.main_window_ui.log_window.append(format!("\x1b[33m[warn]\x1b[0m  {s}\n").as_str());
  }
  pub fn dual_error(&mut self, s: String) {
    let Some(ref mut ui) = self.ui else { todo!(); };
    error!("{}", s);
    ui.main_window_ui.log_window.append(format!("\x1b[31m[error]\x1b[0m {s}\n").as_str());
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