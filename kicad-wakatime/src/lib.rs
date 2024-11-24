use core::str;
use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Write};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use active_win_pos_rs::{get_active_window, ActiveWindow};
use fltk::prelude::*;
use ini::Ini;
use kicad::{KiCad, KiCadConnectionConfig, board::{Board, BoardItem}};
use kicad::protos::base_types::{DocumentSpecifier, document_specifier::Identifier};
use kicad::protos::enums::KiCadObjectType;
use log::debug;
use log::info;
use log::error;
use log::warn;
use notify::{Watcher, RecommendedWatcher, RecursiveMode};

pub mod ui;
pub mod traits;

use ui::{Message, Ui};

const PLUGIN_VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub struct Plugin {
  pub version: &'static str,
  pub disable_heartbeats: bool,
  pub config: Ini,
  pub ui: Ui,
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
  pub time: Duration,
  // the last time a heartbeat was sent
  pub last_sent_time: Duration,
  // the last file that was sent
  pub last_sent_file: String,
}

impl<'a> Plugin {
  pub fn new(
    disable_heartbeats: bool,
  ) -> Self {
    Plugin {
      version: PLUGIN_VERSION,
      disable_heartbeats,
      config: Ini::default(),
      ui: Ui::new(),
      tx: None,
      rx: None,
      kicad: None,
      filename: String::default(),
      full_path: PathBuf::default(),
      full_paths: HashMap::default(),
      warned_filenames: vec![],
      file_watcher: None,
      items: HashMap::default(),
      time: Duration::default(),
      last_sent_time: Duration::default(),
      last_sent_file: String::default(),
    }
  }
  pub fn main_loop(&mut self) -> Result<(), anyhow::Error> {
    self.set_current_time(self.current_time());
    let Ok(w) = self.get_active_window() else { return Ok(()); };
    let Some(ref k) = self.kicad else { return Ok(()); };
    if w.title.contains("Schematic Editor") {
      let schematic = k.get_open_schematic()?;
      // the KiCAD IPC API does not work properly with schematics as of November 2024
      // (cf. kicad-rs/issues/3), so for the schematic editor, heartbeats for file
      // modification without save cannot be sent
      let schematic_ds = schematic.doc;
      debug!("schematic_ds = {:?}", schematic_ds.clone());
      self.set_current_file_from_document_specifier(schematic_ds.clone())?;
    }
    else if w.title.contains("PCB Editor") {
      // for the PCB editor, we can instead use the Rust bindings proper
      let board = k.get_open_board()?;
      let board_ds = board.doc;
      debug!("board_ds = {:?}", board_ds.clone());
      self.set_current_file_from_document_specifier(board_ds.clone())?;
      self.set_many_items()?;
    }
    Ok(())
  }
  pub fn get_active_window(&mut self) -> Result<ActiveWindow, ()> {
    let active_window = get_active_window();
    if active_window.clone().is_ok_and(|w| w.title == "") {
      self.dual_error(String::from("Could not get title of active window!"));
      self.dual_error(String::from("If you are on macOS, please give kicad-wakatime Screen Recording permission"));
      self.dual_error(String::from("(System Settings -> Privacy and Security -> Screen Recording)"));
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
      info!("File does not exist!");
      self.get_latest_release();
    }
    Ok(())
  }
  pub fn get_latest_release(&mut self) {
    info!("Downloading latest WakaTime CLI...");
    let client = reqwest::blocking::Client::new();
    // need to insert some kind of user agent to avoid getting 403 forbidden
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("user-agent", "kicad-wakatime/1.0".parse().unwrap());
    // create .wakatime folder if it does not exist
    // we will be downloading the .zip into there
    if let Ok(false) = fs::exists(self.wakatime_folder_path()) {
      fs::create_dir(self.wakatime_folder_path());
    }
    // get download URL
    let res = client.get("https://api.github.com/repos/wakatime/wakatime-cli/releases/latest")
      .headers(headers.clone())
      .send()
      .expect("Could not make request!");
    let json = res.json::<serde_json::Value>().unwrap();
    let mut asset = json["assets"]
      .as_array()
      .unwrap()
      .into_iter()
      .find(|v| v["name"].as_str().unwrap().to_owned() == self.cli_zip_name(env_consts()))
      .unwrap();
    let download_url = asset["browser_download_url"].as_str().unwrap().to_owned();
    // download .zip file
    let res = client.get(download_url)
      .headers(headers)
      .send()
      .expect("Could not make request!");
    let zip_bytes = res.bytes().expect("Could not parse bytes!");
    debug!("{:?}", self.cli_zip_path(env_consts()));
    let mut zip_file = fs::File::create(self.cli_zip_path(env_consts())).unwrap();
    zip_file.write_all(&zip_bytes);
    let zip_vec_u8: Vec<u8> = fs::read(self.cli_zip_path(env_consts())).unwrap();
    // extract .zip file
    zip_extract::extract(
      Cursor::new(zip_vec_u8),
      &self.wakatime_folder_path(),
      true
    );
    // remove zip file
    fs::remove_file(self.cli_zip_path(env_consts()));
  }
  pub fn load_config(&mut self) {
    let wakatime_cfg_path = self.wakatime_cfg_path();
    if !fs::exists(&wakatime_cfg_path).unwrap() {
      Ini::new().write_to_file(&wakatime_cfg_path);
    }
    self.config = Ini::load_from_file(&wakatime_cfg_path).unwrap();
  }
  pub fn store_config(&self) {
    Ini::write_to_file(&self.config, self.wakatime_cfg_path());
  }
  pub fn set_api_key(&mut self, api_key: String) {
    self.config.with_section(Some("settings"))
      .set("api_key", api_key);
  }
  pub fn get_api_key(&mut self) -> String {
    match self.config.with_section(Some("settings")).get("api_key") {
      Some(api_key) => api_key.to_string(),
      None => String::new(),
    }
  }
  pub fn set_api_url(&mut self, api_url: String) {
    self.config.with_section(Some("settings"))
      .set("api_url", api_url);
  }
  pub fn get_api_url(&mut self) -> String {
    match self.config.with_section(Some("settings")).get("api_url") {
      Some(api_url) => api_url.to_string(),
      None => String::new(),
    }
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
  pub fn await_get_open_board<'b>(&'b mut self) -> Result<Option<Board<'a>>, anyhow::Error> where 'b: 'a {
    let k = self.kicad.as_ref().unwrap();
    let mut board: Option<Board>;
    loop {
      board = k.get_open_board().ok();
      if board.is_some() {
        break;
      }
      sleep(Duration::from_secs(5));
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
  pub fn set_current_file_from_document_specifier(
    &mut self,
    specifier: DocumentSpecifier,
  ) -> Result<(), anyhow::Error> {
    debug!("Updating current file...");
    // filename
    // TODO: other variants
    let filename = self.get_filename_from_document_specifier(&specifier);
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
  pub fn try_ui_recv(&mut self) {
    match self.ui.receiver.recv() {
      Some(Message::OpenSettingsWindow) => {
        self.ui.settings_window_ui.settings_window.show();
      },
      Some(Message::CloseSettingsWindow) => {
        self.ui.settings_window_ui.settings_window.hide();
        self.store_config();
      },
      Some(Message::UpdateSettings) => {
        self.set_api_key(self.ui.settings_window_ui.api_key.value());
        self.set_api_url(self.ui.settings_window_ui.server_url.value().unwrap());
        self.store_config();
      }
      None => {},
    }
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
    let api_key = self.get_api_key();
    // TODO: api key validity check
    let quoted_api_key = format!("\"{api_key}\"");
    // TODO: metrics?
    // TODO: api_url?
    // TODO: is_unsaved_entity
    // create process
    let cli_path = self.cli_path(env_consts());
    let mut cli = std::process::Command::new(cli_path);
    cli.args(&["--entity", &quoted_full_path]);
    cli.args(&["--plugin", &quoted_user_agent]);
    cli.args(&["--key", &quoted_api_key]);
    if is_file_saved {
      cli.arg("--write");
    }
    self.dual_info(String::from("Executing WakaTime CLI..."));
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
  /// Return the path to the .wakatime.cfg file.
  pub fn wakatime_cfg_path(&self) -> PathBuf {
    let home_dir = home::home_dir().expect("Unable to get your home directory!");
    home_dir.join(".wakatime.cfg")
  }
  /// Return the path to the .wakatime folder.
  pub fn wakatime_folder_path(&self) -> PathBuf {
    let home_dir = home::home_dir().expect("Unable to get your home directory!");
    home_dir.join(".wakatime")
  }
  /// Return the file stem of the WakaTime CLI for the current OS and architecture.
  pub fn cli_name(&self, consts: (&'static str, &'static str)) -> String {
    let (os, arch) = consts;
    match os {
      "windows" => format!("wakatime-cli-windows-{arch}"),
      _o => format!("wakatime-cli-{os}-{arch}"),
    }
  }
  /// Return the file name of the WakaTime CLI .zip file for the current OS and architecture. 
  pub fn cli_zip_name(&self, consts: (&'static str, &'static str)) -> String {
    format!("{}.zip", self.cli_name(consts))
  }
  /// Return the file name of the WakaTime CLI for the current OS and architecture.
  pub fn cli_exe_name(&self, consts: (&'static str, &'static str)) -> String {
    let (os, _arch) = consts;
    let cli_name = self.cli_name(consts);
    match os {
      "windows" => format!("{cli_name}.exe"),
      _o => cli_name,
    }
  }
  /// Return the path to the WakaTime CLI for the current OS and architecture.
  pub fn cli_path(&self, consts: (&'static str, &'static str)) -> PathBuf {
    let wakatime_folder_path = self.wakatime_folder_path();
    let cli_exe_name = self.cli_exe_name(consts);
    wakatime_folder_path.join(cli_exe_name)
  }
  /// Return the path to the downloaded WakaTime CLI .zip file for the current OS and architecture.
  /// The file is downloaded into the .wakatime folder.
  pub fn cli_zip_path(&self, consts: (&'static str, &'static str)) -> PathBuf {
    self.wakatime_folder_path().join(self.cli_zip_name(consts))
  }
  pub fn dual_info(&mut self, s: String) {
    info!("{}", s);
    self.ui.main_window_ui.log_window.append(format!("\x1b[32m[info]\x1b[0m  {s}\n").as_str());
  }
  pub fn dual_warn(&mut self, s: String) {
    warn!("{}", s);
    self.ui.main_window_ui.log_window.append(format!("\x1b[33m[warn]\x1b[0m  {s}\n").as_str());
  }
  pub fn dual_error(&mut self, s: String) {
    error!("{}", s);
    self.ui.main_window_ui.log_window.append(format!("\x1b[31m[error]\x1b[0m {s}\n").as_str());
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