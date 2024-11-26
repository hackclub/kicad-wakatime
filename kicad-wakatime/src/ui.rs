//     // settings window
//     let mut settings_window = Window::new(516, 350, 456, 195, None);
//     settings_window.make_modal(true);
//     settings_window.set_label(r#"kicad-wakatime settings ^w^"#);
//     settings_window.set_type(WindowType::Double);
//     let mut projects_folder = Input::new(15, 29, 420, 24, None);
//     projects_folder.set_label(r#"track ALL projects in this folder:"#);
//     projects_folder.set_align(unsafe { std::mem::transmute(5)});
//     let mut api_key = Input::new(15, 74, 420, 24, None);
//     api_key.set_label(r#"WakaTime API key:"#);
//     api_key.set_align(unsafe {std::mem::transmute(5)});
//     let mut server_url = InputChoice::new(16, 118, 420, 24, None);
//     server_url.set_label(r#"WakaTime API url:"#);
//     server_url.set_align(unsafe {std::mem::transmute(5)});
//     server_url.add("https:\\/\\/api.wakatime.com\\/api\\/v1");
//     server_url.add("https:\\/\\/waka.hackclub.com\\/api");
//     let mut ok_button = ReturnButton::new(349, 157, 86, 22, None);
//     ok_button.set_label(r#"okay!"#);
//     ok_button.set_callback(move |_| {
//       sender.send(Message::UpdateSettings);
//       sender.send(Message::CloseSettingsWindow);
//     });
//     settings_window.end();
//   }
// }

use eframe::egui::{self, Color32};

use crate::Plugin;

pub trait Ui {
  fn draw_ui(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame);
}

impl Ui for Plugin {
  fn draw_ui(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
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
    egui::CentralPanel::default().show(ctx, |ui| {
      // ui.heading("kicad-wakatime");
      ui.label(format!("status: {status}"));
      ui.label(format!("last heartbeat: {last_heartbeat_label_text}"));
      ui.add_space(20.0);
      ui.separator();
      egui_logger::logger_ui()
        .warn_color(Color32::YELLOW)
        .error_color(Color32::RED)
        .show(ui);
    });
  }
}