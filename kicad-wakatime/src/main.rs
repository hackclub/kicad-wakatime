use kicad_wakatime::ui::{Message, Ui};
use std::thread::sleep;
use std::time::Duration;

// use cocoa::appkit::NSApp;
// use cocoa::appkit::NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular;
use kicad_wakatime::{Plugin, traits::DebugProcesses};
// use std::fs;
// use std::process;
use clap::Parser;
// use cocoa::appkit::NSApplication;
use env_logger::Env;
// use fltk::{prelude::*, window::Window};
use fltk::prelude::*;
use log::debug;
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

  let fltk_app = fltk::app::App::default();

  // initialization
  let mut plugin = Plugin::new(
    args.disable_heartbeats,
    args.sleepy,
  );
  plugin.ui = Some(Ui::new());
  plugin.dual_info(String::from("Initializing kicad-wakatime..."));

  plugin.tx = Some(tx);
  plugin.rx = Some(rx);
  plugin.check_cli_installed()?;
  plugin.get_api_key()?;
  plugin.connect_to_kicad()?;

  plugin.ui.as_mut().unwrap().main_window_ui.main_window.end();
  plugin.ui.as_mut().unwrap().main_window_ui.main_window.show();

  while fltk_app.wait() {
    // if let Some(Message::OpenSettingsWindow) = plugin.ui.as_mut().unwrap().receiver.recv() {
    match plugin.ui.as_mut().unwrap().receiver.recv() {
      Some(Message::OpenSettingsWindow) => {
        plugin.ui.as_mut().unwrap().settings_window_ui.settings_window.show();
      },
      Some(Message::CloseSettingsWindow) => {
        plugin.ui.as_mut().unwrap().settings_window_ui.settings_window.hide();
      },
      None => {},
    }
  }

  fltk::app::add_idle3(move |_| {
    plugin.set_current_time(plugin.current_time());
    let Ok(w) = plugin.get_active_window() else { return; };
    let Some(ref k) = plugin.kicad else { return; };
    if w.title.contains("Schematic Editor") {
      let Ok(schematic) = k.get_open_schematic() else { return; };
      // the KiCAD IPC API does not work properly with schematics as of November 2024
      // (cf. kicad-rs/issues/3), so for the schematic editor, heartbeats for file
      // modification without save cannot be sent
      let schematic_ds = schematic.doc;
      debug!("schematic_ds = {:?}", schematic_ds.clone());
      plugin.set_current_file_from_document_specifier(schematic_ds.clone());
    }
    else if w.title.contains("PCB Editor") {
      // for the PCB editor, we can instead use the Rust bindings proper
      let Ok(board) = k.get_open_board() else { return; };
      let board_ds = board.doc;
      debug!("board_ds = {:?}", board_ds.clone());
      plugin.set_current_file_from_document_specifier(board_ds.clone());
      plugin.set_many_items();
    } else {
      debug!("w.title = {}", w.title);
    }
    plugin.try_recv();
    plugin.first_iteration_finished = true;
    if plugin.sleepy {
      sleep(Duration::from_secs(5));
    }
  });
  
  // fltk_app.run()?;

  Ok(())
}