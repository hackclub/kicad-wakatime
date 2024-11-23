use fltk::app::Receiver;
use fltk::app::Sender;
use fltk::button::*;
use fltk::enums::*;
use fltk::input::Input;
use fltk::misc::InputChoice;
use fltk::output::*;
use fltk::prelude::*;
use fltk::window::*;
use fltk::group::experimental::Terminal;

#[derive(Clone, Copy, Debug)]
pub enum Message {
  OpenSettingsWindow,
  CloseSettingsWindow,
}

#[derive(Clone, Debug)]
pub struct MainWindowUi {
	pub main_window: Window,
	pub status_box: Output,
	// pub exit_button: Button,
	pub log_window: Terminal,
	pub last_heartbeat_box: Output,
	pub settings_button: Button,
}

#[derive(Clone, Debug)]
pub struct SettingsWindowUi {
  pub settings_window: Window,
  pub api_key: Input,
  pub server_url: InputChoice,
  pub ok_button: ReturnButton,
}

pub struct Ui {
  pub sender: Sender<Message>,
  pub receiver: Receiver<Message>,
  pub main_window_ui: MainWindowUi,
  pub settings_window_ui: SettingsWindowUi,
}

impl Ui {
  pub fn new() -> Self {
    let (sender, receiver) = fltk::app::channel::<Message>();
    // main window
    let mut main_window = Window::new(389, 286, 382, 260, None);
    main_window.set_label(r#"kicad-wakatime ^_^"#);
    main_window.set_type(WindowType::Double);
    main_window.make_resizable(true);
    let mut status_box = Output::new(60, 16, 92, 22, None);
    status_box.set_label(r#"status:"#);
    status_box.set_frame(FrameType::NoBox);
    // let mut exit_button = Button::new(303, 40, 64, 22, None);
    // exit_button.set_label(r#"exit"#);
    let mut log_window = Terminal::new(15, 85, 352, 159, None);
    log_window.set_label(r#"log:"#);
    log_window.set_align(unsafe {std::mem::transmute(5)});
    main_window.resizable(&log_window);
    let mut last_heartbeat_box = Output::new(108, 40, 92, 22, None);
    last_heartbeat_box.set_label(r#"last heartbeat:"#);
    last_heartbeat_box.set_frame(FrameType::NoBox);
    let mut settings_button = Button::new(303, 16, 64, 22, None);
    settings_button.set_label(r#"settings"#);
    settings_button.set_callback(move |_| {
      sender.send(Message::OpenSettingsWindow);
    });
    main_window.end();
    main_window.show();
    let main_window_ui = MainWindowUi {
      main_window,
      status_box,
      // exit_button,
      log_window,
      last_heartbeat_box,
      settings_button,
    };
    // settings window
    let mut settings_window = Window::new(516, 350, 456, 158, None);
    settings_window.set_label(r#"kicad-wakatime settings ^w^"#);
    settings_window.set_type(WindowType::Double);
    let mut api_key = Input::new(15, 26, 420, 24, None);
    api_key.set_label(r#"hey, what's your WakaTime API key?"#);
    api_key.set_align(unsafe {std::mem::transmute(5)});
    let mut server_url = InputChoice::new(15, 73, 420, 24, None);
    server_url.set_label(r#"what server do you want to connect to?"#);
    server_url.set_align(unsafe {std::mem::transmute(5)});
    let mut ok_button = ReturnButton::new(349, 115, 86, 22, None);
    ok_button.set_label(r#"okay!"#);
    ok_button.set_callback(move |_| {
      sender.send(Message::CloseSettingsWindow);
    });
    settings_window.end();
    let settings_window_ui = SettingsWindowUi {
      settings_window,
      api_key,
      server_url,
      ok_button,
    };
    Self {
      sender,
      receiver,
      main_window_ui,
      settings_window_ui,
    }
  }
}