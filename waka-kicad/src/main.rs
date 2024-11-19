use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use waka_kicad::traits::FindProcess;
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
  // main loop
  loop {
    plugin.set_current_time(plugin.current_time());
    // TODO
    // let board = plugin.await_get_open_board()?.unwrap();
    // let identifier = board.doc.identifier;
    plugin.set_many_items()?;
    // TODO: a new file is being focused on
    // TODO: the currently focused file has been saved
    // TODO: this block is not correct
    if plugin.enough_time_passed() {
      info!("A heartbeat should be sent (enough time passed)");
      plugin.send_heartbeat(false);
    }
    sleep(Duration::from_secs(5));
  }
  // TODO: this is unreachable
  Ok(())
}