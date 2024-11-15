use std::{fs, path::Path};
// use waka_kicad::FindProcess;
// only works on KiCAD nightly
use kicad::{DocumentType, KiCad, KiCadConnectionConfig};
// use log::debug;
// use log::error;
use log::info;
// use sysinfo::System;

fn main() -> Result<(), anyhow::Error> {
  env_logger::init();
  // get info
  // matches wakatime-cli release names
  let os = match std::env::consts::OS {
    "macos" => "darwin", // same as wakatime-cli
    a => a,
  };
  info!("os = {os}");
  let arch = match std::env::consts::ARCH {
    "x86" => "386",
    "x86_64" => "amd64",
    "aarch64" => "arm64", // e.g. Apple Silicon
    a => a,
  };
  info!("arch = {arch}");
  // check that wakatime-cli is installed
  let home_dir = home::home_dir().expect("Unable to get your home directory!");
  info!("home_dir = {:?}", home_dir);
  let wk_release_name = match os {
    "windows" => format!("wakatime-cli-windows-{arch}.exe"),
    _o => format!("wakatime-cli-{os}-{arch}"),
  };
  let wk_path = home_dir.join(".wakatime").join(wk_release_name);
  info!("WakaTime CLI path: {:?}", wk_path);
  if fs::exists(wk_path)? {
    info!("File exists!");
    // TODO: update to latest version if needed
  } else {
    // TODO: download latest version
    info!("File does not exist");
  }
  // TODO: wait instead of expect
  // connect to KiCAD
  let k = KiCad::new(KiCadConnectionConfig {
    client_name: String::from("waka-kicad"),
    ..Default::default()
  }).expect("KiCAD not running!");
  info!("Connected to KiCAD {}", k.get_version().unwrap());
  // get what is open
  if let Ok(schematics) = k.get_open_documents(DocumentType::DOCTYPE_SCHEMATIC) {
    info!("Found {} open schematic(s)", schematics.len());
  }
  if let Ok(pcbs) = k.get_open_documents(DocumentType::DOCTYPE_PCB) {
    info!("Found {} open PCB(s)", pcbs.len());
  }
  if let Ok(board) = k.get_open_board() {
    info!("Found open board: {:?}", board);
  }
  Ok(())
}
