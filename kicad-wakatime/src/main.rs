use eframe::egui::{self};
// use cocoa::appkit::NSApp;
// use cocoa::appkit::NSApplication;
// use cocoa::appkit::NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular;
use kicad_wakatime::{ui::Ui, traits::{DebugProcesses, FindProcess}, Plugin};
use clap::Parser;
use log::debug;
use sysinfo::System;

/// WakaTime plugin for KiCAD nightly
#[derive(Parser)]
pub struct Args {
  #[clap(long)]
  disable_heartbeats: bool,
  #[clap(long)]
  /// Redownload WakaTime CLI
  redownload: bool,
}

fn main() -> Result<(), anyhow::Error> {
  // pre-initialization
  let args = Args::parse();
  egui_logger::builder().init().unwrap();
  debug!("(os, arch) = {:?}", kicad_wakatime::env_consts());

  #[cfg(target_os = "macos")]
  core_graphics::access::ScreenCaptureAccess::default().request();

  let mut sys = System::new_all();
  sys.refresh_all();
  sys.debug_processes();

  let (tx, rx) = std::sync::mpsc::channel::<Result<notify::Event, notify::Error>>();

  let native_options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default().with_inner_size([640.0, 400.0]),
    ..Default::default()
  };

  // initialization
  let mut plugin = Plugin::new(
    args.disable_heartbeats,
    args.redownload,
  );
  plugin.dual_info(String::from("Initializing kicad-wakatime..."));
  plugin.tx = Some(tx);
  plugin.rx = Some(rx);

  let _ = eframe::run_simple_native(
    "kicad-wakatime ^_^",
    native_options,
    move |ctx, _frame| {
      // draw UI
      plugin.draw_ui(ctx, _frame);
      // connection check
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
    }
  );

  // fltk::app::add_idle3(move |_| {
  // });

  Ok(())
}