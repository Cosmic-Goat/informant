#[macro_use]
extern crate smart_default;

use std::process::Command;
use std::time::Duration;

use ggez::conf::{self, FullscreenType};
use ggez::event::{self, EventLoop, KeyCode, KeyMods};
use ggez::graphics::{self, Color, Font, Image, Rect};
use ggez::winit::dpi::LogicalSize;
use ggez::{timer, Context, GameError, GameResult};

use fontconfig::Fontconfig;
use keyframe::functions::EaseIn;
use settings::{Input, Settings};

use crate::button::Button;

mod anim;
mod button;
mod settings;

struct UI {
    buttons: Vec<Button>,
}

pub struct MainState {
    dt: Duration,
    time: Duration,
    pos: (f32, f32),
    ui: UI,
    scale_factor: f32,
    font: Font,
    config: Settings,
}

impl MainState {
    fn new(
        ctx: &mut Context,
        scale_factor: f32,
        buttons: Vec<Button>,
        settings: Settings,
    ) -> GameResult<MainState> {
        let fc = Fontconfig::new().unwrap();
        let font = fc
            .find(&settings.font.family, settings.font.style.as_deref())
            .unwrap();
        println!("{}", font.path.to_str().unwrap());

        let bytes = std::fs::read(font.path).unwrap();
        let font = Font::new_glyph_font_bytes(ctx, &bytes).unwrap();

        let state = MainState {
            dt: Duration::new(0, 0),
            time: Duration::new(0, 0),
            pos: (0.0, 0.0),
            ui: UI { buttons },
            scale_factor,
            font,
            config: settings,
        };

        Ok(state)
    }

    fn execute(inputs: &Vec<String>) {
        let output = Command::new(&inputs[0])
            .args(&inputs[1..])
            .output()
            .expect("failed to execute process");
        print!("{}", String::from_utf8(output.stdout).unwrap());
    }
}

impl event::EventHandler<GameError> for MainState {
    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        // Colour for the fading-in background effect
        let colour = keyframe::ease(
            EaseIn,
            mint::Vector4::from_slice(&self.config.background.fade_in_color),
            mint::Vector4::from_slice(&self.config.background.color),
            self.time.as_secs_f32() / self.config.background.fade_duration,
        );

        // Clear the screen using the background colour
        graphics::clear(ctx, [colour.x, colour.y, colour.z, colour.w].into());

        let font_size = self.config.font.size * self.scale_factor;

        for (i, button) in self.ui.buttons.iter().enumerate() {
            button
                .draw(
                    self.config.anim.duration,
                    self.config.anim.delay * i as f32,
                    self.time,
                    ctx,
                )?
                .draw_image(ctx)?
                .draw_label(self.font, font_size, ctx)?;
        }

        let text = graphics::Text::new((
            format!(
                "fps: {}, mouse: {} {}",
                ggez::timer::fps(ctx).round(),
                self.pos.0,
                self.pos.1
            ),
            self.font,
            48.0,
        ));
        let test = glam::vec2(100.0, 100.0);
        graphics::draw(ctx, &text, (test,))?;

        graphics::present(ctx)?;
        Ok(())
    }

    fn update(&mut self, ctx: &mut Context) -> GameResult {
        self.dt = timer::delta(ctx);
        self.time = timer::time_since_start(ctx);
        Ok(())
    }

    fn resize_event(&mut self, _ctx: &mut Context, width: f32, height: f32) {
        // Button sizes get set here, since a resize event is fired on first draw (I think)
        // TODO: Maybe make a system for positioning buttons? Auto positioning, manual, etc
        let button_size = width / 6.0;
        // let grid_width = width / 6.0;
        let grid_height = height / 2.0;
        let n = (self.ui.buttons.len() + 1) as f32;

        for (i, button) in self.ui.buttons.iter_mut().enumerate() {
            button.set_size(button_size, button_size);
            button.set_pos((i + 1) as f32 * width / n, grid_height);
        }
    }

    fn key_down_event(&mut self, ctx: &mut Context, key: KeyCode, mods: KeyMods, _repeat: bool) {
        if let Some(command) = self.config.keymap.get(&Input { key, mods }) {
            MainState::execute(command);
        }

        if key == KeyCode::Escape {
            event::quit(ctx)
        }
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32, _xrel: f32, _yrel: f32) {
        // Mouse coordinates are PHYSICAL coordinates, but here we want logical coordinates.

        // If you simply use the initial coordinate system, then physical and logical
        // coordinates are identical.
        self.pos.0 = x;
        self.pos.1 = y;

        for button in &mut self.ui.buttons {
            button.hover(x, y);
        }
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        _button: event::MouseButton,
        _x: f32,
        _y: f32,
    ) {
        for button in &mut self.ui.buttons {
            if button.is_hovered {
                MainState::execute(&button.command)
            }
        }
    }
}

fn main() -> GameResult {
    let cb = ggez::ContextBuilder::new("informant", "cosmicdoge")
        .modules(conf::ModuleConf {
            gamepad: false,
            audio: false,
        })
        .with_conf_file(false)
        .window_mode(conf::WindowMode::default().transparent(true));
    let (mut ctx, event_loop) = cb.build()?;

    let (settings, buttons) = Settings::new(&ctx);

    let window = graphics::window(&ctx);
    let scale = window.scale_factor() as f32;
    let monitor = window.current_monitor().unwrap();

    let monitor_width = monitor.size().width as f32;
    let monitor_height = monitor.size().height as f32;

    graphics::set_fullscreen(&mut ctx, settings.fullscreen);

    if settings.fullscreen != FullscreenType::Windowed {
        // Set window size to cover entire screen.
        graphics::set_drawable_size(&mut ctx, monitor_width, monitor_height);
        graphics::set_screen_coordinates(
            &mut ctx,
            Rect {
                x: 0.,
                y: 0.,
                w: monitor_width,
                h: monitor_height,
            },
        );
    }

    if settings.fullscreen == FullscreenType::Desktop {
        let window = graphics::window(&ctx);
        let pos = monitor.position();
        window.set_always_on_top(true);
        window.set_decorations(false);
        window.set_resizable(false);
        window.set_outer_position(pos);
    }

    // TODO: Handle setting fullscreen result?

    // Convert button data from config file into button structs
    let buttons = buttons
        .iter()
        .map(|b| Button::new_empty(&mut ctx, b, scale))
        .collect();

    let game = MainState::new(&mut ctx, scale, buttons, settings)?;
    event::run(ctx, event_loop, game)
}
