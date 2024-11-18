// use waka_kicad::FindProcess;
use std::fs;
use env_logger::Env;
// only works on KiCAD nightly
use kicad::{DocumentType, KiCad, KiCadConnectionConfig};
// use log::debug;
// use log::error;
use log::info;
use waka_kicad::WakaKicad;
// use sysinfo::System;

fn main() -> Result<(), anyhow::Error> {
  env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
  let plugin = WakaKicad::default();
  // check that wakatime-cli is installed
  let cli_path = plugin.cli_path(waka_kicad::env_consts());
  info!("WakaTime CLI path: {:?}", cli_path);
  if fs::exists(cli_path)? {
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
