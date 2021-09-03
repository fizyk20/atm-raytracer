use std::{cell::RefCell, rc::Rc};

use fltk::{
    app,
    draw::{draw_rectf, set_draw_color, Offscreen},
    enums::{Color, ColorDepth, Event, Mode},
    frame::Frame,
    group::{Pack, PackType},
    image::RgbImage,
    prelude::*,
    window::Window,
};

use crate::{generator::AllData, renderer};

#[derive(Clone, Debug)]
struct ViewState {
    mouse_x: i32,
    mouse_y: i32,
    pan_x: i32,
    pan_y: i32,
    scale: f64,
    cursor: Option<(i32, i32)>,
    orig_w: i32,
    orig_h: i32,
    image: RgbImage,
}

impl ViewState {
    fn new(image: RgbImage) -> ViewState {
        ViewState {
            mouse_x: 0,
            mouse_y: 0,
            pan_x: 0,
            pan_y: 0,
            scale: 1.0,
            cursor: None,
            orig_w: image.width(),
            orig_h: image.height(),
            image,
        }
    }

    fn draw(&mut self, offs: &mut Offscreen) {
        offs.begin();
        set_draw_color(Color::White);
        draw_rectf(0, 0, self.orig_w, self.orig_h);

        let width = ((self.orig_w as f64) * self.scale) as i32;
        let height = ((self.orig_h as f64) * self.scale) as i32;
        let x = self.pan_x + self.orig_w / 2 - width / 2;
        let y = self.pan_y + self.orig_h / 2 - height / 2;
        self.image.scale(width, height, true, true);
        self.image.draw(x, y, width, height);
        offs.end();
    }

    fn mouse_pos(&self) -> (i32, i32) {
        (self.mouse_x, self.mouse_y)
    }

    fn set_mouse_pos(&mut self, x: i32, y: i32) {
        self.mouse_x = x;
        self.mouse_y = y;
    }

    fn pan(&mut self, x: i32, y: i32) {
        self.pan_x += x;
        self.pan_y += y;
    }

    fn scale(&mut self, scale: f64) {
        self.scale *= scale;
    }
}

const WIDTH: i32 = 1280;
const HEIGHT: i32 = 800;

pub fn run(data: AllData) -> Result<(), String> {
    let app = app::App::default().with_scheme(app::Scheme::Gtk);
    app::set_visual(Mode::Rgb8).unwrap();

    let mut wind = Window::default()
        .with_size(WIDTH, HEIGHT)
        .center_screen()
        .with_label("Atm-Raytracer Panorama Viewer");

    let mut pack = Pack::default()
        .with_size(WIDTH - 10, HEIGHT - 10)
        .center_of(&wind);
    pack.set_spacing(10);
    pack.set_type(PackType::Horizontal);

    let mut frame = Frame::default().with_size(1000, 800);

    pack.end();

    wind.end();
    wind.show();

    let image = renderer::draw_image(&data.result, &data.params).into_raw();
    let image = RgbImage::new(
        &image,
        data.params.output.width as i32,
        data.params.output.height as i32,
        ColorDepth::Rgb8,
    )
    .unwrap();

    let fw = frame.w();
    let fh = frame.h();

    let state = Rc::new(RefCell::new(ViewState::new(image)));

    let offs = Rc::new(RefCell::new(Offscreen::new(fw, fh).unwrap()));
    state.borrow_mut().draw(&mut *offs.borrow_mut());

    let offs_rc = offs.clone();
    let state_rc = state.clone();

    frame.draw(move |f| {
        if offs_rc.borrow().is_valid() {
            offs_rc.borrow().copy(f.x(), f.y(), f.w(), f.h(), 0, 0);
        } else {
            state_rc.borrow_mut().draw(&mut *offs_rc.borrow_mut());
        }
    });

    frame.handle(move |f, ev| match ev {
        Event::Push => {
            let coords = app::event_coords();
            state.borrow_mut().set_mouse_pos(coords.0, coords.1);
            true
        }
        Event::Drag => {
            let coords = app::event_coords();
            let (x, y) = state.borrow().mouse_pos();
            state.borrow_mut().pan(coords.0 - x, coords.1 - y);
            state.borrow_mut().set_mouse_pos(coords.0, coords.1);
            state.borrow_mut().draw(&mut *offs.borrow_mut());
            f.redraw();
            true
        }
        Event::MouseWheel => {
            match app::event_dy() {
                app::MouseWheel::Up => {
                    state.borrow_mut().scale(1.0 / 1.1);
                }
                app::MouseWheel::Down => {
                    state.borrow_mut().scale(1.1);
                }
                _ => (),
            }
            state.borrow_mut().draw(&mut *offs.borrow_mut());
            f.redraw();
            true
        }
        _ => false,
    });

    app.run().unwrap();

    Ok(())
}
