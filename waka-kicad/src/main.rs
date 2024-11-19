use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use waka_kicad::{WakaKicad, traits::DebugProcesses};
// use std::fs;
// use std::process;
use env_logger::Env;
use log::debug;
// use log::error;
use log::info;
use sysinfo::System;

fn main() -> Result<(), anyhow::Error> {
  // pre-initialization
  // TODO: clap
  env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
  debug!("(os, arch) = {:?}", waka_kicad::env_consts());
  let mut sys = System::new_all();
  sys.refresh_all();
  sys.debug_processes();
  info!("Initializing waka-kicad...");
  let mut plugin = WakaKicad::default();
  plugin.check_cli_installed()?;
  plugin.get_api_key()?;
  plugin.await_connect_to_kicad()?;
  // main loop
  loop {
    plugin.set_current_time(plugin.current_time());
    // TODO
    let board = plugin.await_get_open_board()?.unwrap();
    let Some(identifier) = board.doc.identifier else { unreachable!(); };
    plugin.set_current_file_from_identifier(identifier);
    // TODO: don't sleep - prevents plugin.send_heartbeat(true) from executing immediately
    // this call should be debounced instead as in plugin.enough_time_passed()
    plugin.set_many_items()?;
    sleep(Duration::from_secs(5));
  }
  // TODO: this is unreachable
  Ok(())
}