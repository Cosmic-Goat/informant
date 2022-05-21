use std::borrow::Borrow;
use std::time::Duration;

use ggez::conf::{self, FullscreenType};
use ggez::event::{self, EventLoop};
use ggez::graphics::{self, Color};
use ggez::winit::dpi::LogicalSize;
use ggez::{timer, Context, GameResult};

use keyframe::functions::EaseInOut;

use fontconfig::Fontconfig;
use keyframe::{ease, EasingFunction};

use crate::button::Button;

mod button;

const BACKGROUND: [f32; 4] = [0.1, 0.1, 0.1, 0.6];

struct UI {
    logout: Button,
    sleep: Button,
    power: Button,
}

pub struct MainState {
    dt: Duration,
    time: Duration,
    pos: [f64; 2],
    ui: UI,
    font: graphics::Font,
}

impl MainState {
    fn new(ctx: &mut Context) -> GameResult<MainState> {
        let fc = Fontconfig::new().unwrap();
        let font = fc.find("iosevka", Some("italic")).unwrap();
        println!("{}", font.path.to_str().unwrap());

        let bytes = std::fs::read(font.path).unwrap();
        let font = graphics::Font::new_glyph_font_bytes(ctx, &bytes).unwrap();

        let state = MainState {
            dt: Duration::new(0, 0),
            time: Duration::new(0, 0),
            pos: [0.0, 0.0],
            ui: UI {
                logout: Button::new_empty(String::from("Logout"), Color::WHITE, 2.),
                sleep: Button::new_empty(String::from("Sleep"), Color::WHITE, 2.),
                power: Button::new_empty(String::from("Power"), Color::WHITE, 2.),
            },
            font,
        };

        Ok(state)
    }
}

#[inline]
fn anim<F: EasingFunction>(
    function: impl Borrow<F>,
    seconds: f32,
    offset: f32,
    time: Duration,
) -> f32 {
    return ease(
        function,
        0.0,
        1.0,
        ((time.as_secs_f32() - offset) / seconds).clamp(0.0, 1.0),
    );
}

impl event::EventHandler<ggez::GameError> for MainState {
    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        // Clear the screen.
        graphics::clear(ctx, BACKGROUND.into());

        let anim_time = 1.7;

        // let mesh = graphics::Mesh::new_line(
        //     ctx,
        //     &[glam::vec2(100., 100.), glam::vec2(30., 50.)],
        //     1.,
        //     Color::WHITE,
        // )?;
        // graphics::draw(ctx, &mesh, (glam::vec2(0., 0.),))?;

        self.ui
            .logout
            .anim_rect(anim(EaseInOut, anim_time, 0.0, self.time), ctx)?
            .draw_label(self.font, 32.0, ctx)?;

        self.ui
            .sleep
            .anim_rect(anim(EaseInOut, anim_time, 0.3, self.time), ctx)?
            .draw_label(self.font, 32.0, ctx)?;
        self.ui
            .power
            .anim_rect(anim(EaseInOut, anim_time, 0.6, self.time), ctx)?
            .draw_label(self.font, 32.0, ctx)?;

        let text = graphics::Text::new((
            format!("fps: {}", ggez::timer::fps(ctx).round()),
            self.font,
            48.0,
        ));
        let test = glam::vec2(100.0, 100.0);
        graphics::draw(ctx, &text, (test,))?;

        graphics::present(ctx)?;
        Ok(())
    }

    fn update(&mut self, ctx: &mut Context) -> GameResult {
        // self.ui.logout.set_size(self.ui.logout.rect.width * 1.001, self.ui.logout.rect.height * 1.001);

        self.dt = timer::delta(ctx);
        self.time = timer::time_since_start(ctx);
        Ok(())
    }

    fn resize_event(&mut self, _ctx: &mut Context, width: f32, height: f32) {
        // Button sizes get set here, since a resize event is fired on first draw (I think)
        let button_size = width / 6.0;
        let grid_width = width / 6.0;
        let grid_height = height / 2.0;

        self.ui.logout.set_size(button_size, button_size);
        self.ui.logout.set_pos((1.5 * grid_width, grid_height));

        self.ui.sleep.set_size(button_size, button_size);
        self.ui.sleep.set_pos((3.0 * grid_width, grid_height));

        self.ui.power.set_size(button_size, button_size);
        self.ui.power.set_pos((4.5 * grid_width, grid_height));
    }
}

fn main() -> GameResult {
    // Create an eventloop to get the monitor's size, in case some WMs don't respect set_inner_size
    let size = EventLoop::new().primary_monitor().unwrap().size();
    // TODO: Make this a part of the config
    const FULLSCREEN: FullscreenType = FullscreenType::True;

    let cb = ggez::ContextBuilder::new("informant", "cosmicdoge").window_mode(
        conf::WindowMode::default()
            .dimensions(size.width as f32, size.height as f32)
            .fullscreen_type(FULLSCREEN)
            .transparent(true),
    );
    let (mut ctx, event_loop) = cb.build()?;

    if FULLSCREEN != FullscreenType::True {
        let window = graphics::window(&ctx);
        let monitor = window.current_monitor().unwrap();
        let monitor_width = (monitor.size().width as f64 / monitor.scale_factor()) as i32;
        let monitor_height = (monitor.size().height as f64 / monitor.scale_factor()) as i32;
        let pos = monitor.position();
        window.set_always_on_top(true);
        window.set_decorations(false);
        window.set_resizable(false);
        window.set_outer_position(pos);
        window.set_inner_size(LogicalSize::new(monitor_width, monitor_height));
    }

    let game = MainState::new(&mut ctx)?;
    event::run(ctx, event_loop, game)

    // let mut sequence = keyframes![
    // (0.0, 0.0),
    // (1.)]
}
