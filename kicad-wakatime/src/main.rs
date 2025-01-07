#![windows_subsystem = "windows"]

use std::{env, fs::File};
use std::io::Write;
use chrono::Local;
use eframe::egui::{self};
// use cocoa::appkit::NSApp;
// use cocoa::appkit::NSApplication;
// use cocoa::appkit::NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular;
use kicad_wakatime::{ui::Ui, Plugin};
use clap::Parser;
use log::debug;
use log::error;
use log::info;
// use log::warn;
use multi_log::MultiLogger;

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
  env::set_var("RUST_BACKTRACE", "1");
  let args = Args::parse();
  // egui_logger
  let egui_logger = Box::new(egui_logger::builder().build());
  // need to find path like this because Plugin will not have been made yet
  let home_dir = home::home_dir().expect("Unable to get your home directory!");
  let kicad_wakatime_log_path = home_dir.join(".kicad-wakatime.log");
  let target = Box::new(File::create(kicad_wakatime_log_path)?);
  // env_logger
  let env_logger = Box::new(
    env_logger::Builder::new()
      .target(env_logger::Target::Pipe(target))
      .filter(None, log::LevelFilter::Debug)
      .format(|buf, record| {
        writeln!(
          buf,
          "{} [{}] [{}] {}: {}",
          Local::now().format("%H:%M:%S"),
          record.level(),
          record.line().unwrap_or(0),
          record.file().unwrap_or("unknown"),
          record.args(),
        )
      })
      .build()
  );
  MultiLogger::init(vec![egui_logger, env_logger], log::Level::Debug)
    .expect("Could not initialize multi logger!");
  log_panics::init();

  debug!("(os, arch) = {:?}", kicad_wakatime::env_consts());

  #[cfg(target_os = "macos")]
  core_graphics::access::ScreenCaptureAccess::default().request();

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
  info!("Initializing kicad-wakatime...");
  plugin.tx = Some(tx);
  plugin.rx = Some(rx);

  // settings population
  plugin.load_config();
  plugin.projects_folder = plugin.get_projects_folder().to_str().unwrap().to_string();
  plugin.api_key = plugin.get_api_key();
  plugin.api_url = plugin.get_api_url();

  let _ = eframe::run_simple_native(
    "kicad-wakatime ^_^",
    native_options,
    move |ctx, _frame| {
      plugin.draw_ui(ctx, _frame);
      // have to handle the error case this way since the callback does not return Result
      match plugin.main_loop() {
        Ok(_) => {},
        Err(e) => {
          error!("{:?}", e);
          plugin.first_iteration_finished = true;
        }
      };
      match plugin.try_recv() {
        Ok(_) => {},
        Err(e) => {
          error!("{:?}", e);
        }
      };
      ctx.request_repaint();
    }
  );

  Ok(())
}