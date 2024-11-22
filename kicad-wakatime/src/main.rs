use std::thread::sleep;
use std::time::Duration;

use cocoa::appkit::NSApp;
use cocoa::appkit::NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular;
use kicad_wakatime::{Plugin, traits::DebugProcesses};
// use std::fs;
// use std::process;
use active_win_pos_rs::get_active_window;
use clap::Parser;
use cocoa::appkit::NSApplication;
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

pub struct Ui {
  pub main_window: Window,
  pub status_box: Output,
  pub exit_button: Button,
  pub log_window: Terminal,
  pub last_heartbeat_box: Output,
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

  // initialization
  info!("Initializing kicad-wakatime...");
  let mut plugin = Plugin::new(
    args.disable_heartbeats,
    args.sleepy,
  );

  plugin.tx = Some(tx);
  plugin.rx = Some(rx);
  plugin.check_cli_installed()?;
  plugin.get_api_key()?;
  plugin.connect_to_kicad()?;

  let fltk_app = fltk::app::App::default();

  let mut main_window = Window::new(389, 286, 382, 260, None);
  main_window.set_label(r#"kicad-wakatime ^_^"#);
  main_window.set_type(WindowType::Double);
  main_window.make_resizable(true);
  let mut status_box = Output::new(60, 16, 92, 22, None);
  status_box.set_label(r#"status:"#);
  status_box.set_frame(FrameType::NoBox);
  let mut exit_button = Button::new(303, 15, 64, 22, None);
  exit_button.set_label(r#"exit"#);
  exit_button.set_callback(|_| {});
  let mut log_window = Terminal::new(15, 85, 352, 159, None);
  log_window.set_label(r#"log:"#);
  log_window.set_align(unsafe {std::mem::transmute(5)});
  main_window.resizable(&log_window);
  let mut last_heartbeat_box = Output::new(108, 40, 92, 22, None);
  last_heartbeat_box.set_label(r#"last heartbeat:"#);
  last_heartbeat_box.set_frame(FrameType::NoBox);
  main_window.end();
  main_window.show();

  // while fltk_app.wait() {
  fltk::app::add_idle3(move |_| {
    plugin.set_current_time(plugin.current_time());
    let w = plugin.get_active_window();
    let Ok(w) = w else { return; };
    let k = plugin.kicad.as_ref().unwrap();
    if w.title.contains("Schematic Editor") {
      let schematic = k.get_open_schematic().expect("no schematics are open");
      // the KiCAD IPC API does not work properly with schematics as of November 2024
      // (cf. kicad-rs/issues/3), so for the schematic editor, heartbeats for file
      // modification without save cannot be sent
      let schematic_ds = schematic.doc;
      debug!("schematic_ds = {:?}", schematic_ds.clone());
      plugin.set_current_file_from_document_specifier(schematic_ds.clone());
    }
    else if w.title.contains("PCB Editor") {
      // for the PCB editor, we can instead use the Rust bindings proper
      let board = k.get_open_board().expect("no boards are open");
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
  
  fltk_app.run();

  Ok(())
}