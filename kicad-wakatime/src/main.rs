use std::thread::sleep;
use std::time::Duration;

use kicad_wakatime::{Plugin, traits::DebugProcesses};
// use std::fs;
// use std::process;
use active_win_pos_rs::get_active_window;
use clap::Parser;
use env_logger::Env;
use log::debug;
// use log::error;
use log::info;
use sysinfo::System;

/// WakaTime plugin for KiCAD nightly
#[derive(Parser)]
pub struct Args {
  #[clap(long)]
  debug: bool,
  #[clap(long)]
  disable_heartbeats: bool,
  /// Sleep for 5 seconds after every iteration
  #[clap(long)]
  sleepy: bool,
}

fn main() -> Result<(), anyhow::Error> {
  // pre-initialization
  let args = Args::parse();
  let log_level = match args.debug {
    true => "debug",
    false => "info",
  };
  env_logger::Builder::from_env(Env::default().default_filter_or(log_level)).init();
  debug!("(os, arch) = {:?}", kicad_wakatime::env_consts());
  let mut sys = System::new_all();
  sys.refresh_all();
  sys.debug_processes();

  let (tx, rx) = std::sync::mpsc::channel::<Result<notify::Event, notify::Error>>();

  // initialization
  info!("Initializing kicad-wakatime...");
  let mut plugin = Plugin::new(
    args.disable_heartbeats,
  );
  plugin.tx = Some(tx);
  plugin.rx = Some(rx);
  // plugin.create_file_watcher()?;
  // plugin.file_watcher = Some(watcher);
  plugin.check_cli_installed()?;
  plugin.get_api_key()?;
  plugin.await_connect_to_kicad()?;

  // main loop
  loop {
    plugin.set_current_time(plugin.current_time());
    let w = plugin.get_active_window();
    debug!("w.title = {}", w.title);
    let k = plugin.kicad.as_ref().unwrap();
    if w.title.contains("Schematic Editor") {
      let schematic = k.get_open_schematic()?;
      // the KiCAD IPC API does not work properly with schematics as of November 2024
      // (cf. kicad-rs/issues/3), so for the schematic editor, heartbeats for file
      // modification without save cannot be sent
      let schematic_ds = schematic.doc;
      debug!("schematic_ds = {:?}", schematic_ds.clone());
      plugin.set_current_file_from_document_specifier(schematic_ds.clone())?;
    }
    else if w.title.contains("PCB Editor") {
      // for the PCB editor, we can instead use the Rust bindings proper
      let board = k.get_open_board()?;
      let board_ds = board.doc;
      debug!("board_ds = {:?}", board_ds.clone());
      plugin.set_current_file_from_document_specifier(board_ds.clone())?;
      plugin.set_many_items()?;
    } else {
    }
    plugin.try_recv()?;
    plugin.first_iteration_finished = true;
    if args.sleepy {
      sleep(Duration::from_secs(5));
    }
  }

  // TODO: this is unreachable
  Ok(())
}