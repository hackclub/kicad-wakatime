// use waka_kicad::FindProcess;
// use std::fs;
// use std::process;
use env_logger::Env;
// use log::debug;
// use log::error;
use log::info;
use waka_kicad::WakaKicad;
// use sysinfo::System;

fn main() -> Result<(), anyhow::Error> {
  env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
  info!("Initializing waka-kicad...");
  let mut plugin = WakaKicad::default();
  plugin.check_cli_installed()?;
  plugin.await_connect_to_kicad()?;
  plugin.await_get_open_board()?;
  // plugin.get_many_types()?;
  Ok(())
}
