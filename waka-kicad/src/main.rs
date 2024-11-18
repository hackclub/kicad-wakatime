use waka_kicad::FindProcess;
// use std::fs;
// use std::process;
use env_logger::Env;
use log::debug;
// use log::error;
use log::info;
use waka_kicad::WakaKicad;
use sysinfo::System;

fn main() -> Result<(), anyhow::Error> {
  env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
  debug!("(os, arch) = {:?}", waka_kicad::env_consts());
  let mut sys = System::new_all();
  sys.refresh_all();
  debug!("eeschema -> {:?}", sys.find_process("eeschema"));
  debug!("pcbnew -> {:?}", sys.find_process("pcbnew"));
  info!("Initializing waka-kicad...");
  let mut plugin = WakaKicad::default();
  plugin.check_cli_installed()?;
  plugin.get_api_key()?;
  plugin.await_connect_to_kicad()?;
  plugin.await_get_open_board()?;
  // plugin.get_many_types()?;
  Ok(())
}
