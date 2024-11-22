use kicad_wakatime::Ui;
use std::thread::sleep;
use std::time::Duration;

// use cocoa::appkit::NSApp;
// use cocoa::appkit::NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular;
use kicad_wakatime::{Plugin, traits::DebugProcesses};
// use std::fs;
// use std::process;
use active_win_pos_rs::get_active_window;
use clap::Parser;
// use cocoa::appkit::NSApplication;
use env_logger::Env;
// use fltk::{prelude::*, window::Window};
use fltk::browser::*;
use fltk::button::*;
use fltk::dialog::*;
use fltk::enums::*;
use fltk::frame::*;
use fltk::group::*;
use fltk::group::experimental::*;
use fltk::image::*;
use fltk::input::*;
use fltk::menu::*;
use fltk::misc::*;
use fltk::output::*;
use fltk::{prelude::*, *};
use fltk::table::*;
use fltk::text::*;
use fltk::tree::*;
use fltk::valuator::*;
use fltk::widget::*;
use fltk::window::*;
use log::info;
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

  // #[cfg(target_os = "macos")]
  // unsafe {
  //   let ns_app = NSApplication::sharedApplication(cocoa::base::nil);
  //   ns_app.setActivationPolicy_(NSApplicationActivationPolicyRegular);
  //   ns_app.finishLaunching();
  //   ns_app.run();
  //   println!("got here");
  // }

  let (tx, rx) = std::sync::mpsc::channel::<Result<notify::Event, notify::Error>>();

  let fltk_app = fltk::app::App::default();

  // initialization
  let mut plugin = Plugin::new(
    args.disable_heartbeats,
    args.sleepy,
  );
  plugin.ui = Some(Ui::make_window());
  plugin.dual_info(String::from("Initializing kicad-wakatime..."));

  plugin.tx = Some(tx);
  plugin.rx = Some(rx);
  plugin.check_cli_installed()?;
  plugin.get_api_key()?;
  plugin.connect_to_kicad()?;

  plugin.ui.as_mut().unwrap().main_window.end();
  plugin.ui.as_mut().unwrap().main_window.show();

  // dual_debug(&mut log_window, format!("it works!"));

  // while fltk_app.wait() {
  fltk::app::add_idle3(move |_| {
    plugin.set_current_time(plugin.current_time());
    let w = plugin.get_active_window();
    let Ok(w) = w else { return; };
    let Some(ref k) = plugin.kicad else { return; };
    if w.title.contains("Schematic Editor") {
      let schematic = k.get_open_schematic().expect("no schematics are open");
      // the KiCAD IPC API does not work properly with schematics as of November 2024
      // (cf. kicad-rs/issues/3), so for the schematic editor, heartbeats for file
      // modification without save cannot be sent
      let schematic_ds = schematic.doc;
      // plugin.dual_debug(format!("schematic_ds = {:?}", schematic_ds.clone()));
      plugin.set_current_file_from_document_specifier(schematic_ds.clone());
    }
    else if w.title.contains("PCB Editor") {
      // for the PCB editor, we can instead use the Rust bindings proper
      let board = k.get_open_board().expect("no boards are open");
      let board_ds = board.doc;
      // plugin.dual_debug(format!("board_ds = {:?}", board_ds.clone()));
      plugin.set_current_file_from_document_specifier(board_ds.clone());
      plugin.set_many_items();
    } else {
      // plugin.dual_debug(format!("w.title = {}", w.title));
    }
    plugin.try_recv();
    plugin.first_iteration_finished = true;
    if plugin.sleepy {
      sleep(Duration::from_secs(5));
    }
  });
  
  fltk_app.run();

  Ok(())
}