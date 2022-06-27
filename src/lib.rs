#[macro_use]
extern crate smart_default;

use settings::{FullscreenType, Settings};
use std::{cell::RefCell, collections::HashMap, fs, process::Command};
use wry::{
    application::{
        event::{Event, KeyEvent, StartCause, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        keyboard::Key,
        window::{Fullscreen, Window, WindowBuilder, WindowId},
    },
    http::{Request, Response, ResponseBuilder},
    webview::{WebView, WebViewBuilder},
    Error,
};

mod settings;

pub struct Melange {
    config_dir: String,
    settings: Settings,
}

thread_local! {
    static WEBVIEWS: RefCell<HashMap<WindowId, WebView>> = RefCell::new(HashMap::new());
}

impl Melange {
    pub fn new(config_dir: String) -> Self {
        let settings = Settings::new(&config_dir);

        Melange {
            config_dir,
            settings,
        }
    }

    fn ipc_handler(window: &Window, message: String, commands: &HashMap<String, Vec<String>>) {
        if let Some(inputs) = commands.get(&message) {
            let output = Command::new(&inputs[0])
                .args(&inputs[1..])
                .output()
                .expect("failed to execute process");
            let stdout = String::from_utf8(output.stdout)
                .unwrap()
                // Replace newline characters with '\' + 'n' so that it is not interpreted incorrectly in evaluate_script
                .replace("\n", "\\n");
            println!("{stdout}");

            WEBVIEWS.with(|webviews| {
                let webviews = webviews.borrow();
                if let Some(wv) = webviews.get(&window.id()) {
                    wv.evaluate_script(dbg!(format!("window.response('{stdout}')").as_str()));
                }
            });
        } else {
            println!("not found");
        }
    }

    fn protocol(config_dir: &str, request: &Request) -> Result<Response, wry::Error> {
        // Remove url scheme
        let uri = dbg!(request.uri().replace("melange://", ""));

        if uri.starts_with(config_dir) {
            // TODO: Add check to make sure only files in the config directory can be accessed (with an option, maybe?)

            // get the file's location
            let path = fs::canonicalize(&uri)?;
            // Use MimeGuess to guess a mime type
            let mime = mime_guess::from_path(&path).first_raw().unwrap_or("");

            // Read the file content from file path
            let content = fs::read(path)?;
            ResponseBuilder::new().mimetype(mime).body(content)
        } else {
            ResponseBuilder::new()
                .status(403)
                .mimetype("text/strings")
                .body("Cannot access!".as_bytes().to_vec())
        }
    }

    pub fn make_window(&self, event_loop: &EventLoop<()>) -> Window {
        let window = WindowBuilder::new()
            .with_title(&self.settings.window.title)
            .with_decorations(self.settings.window.decorated)
            .with_always_on_top(self.settings.window.always_on_top)
            .with_transparent(self.settings.window.transparent)
            .with_fullscreen(match self.settings.window.mode {
                FullscreenType::Windowed => None,
                FullscreenType::Borderless => None,
                FullscreenType::Full => Some(Fullscreen::Borderless(None)),
            })
            .build(event_loop)
            .unwrap();

        match self.settings.window.mode {
            FullscreenType::Windowed => {
                // Only set the window size and position if it's specified in the config,
                // otherwise just let the WM handle it with its default behaviour
                if let Some(size) = self.settings.window.size {
                    window.set_inner_size(size);
                };
                if let Some(position) = self.settings.window.position {
                    window.set_outer_position(position);
                };
            }
            FullscreenType::Borderless => {
                let monitor = window.primary_monitor().unwrap();
                window.set_inner_size(monitor.size());
                window.set_outer_position(monitor.position());
            }
            _ => {}
        };

        window
    }

    pub fn make_webview(self, window: Window) {
        // Allow the use of web servers, e.g. for local dev
        let url = if self.config_dir.starts_with("http") {
            self.config_dir.to_owned()
        } else {
            format!("melange://{}/index.html", self.config_dir)
        };

        let id = window.id();
        let webview = WebViewBuilder::new(window)
            .unwrap()
            .with_transparent(true)
            .with_devtools(self.settings.debug.devtools)
            .with_ipc_handler(move |w, m| Melange::ipc_handler(w, m, &self.settings.commands))
            .with_custom_protocol("melange".into(), move |s| {
                Melange::protocol(&self.config_dir, s)
            })
            // tell the webview to load the custom protocol
            .with_url(&url)
            .unwrap()
            .build();

        // This has to be set AFTER any window size changes are made, otherwise they won't take effect
        // Doesn't seem to work with setting a window size, so disabled for now
        // webview.window().set_resizable(false);

        // Insert webview into the static variable, in order to call evaluate_script on it
        if let Ok(wv) = webview {
            WEBVIEWS.with(|webviews| {
                let mut webviews = webviews.borrow_mut();
                webviews.insert(id, wv);
            });
        }
    }

    pub fn run_loop(event_loop: EventLoop<()>) {
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::NewEvents(StartCause::Init) => println!("Wry application started!"),
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => *control_flow = ControlFlow::Exit,
                // TODO: Add setting for quit key?
                Event::WindowEvent {
                    event:
                        WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    logical_key: Key::Escape,
                                    ..
                                },
                            ..
                        },
                    ..
                } => *control_flow = ControlFlow::Exit,
                _ => (),
            }
        });
    }
}
