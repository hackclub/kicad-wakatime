use std::path::PathBuf;

// use cocoa::appkit::NSApp;
// use cocoa::appkit::NSApplication;
// use cocoa::appkit::NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular;
use kicad_wakatime::{traits::{DebugProcesses, FindProcess}, Plugin};
use clap::Parser;
use env_logger::Env;
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
  #[clap(long)]
  /// Redownload WakaTime CLI
  redownload: bool,
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

  #[cfg(target_os = "macos")]
  core_graphics::access::ScreenCaptureAccess::default().request();

  let mut sys = System::new_all();
  sys.refresh_all();
  sys.debug_processes();

  let (tx, rx) = std::sync::mpsc::channel::<Result<notify::Event, notify::Error>>();

  let fltk_app = fltk::app::App::default();

  // initialization
  let mut plugin = Plugin::new(
    args.disable_heartbeats,
  );
  plugin.dual_info(String::from("Initializing kicad-wakatime..."));

  plugin.tx = Some(tx);
  plugin.rx = Some(rx);
  plugin.check_cli_installed(args.redownload)?;
  plugin.check_up_to_date()?;
  plugin.load_config();
  plugin.connect_to_kicad()?;

  plugin.ui.main_window_ui.main_window.end();
  plugin.ui.main_window_ui.main_window.show();
  // settings population
  let projects_folder = plugin.get_projects_folder();
  let api_key = plugin.get_api_key();
  let api_url = plugin.get_api_url();
  plugin.ui.settings_window_ui.projects_folder.set_value(projects_folder.to_str().unwrap());
  plugin.ui.settings_window_ui.api_key.set_value(api_key.as_str());
  plugin.ui.settings_window_ui.server_url.set_value(api_url.as_str());

  plugin.watch_files(PathBuf::from(projects_folder));

  fltk::app::add_idle3(move |_| {
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    if plugin.kicad.is_some() && sys.find_process("kicad").is_none() {
      plugin.dual_warn(String::from("Lost connection to KiCAD!"));
      plugin.kicad = None;
      return;
    }
    if plugin.kicad.is_none() && sys.find_process("kicad").is_some() {
      let _ = plugin.connect_to_kicad();
      return;
    }
    // have to handle the error case this way since the callback to add_idle3
    // does not return Result
    match plugin.main_loop() {
      Ok(_) => {},
      Err(e) => { plugin.dual_error(format!("{:?}", e)); }
    };
    match plugin.try_recv() {
      Ok(_) => {},
      Err(e) => {
        plugin.dual_error(format!("{:?}", e));
      }
    };
    plugin.try_ui_recv();
    plugin.ui.main_window_ui.main_window.redraw();
    fltk::app::sleep(0.016);
  });
  
  fltk_app.run()?;

  Ok(())
}