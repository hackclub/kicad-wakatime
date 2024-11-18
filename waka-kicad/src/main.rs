// use waka_kicad::FindProcess;
use std::fs;
use std::process;
use env_logger::Env;
// only works on KiCAD nightly
use kicad::{DocumentType, KiCad, KiCadConnectionConfig};
// use log::debug;
use log::error;
use log::info;
use waka_kicad::WakaKicad;
// use sysinfo::System;

fn main() -> Result<(), anyhow::Error> {
  env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
  let mut plugin = WakaKicad::default();
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
  plugin.await_connect_to_kicad();
  plugin.await_get_open_board();
  // plugin.get_many_types()?;
  Ok(())
}
