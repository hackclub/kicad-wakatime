use std::thread::sleep;
use std::time::Duration;

use kicad_wakatime::{Plugin, traits::DebugProcesses};
// use std::fs;
// use std::process;
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

  // initialization
  info!("Initializing kicad-wakatime...");
  let mut plugin = Plugin::new(
    args.disable_heartbeats,
  );
  plugin.create_file_watcher()?;
  // plugin.file_watcher = Some(watcher);
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
    plugin.try_recv()?;
    plugin.first_iteration_finished = true;
    if args.sleepy {
      sleep(Duration::from_secs(5));
    }
  }

  // TODO: this is unreachable
  Ok(())
}