use core::str;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use active_win_pos_rs::{get_active_window, ActiveWindow};
use chrono::{DateTime, Local};
use ini::Ini;
use log::debug;
use log::info;
use log::error;
use log::warn;
use notify::{Watcher, RecommendedWatcher, RecursiveMode};
use zip::ZipArchive;

pub mod ui;

const PLUGIN_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Plugin {
  pub version: &'static str,
  pub disable_heartbeats: bool,
  pub redownload: bool,
  pub wakatime_config: Ini,
  pub kicad_wakatime_config: Ini,
  pub settings_open: bool,
  pub tx: Option<Sender<notify::Result<notify::Event>>>,
  pub rx: Option<Receiver<notify::Result<notify::Event>>>,
  // filename of currently focused file
  pub filename: String,
  // path of currently focused file
  pub full_path: PathBuf,
  pub full_paths: HashMap<String, PathBuf>,
  pub file_watcher: Option<RecommendedWatcher>,
  pub projects_folder: String,
  pub api_key: String,
  pub api_url: String,
  pub time: Duration,
  // the last time a heartbeat was sent
  pub last_sent_time: Duration,
  pub last_sent_time_chrono: Option<DateTime<Local>>,
  // the last file that was sent
  pub last_sent_file: String,
  pub first_iteration_finished: bool,
}

impl Plugin {
  pub fn new(
    disable_heartbeats: bool,
    redownload: bool,
  ) -> Self {
    Plugin {
      version: PLUGIN_VERSION,
      disable_heartbeats,
      redownload,
      wakatime_config: Ini::default(),
      kicad_wakatime_config: Ini::default(),
      settings_open: false,
      tx: None,
      rx: None,
      filename: String::default(),
      full_path: PathBuf::default(),
      full_paths: HashMap::default(),
      file_watcher: None,
      projects_folder: String::default(),
      api_key: String::default(),
      api_url: String::default(),
      time: Duration::default(),
      last_sent_time: Duration::default(),
      last_sent_time_chrono: None,
      last_sent_file: String::default(),
      first_iteration_finished: false,
    }
  }
  pub fn main_loop(&mut self) -> Result<(), anyhow::Error> {
    if !self.first_iteration_finished {
      self.check_up_to_date()?;
      self.check_cli_installed(self.redownload)?;
      let projects_folder = self.get_projects_folder();
      self.watch_files(projects_folder.clone())?;
    }
    self.set_current_time(self.current_time());
    let Ok(w) = self.get_active_window() else {
      self.first_iteration_finished = true;
      return Ok(());
    };
    // note: written this way, split can be Some for some things that aren't KiCAD, e.g. VS Code.
    // we sanity check it later.
    let split = w.title.split_once(" — ");
    let Some((mut project, editor)) = split else {
      self.first_iteration_finished = true;
      return Ok(());
    };
    if project.starts_with("*") {
      project = &project[1..project.len()];
    }
    let filename = match editor {
      "Schematic Editor" => format!("{project}.kicad_sch"),
      "PCB Editor" => format!("{project}.kicad_pcb"),
      _ => String::new(),
    };
    let Some(_full_path) = self.get_full_path(filename.clone()) else {
      self.first_iteration_finished = true;
      return Ok(());
    };
    // let project_folder = full_path.parent().unwrap().to_path_buf();
    // let backups_folder = project_folder.join(format!("{project}-backups"));
    self.set_current_file(filename.clone())?;
    // self.look_at_backups_of_filename(filename, backups_folder);
    self.first_iteration_finished = true;
    Ok(())
  }
  pub fn get_active_window(&mut self) -> Result<ActiveWindow, ()> {
    let active_window = get_active_window();
    // as far as i can tell, active_win_pos_rs will focus on kicad-wakatime
    // when it starts, and that window should by all means have a title.
    // if the field is empty, kicad-wakatime is missing permissions
    if active_window.clone().is_ok_and(|w| w.app_name == "kicad-wakatime" && w.title.is_empty()) {
      error!("Could not get title of active window!");
      error!("If you are on macOS, please give kicad-wakatime Screen Recording permission");
      error!("(System Settings -> Privacy and Security -> Screen Recording)");
    }
    active_window
  }
  pub fn check_cli_installed(&mut self, redownload: bool) -> Result<(), anyhow::Error> {
    let cli_path = self.cli_path(env_consts());
    info!("WakaTime CLI path: {:?}", cli_path);
    if fs::exists(cli_path.clone())? {
      let mut cli = std::process::Command::new(cli_path);
      cli.arg("--version");
      let cli_output = cli.output()
        .expect("Could not execute WakaTime CLI!");
      let cli_stdout = cli_output.stdout;
      let cli_stdout = std::str::from_utf8(&cli_stdout)?;
      // TODO: update to latest version if needed
      info!("WakaTime CLI version: {cli_stdout}");
    } else {
      info!("File does not exist!");
      self.get_latest_release()?;
    }
    if redownload {
      info!("Redownloading WakaTime CLI (--redownload used)");
      self.get_latest_release()?;
    }
    Ok(())
  }
  pub fn check_up_to_date(&mut self) -> Result<(), anyhow::Error> {
    let client = reqwest::blocking::Client::new();
    // need to insert some kind of user agent to avoid getting 403 forbidden
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("user-agent", "kicad-wakatime/1.0".parse().unwrap());
    info!("Checking kicad-wakatime version");
    let res = client.get("https://api.github.com/repos/hackclub/kicad-wakatime/releases/latest")
      .headers(headers)
      .send()?;
      // .expect("Could not make request!");
    let json = res.json::<serde_json::Value>().unwrap();
    // sanity check
    if let serde_json::Value::String(message) = &json["message"] {
      if message == &String::from("Not Found") {
        warn!("No kicad-wakatime releases found!");
        return Ok(())
      }
    }
    let name = json["name"]
      .as_str()
      .unwrap()
      .to_string();
    if name != PLUGIN_VERSION {
      info!("kicad-wakatime update available!");
      info!("Visit https://github.com/hackclub/kicad-wakatime to download it");
    } else {
      info!("Up to date!");
    }
    Ok(())
  }
  pub fn get_latest_release(&mut self) -> Result<(), anyhow::Error> {
    let client = reqwest::blocking::Client::new();
    // need to insert some kind of user agent to avoid getting 403 forbidden
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("user-agent", "kicad-wakatime/1.0".parse().unwrap());
    // create .wakatime folder if it does not exist
    // we will be downloading the .zip into there
    if let Ok(false) = fs::exists(self.wakatime_folder_path()) {
      fs::create_dir(self.wakatime_folder_path())?;
    }
    // get download URL
    info!("Getting latest version from GitHub API");
    let res = client.get("https://api.github.com/repos/wakatime/wakatime-cli/releases/latest")
      .headers(headers.clone())
      .send()?;
      // .expect("Could not make request!");
    let json = res.json::<serde_json::Value>().unwrap();
    let asset = json["assets"]
      .as_array()
      .unwrap()
      .iter()
      .find(|v| v["name"].as_str().unwrap().to_owned() == self.cli_zip_name(env_consts()))
      .unwrap();
    let download_url = asset["browser_download_url"].as_str().unwrap().to_owned();
    // download .zip file
    info!("Downloading {download_url}...");
    let res = client.get(download_url)
      .headers(headers)
      .send()?;
      // .expect("Could not make request!");
    let zip_bytes = res.bytes()?;
    let mut zip_file = fs::File::create(self.cli_zip_path(env_consts())).unwrap();
    zip_file.write_all(&zip_bytes)?;
    let zip_vec_u8: Vec<u8> = fs::read(self.cli_zip_path(env_consts())).unwrap();
    // extract .zip file
    info!("Extracting .zip...");
    zip_extract::extract(
      Cursor::new(zip_vec_u8),
      &self.wakatime_folder_path(),
      true
    )?;
    // remove zip file
    fs::remove_file(self.cli_zip_path(env_consts()))?;
    // return
    info!("Finished!");
    Ok(())
  }
  pub fn load_config(&mut self) -> Result<(), anyhow::Error> {
    // wakatime config
    let wakatime_cfg_path = self.wakatime_cfg_path();
    if !fs::exists(&wakatime_cfg_path).unwrap() {
      Ini::new().write_to_file(&wakatime_cfg_path)?;
    }
    self.wakatime_config = Ini::load_from_file(&wakatime_cfg_path).unwrap();
    // kicad-wakatime config
    let kicad_wakatime_cfg_path = self.kicad_wakatime_cfg_path();
    if !fs::exists(&kicad_wakatime_cfg_path).unwrap() {
      Ini::new().write_to_file(&kicad_wakatime_cfg_path)?;
    }
    self.kicad_wakatime_config = Ini::load_from_file(&kicad_wakatime_cfg_path).unwrap();
    Ok(())
  }
  pub fn store_config(&self) -> Result<(), anyhow::Error> {
    Ini::write_to_file(&self.wakatime_config, self.wakatime_cfg_path())?;
    Ini::write_to_file(&self.kicad_wakatime_config, self.kicad_wakatime_cfg_path())?;
    Ok(())
  }
  pub fn set_api_key(&mut self, api_key: String) {
    self.wakatime_config.with_section(Some("settings"))
      .set("api_key", api_key);
  }
  pub fn get_api_key(&mut self) -> String {
    match self.wakatime_config.with_section(Some("settings")).get("api_key") {
      Some(api_key) => api_key.to_string(),
      None => String::new(),
    }
  }
  pub fn set_api_url(&mut self, api_url: String) {
    self.wakatime_config.with_section(Some("settings"))
      .set("api_url", api_url);
  }
  pub fn get_api_url(&mut self) -> String {
    match self.wakatime_config.with_section(Some("settings")).get("api_url") {
      Some(api_url) => api_url.to_string(),
      None => String::new(),
    }
  }
  pub fn set_projects_folder(&mut self, projects_folder: String) {
    self.kicad_wakatime_config.with_section(Some("settings"))
      .set("projects_folder", projects_folder);
  }
  pub fn get_projects_folder(&mut self) -> PathBuf {
    match self.kicad_wakatime_config.with_section(Some("settings")).get("projects_folder") {
      Some(projects_folder) => PathBuf::from(projects_folder),
      None => PathBuf::new(),
    }
  }
  pub fn language(&self) -> String {
    if self.filename.ends_with(".kicad_sch") {
      String::from("KiCAD Schematic")
    } else if self.filename.ends_with(".kicad_pcb") {
      String::from("KiCAD PCB")
    } else {
      unreachable!()
    }
  }
  pub fn get_full_path(&self, filename: String) -> Option<&PathBuf> {
    self.full_paths.get(&filename)
  }
  pub fn recursively_add_full_paths(&mut self, path: PathBuf) -> Result<(), anyhow::Error> {
    for path in fs::read_dir(path)? {
      let path = path.unwrap().path();
      if path.is_dir() { self.recursively_add_full_paths(path.clone())?; };
      if !path.is_file() { continue; };
      let file_name = path.file_name().unwrap().to_str().unwrap();
      // let file_stem = path.file_stem().unwrap().to_str().unwrap();
      let Some(file_extension) = path.extension() else { continue; };
      let file_extension = file_extension.to_str().unwrap();
      if file_extension == "kicad_sch" || file_extension == "kicad_pcb" {
        if self.full_paths.contains_key(file_name) {
          error!("Found multiple files named {file_name} in the projects folder!");
          error!("Please select a folder that only contains one file named {file_name}!");
          self.full_paths = HashMap::new();
          return Ok(())
        }
        self.full_paths.insert(
          file_name.to_string(),
          path
        );
      }
    }
    Ok(())
  }
  pub fn set_current_file(&mut self, filename: String) -> Result<(), anyhow::Error> {
    if self.filename != filename {
      info!("Focused file changed!");
      // since the focused file changed, it might be time to send a heartbeat.
      // self.filename and self.path are not actually updated here,
      // so self.maybe_send_heartbeat() can use the difference as a condition in its check
      info!("Filename: {}", filename.clone());
      self.maybe_send_heartbeat(filename.clone(), false)?;
      debug!("self.filename = {:?}", self.filename.clone());
      debug!("self.full_path = {:?}", self.full_path.clone());
    } else {
      // debug!("Focused file did not change!");
    }
    Ok(())
  }
  pub fn look_at_backups_of_filename(
    &mut self,
    filename: String,
    backups_folder: PathBuf
  ) -> Result<(), anyhow::Error> {
    // get all backups from the backups folder sorted by creation time
    info!("Looking at backups of {filename}...");
    std::thread::sleep(Duration::from_millis(500));
    let mut backups = fs::read_dir(backups_folder)?
      .flatten()
      .map(|x| x.path())
      .collect::<Vec<_>>();
    backups.sort_by_key(|x| x.metadata().unwrap().created().unwrap());
    let backups_count = backups.len();
    let mut v1: Vec<u8> = vec![];
    let mut v2: Vec<u8> = vec![];
    let p1 = &backups[backups_count - 1];
    let p2 = &backups[backups_count - 2];
    let f1 = File::open(p1)?;
    let f2 = File::open(p2)?;
    let mut newest_backup = ZipArchive::new(f1)?;
    let mut second_newest_backup = ZipArchive::new(f2)?;
    let mut newest_backup_of_filename = newest_backup.by_name(&filename)?;
    let mut second_newest_backup_of_filename = second_newest_backup.by_name(&filename)?;
    newest_backup_of_filename.read_to_end(&mut v1)?;
    second_newest_backup_of_filename.read_to_end(&mut v2)?;
    if v1.ne(&v2) {
      info!("Change detected!");
      self.maybe_send_heartbeat(filename, false)?;
    } else {
      info!("No change detected!");
    }
    Ok(())
  }
  pub fn create_file_watcher(&mut self) -> Result<(), anyhow::Error> {
    self.file_watcher = Some(notify::recommended_watcher(self.tx.clone().unwrap())?);
    Ok(())
  }
  pub fn watch_files(&mut self, path: PathBuf) -> Result<(), anyhow::Error> {
    if path == PathBuf::from("") {
      return Ok(())
    }
    info!("Watching {:?} for changes", path);
    self.create_file_watcher()?;
    self.file_watcher.as_mut().unwrap().watch(path.as_path(), RecursiveMode::Recursive)?;
    self.full_paths = HashMap::new();
    self.recursively_add_full_paths(path.clone())?;
    debug!("full_paths = {:?}", self.full_paths);
    Ok(())
  }
  pub fn try_recv(&mut self) -> Result<(), anyhow::Error> {
    let Some(ref rx) = self.rx else { unreachable!(); };
    let recv = rx.try_recv();
    if recv.is_ok() {
      if let Ok(Ok(notify::Event { kind, paths, attrs: _ })) = recv {
        let path = paths[0].clone();
        let is_backup = path.parent().unwrap().to_str().unwrap().ends_with("-backups");
        if path == self.full_path {
          info!("File saved!");
          self.maybe_send_heartbeat(self.filename.clone(), true)?;
        } else if is_backup && kind.is_create() {
          info!("New backup created!");
          self.look_at_backups_of_filename(self.filename.clone(), path.parent().unwrap().to_path_buf())?;
        }
      }
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
    self.time_passed() > Duration::from_secs(120)
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
    if self.time_passed() < Duration::from_millis(1000) {
      debug!("Not sending heartbeat (too fast!)");
      return Ok(())
    }
    if is_file_saved ||
    self.enough_time_passed() ||
    self.filename != filename {
      self.filename = filename.clone();
      self.full_path = self.get_full_path(filename.clone()).unwrap().to_path_buf();
      self.send_heartbeat(is_file_saved)?;
    } else {
      debug!("Not sending heartbeat (no conditions met)");
    }
    Ok(())
  }
  pub fn send_heartbeat(&mut self, is_file_saved: bool) -> Result<(), anyhow::Error> {
    info!("Sending heartbeat...");
    if self.disable_heartbeats {
      warn!("Heartbeats are disabled (using --disable-heartbeats)");
      warn!("Updating last_sent_time anyway");
      self.last_sent_time = self.current_time();
      self.last_sent_time_chrono = Some(Local::now());
      return Ok(())
    }
    let full_path = self.full_path.clone();
    let full_path_string = full_path.clone().into_os_string().into_string().unwrap();
    let quoted_full_path = format!("\"{full_path_string}\"");
    let plugin_version = self.version;
    // TODO: populate again
    let kicad_version = "unknown";
    let quoted_user_agent = format!("\"kicad/{kicad_version} kicad-wakatime/{plugin_version}\"");
    let api_key = self.get_api_key();
    let quoted_api_key = format!("\"{api_key}\"");
    let api_url = self.get_api_url();
    let quoted_api_url = format!("\"{api_url}\"");
    let language = self.language();
    let quoted_language = format!("\"{language}\"");
    let file_stem = full_path.clone().file_stem().unwrap().to_str().unwrap().to_string();
    // create process
    let cli_path = self.cli_path(env_consts());
    let mut cli = std::process::Command::new(cli_path);
    cli.args(["--entity", &quoted_full_path]);
    cli.args(["--plugin", &quoted_user_agent]);
    cli.args(["--key", &quoted_api_key]);
    cli.args(["--api-url", &quoted_api_url]);
    cli.args(["--language", &quoted_language]);
    cli.args(["--project", &file_stem]);
    if is_file_saved {
      cli.arg("--write");
    }
    info!("Executing WakaTime CLI...");
    let cli_output = cli.output()
      .expect("Could not execute WakaTime CLI!");
    let cli_status = cli_output.status;
    let cli_stdout = cli_output.stdout;
    let cli_stderr = cli_output.stderr;
    // TODO: handle failing statuses (102/112, 103, 104)
    debug!("cli_status = {cli_status}");
    debug!("cli_stdout = {:?}", str::from_utf8(&cli_stdout).unwrap());
    debug!("cli_stderr = {:?}", str::from_utf8(&cli_stderr).unwrap());
    // heartbeat should have been sent at this point
    info!("Finished!");
    self.last_sent_time = self.current_time();
    self.last_sent_time_chrono = Some(Local::now());
    self.last_sent_file = full_path_string;
    debug!("last_sent_time = {:?}", self.last_sent_time);
    debug!("last_sent_file = {:?}", self.last_sent_file);
    Ok(())
  }
  /// Return the path to the .wakatime.cfg file.
  pub fn wakatime_cfg_path(&self) -> PathBuf {
    let home_dir = home::home_dir().expect("Unable to get your home directory!");
    home_dir.join(".wakatime.cfg")
  }
  /// Return the path to the .kicad-wakatime.cfg file.
  pub fn kicad_wakatime_cfg_path(&self) -> PathBuf {
    let home_dir = home::home_dir().expect("Unable to get your home directory!");
    home_dir.join(".kicad-wakatime.cfg")
  }
  // /// Return the path to the .kicad-wakatime.log file.
  // pub fn kicad_wakatime_log_path(&self) -> PathBuf {
  //   let home_dir = home::home_dir().expect("Unable to get your home directory!");
  //   home_dir.join(".kicad-wakatime.log")
  // }
  /// Return the path to the .wakatime folder.
  pub fn wakatime_folder_path(&self) -> PathBuf {
    let home_dir = home::home_dir().expect("Unable to get your home directory!");
    home_dir.join(".wakatime")
  }
  /// Return the file stem of the WakaTime CLI executable for the current OS and architecture.
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
  /// Return the file name of the WakaTime CLI executable for the current OS and architecture.
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