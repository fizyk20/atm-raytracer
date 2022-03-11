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

#[derive(Clone)]
struct ViewState {
    mouse_x: i32,
    mouse_y: i32,
    pan_x: f64,
    pan_y: f64,
    scale: f64,
    cursor: Option<(i32, i32)>,
    orig_w: f64,
    orig_h: f64,
    frame_w: i32,
    frame_h: i32,
    image: RgbImage,
    data: AllData,
}

const CURSOR_SIZE: i32 = 20;
const CURSOR_RADIUS: i32 = 10;

const INFO_TITLE: &'static str = "Info about the selected pixel:";
const INFO_NONE: &'static str = "<none>";

fn as_dms(ang: f64) -> (usize, usize, usize) {
    let ang = ang.abs();
    let deg = ang as usize;
    let min = ((ang - deg as f64) * 60.0) as usize;
    let sec = ((ang - deg as f64 - min as f64 / 60.0) * 3600.0) as usize;
    (deg, min, sec)
}

impl ViewState {
    fn new(image: RgbImage, data: AllData, frame_w: i32, frame_h: i32) -> ViewState {
        ViewState {
            mouse_x: 0,
            mouse_y: 0,
            pan_x: 0.0,
            pan_y: 0.0,
            scale: 1.0,
            cursor: None,
            orig_w: image.width() as f64,
            orig_h: image.height() as f64,
            frame_w,
            frame_h,
            image,
            data,
        }
    }

    fn image_to_frame(&self, x: f64, y: f64) -> (f64, f64) {
        (
            (x - self.orig_w / 2.0) * self.scale + self.orig_w / 2.0 + self.pan_x,
            (y - self.orig_h / 2.0) * self.scale + self.orig_h / 2.0 + self.pan_y,
        )
    }

    fn frame_to_image(&self, x: f64, y: f64) -> (f64, f64) {
        (
            (x - self.pan_x - self.orig_w / 2.0) / self.scale + self.orig_w / 2.0,
            (y - self.pan_y - self.orig_h / 2.0) / self.scale + self.orig_h / 2.0,
        )
    }

    fn img_size(&self) -> (i32, i32) {
        let width = (self.orig_w * self.scale) as i32;
        let height = (self.orig_h * self.scale) as i32;
        (width, height)
    }

    fn draw(&mut self, offs: &mut Offscreen) {
        offs.begin();
        set_draw_color(Color::White);
        draw_rectf(0, 0, self.frame_w, self.frame_h);

        let (width, height) = self.img_size();
        let (tx, ty) = self.image_to_frame(0.0, 0.0);
        self.image.scale(width, height, true, true);
        self.image.draw(tx as i32, ty as i32, width, height);

        if let Some((cx, cy)) = self.cursor {
            let (x, y) = self.image_to_frame(cx as f64 + 0.5, cy as f64 + 0.5);
            let (x, y) = (x as i32, y as i32);
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

    fn set_label(&self, frame: &mut Frame) {
        let label = if let Some((cx, cy)) = self.cursor {
            if cx < 0
                || cx >= self.data.params.output.width as i32
                || cy < 0
                || cy >= self.data.params.output.height as i32
            {
                format!("{} {}", INFO_TITLE, INFO_NONE)
            } else {
                let x = cx as usize;
                let y = cy as usize;
                let pixel = &self.data.result[y][x];
                let elev_ang = pixel.elevation_angle;
                let azim = pixel.azimuth;
                let rest = if let Some(tp) = pixel.trace_points.first() {
                    let lon = as_dms(tp.lon);
                    let lat = as_dms(tp.lat);
                    format!(
                        "Physical data:\n\
                        Distance: {:.1} km ({:.1} mi)\n\
                        Elevation: {:.1} m ({:.0} ft)\n\
                        Latitude: {}°{}'{}\"{} ({:.6})\n\
                        Longitude: {}°{}'{}\"{} ({:.6})",
                        tp.distance / 1000.0,
                        tp.distance / 1609.0,
                        tp.elevation,
                        tp.elevation / 0.304,
                        lat.0,
                        lat.1,
                        lat.2,
                        if tp.lat >= 0.0 { "N" } else { "S" },
                        tp.lat,
                        lon.0,
                        lon.1,
                        lon.2,
                        if tp.lon >= 0.0 { "E" } else { "W" },
                        tp.lon
                    )
                } else {
                    format!("Physical data: {}", INFO_NONE)
                };
                format!(
                    "{}\n\n\
                    Pixel coordinates: ({}, {})\n\n\
                    Viewing direction:\n\
                    Elevation: {:.3}°\n\
                    Azimuth: {:.3}°\n\n\
                    {}",
                    INFO_TITLE, x, y, elev_ang, azim, rest
                )
            }
        } else {
            format!("{} {}", INFO_TITLE, INFO_NONE)
        };
        frame.set_label(&label);
    }

    fn mouse_pos(&self) -> (i32, i32) {
        (self.mouse_x, self.mouse_y)
    }

    fn set_mouse_pos(&mut self, x: i32, y: i32) {
        self.mouse_x = x;
        self.mouse_y = y;
    }

    fn pan(&mut self, x: i32, y: i32) {
        self.pan_x += x as f64;
        self.pan_y += y as f64;
    }

    fn scale(&mut self, scale: f64) {
        let (x0, y0) = (500.0, 400.0);
        let (x_inv, y_inv) = self.frame_to_image(x0, y0);
        self.scale *= scale;
        let (x1, y1) = self.image_to_frame(x_inv, y_inv);
        self.pan_x += x0 - x1;
        self.pan_y += y0 - y1;
    }

    fn clear_cursor(&mut self) {
        self.cursor = None;
    }

    fn set_cursor(&mut self, x: i32, y: i32) {
        let (cx, cy) = self.frame_to_image(x as f64, y as f64);
        self.cursor = Some(((cx - 0.5) as i32, (cy - 0.5) as i32));
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

    let fw = 950;
    let fh = 800;

    let mut frame = Frame::default().with_size(fw, fh);

    let mut label = Frame::default()
        .with_size(310, 0)
        .with_label(&format!("{} {}", INFO_TITLE, INFO_NONE))
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

    let state = Rc::new(RefCell::new(ViewState::new(image, data, fw, fh)));

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
                state.borrow_mut().clear_cursor();
                state.borrow_mut().draw(&mut *offs.borrow_mut());
                frame.redraw();
                state.borrow().set_label(&mut label);
                true
            }
            _ => {
                let string = app::event_text();
                if string.starts_with(" ") {
                    let coords = app::event_coords();
                    state.borrow_mut().set_cursor(coords.0, coords.1);
                    state.borrow_mut().draw(&mut *offs.borrow_mut());
                    frame.redraw();
                    state.borrow().set_label(&mut label);
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
