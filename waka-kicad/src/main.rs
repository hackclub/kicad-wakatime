use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use waka_kicad::{WakaKicad, traits::DebugProcesses};
// use std::fs;
// use std::process;
use env_logger::Env;
use log::debug;
use log::error;
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
  let (tx, rx) = std::sync::mpsc::channel::<Result<notify::Event, notify::Error>>();
  let mut watcher = notify::recommended_watcher(tx)?;
  let thr = std::thread::spawn(move || {
    while let Ok(event) = rx.recv() {
      match event {
        Ok(event) => {
          info!("Got event!");
          info!("event = {:?}", event);
        },
        Err(e) => {
          error!("{:?}", e);
        }
      }
    }
  });
  // thr.join().unwrap();
  info!("Initializing waka-kicad...");
  let mut plugin = WakaKicad::default();
  plugin.file_watcher = Some(watcher);
  plugin.check_cli_installed()?;
  plugin.get_api_key()?;
  plugin.await_connect_to_kicad()?;
  // main loop
  loop {
    plugin.set_current_time(plugin.current_time());
    // TODO
    let board = plugin.await_get_open_board()?.unwrap();
    // let Some(identifier) = board.doc.identifier else { unreachable!(); };
    let specifier = board.doc;
    // plugin.set_current_file_from_identifier(identifier)?;
    plugin.set_current_file_from_document_specifier(specifier)?;
    plugin.set_many_items()?;
    plugin.first_iteration_finished = true;
    // sleep(Duration::from_secs(5));
  }
  // TODO: this is unreachable
  Ok(())
}