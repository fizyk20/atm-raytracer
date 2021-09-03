use std::{cell::RefCell, rc::Rc};

use fltk::{
    app,
    draw::{draw_arc, draw_line, draw_rectf, set_draw_color, Offscreen},
    enums::{Align, Color, ColorDepth, Event, Key, Mode},
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

const CURSOR_SIZE: i32 = 20;
const CURSOR_RADIUS: i32 = 10;

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

        let (width, height) = self.img_size();
        let (tx, ty) = self.translation();
        self.image.scale(width, height, true, true);
        self.image.draw(tx, ty, width, height);

        if let Some((cx, cy)) = self.cursor {
            let (ow, oh) = (self.orig_w as f64 + 1.0, self.orig_h as f64 + 1.0);
            let x = ((cx as f64 - ow / 2.0) * self.scale + ow / 2.0) as i32 + self.pan_x;
            let y = ((cy as f64 - oh / 2.0) * self.scale + oh / 2.0) as i32 + self.pan_y;
            draw_line(x - CURSOR_SIZE, y, x + CURSOR_SIZE, y);
            draw_line(x, y - CURSOR_SIZE, x, y + CURSOR_SIZE);
            draw_arc(
                x - CURSOR_RADIUS,
                y - CURSOR_RADIUS,
                CURSOR_RADIUS * 2 + 1,
                CURSOR_RADIUS * 2 + 1,
                0.0,
                360.0,
            );
        }

        offs.end();
    }

    fn img_size(&self) -> (i32, i32) {
        let width = ((self.orig_w as f64) * self.scale) as i32;
        let height = ((self.orig_h as f64) * self.scale) as i32;
        (width, height)
    }

    fn translation(&self) -> (i32, i32) {
        let (width, height) = self.img_size();
        let tx = self.pan_x + self.orig_w / 2 - width / 2;
        let ty = self.pan_y + self.orig_h / 2 - height / 2;
        (tx, ty)
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
        let (x0, y0) = (500.0, 400.0);
        let (ow, oh) = (self.orig_w as f64 + 1.0, self.orig_h as f64 + 1.0);
        let x_inv = (x0 - ow / 2.0 - self.pan_x as f64) / self.scale + ow / 2.0;
        let y_inv = (y0 - oh / 2.0 - self.pan_y as f64) / self.scale + oh / 2.0;
        self.scale *= scale;
        self.pan_x = (x0 - ow / 2.0 - (x_inv - ow / 2.0) * self.scale) as i32;
        self.pan_y = (y0 - oh / 2.0 - (y_inv - oh / 2.0) * self.scale) as i32;
    }

    fn clear_cursor(&mut self) {
        self.cursor = None;
    }

    fn set_cursor(&mut self, x: i32, y: i32) {
        let x =
            ((x - self.pan_x - self.orig_w / 2 - 5) as f64 / self.scale) as i32 + self.orig_w / 2;
        let y =
            ((y - self.pan_y - self.orig_h / 2 - 5) as f64 / self.scale) as i32 + self.orig_h / 2;
        println!("Cursor set! {}, {}", x, y);
        self.cursor = Some((x, y));
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

    let label = Frame::default()
        .with_size(200, 0)
        .with_label("Test\nTest2\nTest3")
        .with_align(Align::Inside | Align::Left | Align::Top);

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

    let state_rc = state.clone();
    let offs_rc = offs.clone();

    frame.handle(move |f, ev| match ev {
        Event::Push => {
            let coords = app::event_coords();
            state_rc.borrow_mut().set_mouse_pos(coords.0, coords.1);
            true
        }
        Event::Drag => {
            let coords = app::event_coords();
            let (x, y) = state_rc.borrow().mouse_pos();
            state_rc.borrow_mut().pan(coords.0 - x, coords.1 - y);
            state_rc.borrow_mut().set_mouse_pos(coords.0, coords.1);
            state_rc.borrow_mut().draw(&mut *offs_rc.borrow_mut());
            f.redraw();
            true
        }
        Event::MouseWheel => {
            match app::event_dy() {
                app::MouseWheel::Up => {
                    state_rc.borrow_mut().scale(1.0 / 1.1);
                }
                app::MouseWheel::Down => {
                    state_rc.borrow_mut().scale(1.1);
                }
                _ => (),
            }
            state_rc.borrow_mut().draw(&mut *offs_rc.borrow_mut());
            f.redraw();
            true
        }
        _ => false,
    });

    wind.handle(move |_, ev| match ev {
        Event::KeyDown => match app::event_key() {
            Key::Escape => {
                println!("Cursor cleared!");
                state.borrow_mut().clear_cursor();
                state.borrow_mut().draw(&mut *offs.borrow_mut());
                frame.redraw();
                true
            }
            _ => {
                let string = app::event_text();
                if string.starts_with(" ") {
                    let coords = app::event_coords();
                    state.borrow_mut().set_cursor(coords.0, coords.1);
                    state.borrow_mut().draw(&mut *offs.borrow_mut());
                    frame.redraw();
                    true
                } else {
                    false
                }
            }
        },
        _ => false,
    });

    app.run().unwrap();

    Ok(())
}
