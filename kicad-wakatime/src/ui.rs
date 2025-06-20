use std::path::PathBuf;

use eframe::egui::{self, Color32, RichText};
use egui_modal::Modal;
// use log::debug;

use crate::Plugin;

pub trait Ui {
  fn draw_ui(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) -> Result<(), anyhow::Error>;
}

impl Ui for Plugin {
  fn draw_ui(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) -> Result<(), anyhow::Error> {
    let projects_folder = self.get_projects_folder();
    let api_key = self.get_api_key();
    let api_url = self.get_api_url();
    let status = if !self.first_iteration_finished {
      "loading..."
    } else if projects_folder.as_os_str().is_empty() || api_key.is_empty() || api_url.is_empty() {
      "need settings!"
    } else {
      "OK"
    };
    let last_heartbeat_label_text = match self.last_sent_time_chrono {
      Some(dt) => dt.format("%H:%M:%S").to_string(),
      None => String::from("N/A"),
    };
    // settings window
    let modal = Modal::new(ctx, "settings");
    // luckily this call has a generic for the return type!
    modal.show(|ui| -> Result<(), anyhow::Error> {
      ui.label(RichText::new("kicad-wakatime settings ^w^").size(16.0));
      ui.add_space(10.0);
      ui.label("track ALL projects in this folder:");
      // ui.text_edit_singleline(&mut self.watched_folder);
      ui.monospace(format!("{:?}", self.projects_folder));
      if ui.button("select folder").clicked() {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
          self.projects_folder = path.to_str().unwrap().to_string();
        }
      }
      ui.label("API key:");
      ui.text_edit_singleline(&mut self.api_key);
      ui.label("API URL:");
      ui.text_edit_singleline(&mut self.api_url);
      ui.label("Symbol Library Name:");
      ui.text_edit_singleline(&mut self.symbol);
      ui.label("Footprint Library Path:");
      ui.text_edit_singleline(&mut self.footprint);
      if ui.button("OK").clicked() {
        self.set_projects_folder(self.projects_folder.clone());
        self.set_api_key(self.api_key.clone());
        self.set_api_url(self.api_url.clone());
        self.store_config()?;
        self.watch_files(PathBuf::from(self.projects_folder.clone()))?;
        modal.close();
      }
      Ok(())
    });
    // main window
    egui::CentralPanel::default().show(ctx, |ui| {
      // ui.heading("kicad-wakatime");
      ui.label(format!("status: {status}"));
      ui.label(format!("last heartbeat: {last_heartbeat_label_text}"));
      if ui.button("settings").clicked() {
        modal.open();
      }
      ui.add_space(20.0);
      ui.separator();
      egui_logger::logger_ui()
        .warn_color(Color32::YELLOW)
        .error_color(Color32::RED)
        .show(ui);
    });
    Ok(())
  }
}
