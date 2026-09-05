//! Pixelated canvas demo for `nano9_raster`. Autoplay cycles primitives; drag to draw.

use nano9_raster::Point;
use nano9_raster_demo::scene::{self, Scene, HEIGHT, HOLD_FRAMES, WIDTH};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::{Clamped, JsCast};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData, MouseEvent};

const IDLE_RESUME_FRAMES: u32 = 180;

struct Demo {
    ctx: CanvasRenderingContext2d,
    canvas: HtmlCanvasElement,
    scene: Scene,
    drag: Drag,
    awaiting_control: bool,
    mode: Mode,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Drag {
    None,
    Chord,
    Control,
}

enum Mode {
    Auto { step: usize, hold: u32 },
    Click { idle: u32 },
}

impl Demo {
    fn new(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
        let ctx = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("no 2d context"))?
            .dyn_into::<CanvasRenderingContext2d>()?;
        ctx.set_image_smoothing_enabled(false);

        let mut demo = Demo {
            ctx,
            canvas,
            scene: Scene::new(),
            drag: Drag::None,
            awaiting_control: false,
            mode: Mode::Auto { step: 0, hold: 0 },
        };
        demo.present()?;
        Ok(demo)
    }

    fn present(&mut self) -> Result<(), JsValue> {
        self.scene.plot_control();
        self.scene.stamp_digit();
        let image = ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(&mut self.scene.buf),
            scene::WIDTH,
            scene::HEIGHT,
        )?;
        self.ctx.put_image_data(&image, 0.0, 0.0)
    }

    fn paint_shape(&mut self) -> Result<(), JsValue> {
        self.scene.pixels = self.scene.collect_pixels();
        self.scene.clear();
        for i in 0..self.scene.pixels.len() {
            let (p, c) = self.scene.pixels[i];
            self.scene.plot(p, c, c, c);
        }
        self.present()
    }

    fn begin_auto(&mut self) -> Result<(), JsValue> {
        self.drag = Drag::None;
        self.awaiting_control = false;
        self.scene.sync_auto_state();
        self.scene.load_shape();
        self.mode = Mode::Auto { step: 0, hold: 0 };
        self.scene.clear();
        self.present()
    }

    fn tick(&mut self) -> Result<(), JsValue> {
        match self.mode {
            Mode::Auto { step, hold } => {
                if hold > 0 {
                    if hold == 1 {
                        self.scene.advance_kind();
                        self.mode = Mode::Auto { step: 0, hold: 0 };
                    } else {
                        self.mode = Mode::Auto {
                            step,
                            hold: hold - 1,
                        };
                    }
                    return Ok(());
                }
                if self.scene.pixels.is_empty() {
                    self.mode = Mode::Auto {
                        step,
                        hold: HOLD_FRAMES,
                    };
                    return self.present();
                }
                if step == 0 {
                    self.scene.clear();
                }
                let n = self.scene.reveal_count();
                let end = (step + n).min(self.scene.pixels.len());
                for i in step..end {
                    let (p, c) = self.scene.pixels[i];
                    self.scene.plot(p, c, c, c);
                }
                let hold = if end >= self.scene.pixels.len() {
                    HOLD_FRAMES
                } else {
                    0
                };
                self.mode = Mode::Auto { step: end, hold };
                self.present()
            }
            Mode::Click { idle } => {
                if self.drag == Drag::None {
                    let idle = idle + 1;
                    if idle >= IDLE_RESUME_FRAMES {
                        self.begin_auto()?;
                    } else {
                        self.mode = Mode::Click { idle };
                    }
                }
                Ok(())
            }
        }
    }

    fn grid_xy(&self, event: &MouseEvent) -> Point {
        let rect = self.canvas.get_bounding_client_rect();
        let w = rect.width().max(1.0);
        let h = rect.height().max(1.0);
        let x =
            ((f64::from(event.client_x()) - rect.left()) * f64::from(WIDTH) / w).floor() as isize;
        let y =
            ((f64::from(event.client_y()) - rect.top()) * f64::from(HEIGHT) / h).floor() as isize;
        (
            x.clamp(0, WIDTH as isize - 1),
            y.clamp(0, HEIGHT as isize - 1),
        )
    }

    fn on_down(&mut self, event: &MouseEvent) -> Result<(), JsValue> {
        if event.button() == 2 {
            event.prevent_default();
            return self.on_right();
        }
        if event.button() != 0 {
            return Ok(());
        }
        self.mode = Mode::Click { idle: 0 };
        let p = self.grid_xy(event);
        if let Some(control) = Scene::control_at(p) {
            self.drag = Drag::None;
            self.awaiting_control = false;
            self.scene.activate_control(control);
            return self.paint_shape();
        }
        if self.scene.kind.has_control_point()
            && (self.awaiting_control || self.scene.near_control(p))
        {
            self.drag = Drag::Control;
            self.awaiting_control = false;
            self.scene.control = p;
        } else {
            self.drag = Drag::Chord;
            self.awaiting_control = false;
            self.scene.start = p;
            self.scene.end = p;
            self.scene.reset_control();
        }
        self.paint_shape()
    }

    fn on_move(&mut self, event: &MouseEvent) -> Result<(), JsValue> {
        if self.drag == Drag::None {
            return Ok(());
        }
        if event.buttons() & 1 == 0 {
            return self.finish_drag(event);
        }
        let p = self.grid_xy(event);
        match self.drag {
            Drag::None => return Ok(()),
            Drag::Chord => {
                if p == self.scene.end {
                    return Ok(());
                }
                self.scene.end = p;
                self.scene.reset_control();
            }
            Drag::Control => {
                if p == self.scene.control {
                    return Ok(());
                }
                self.scene.control = p;
            }
        }
        self.paint_shape()
    }

    fn on_up(&mut self, event: &MouseEvent) -> Result<(), JsValue> {
        if event.button() != 0 {
            return Ok(());
        }
        self.finish_drag(event)
    }

    fn finish_drag(&mut self, event: &MouseEvent) -> Result<(), JsValue> {
        if self.drag == Drag::None {
            return Ok(());
        }
        let p = self.grid_xy(event);
        match self.drag {
            Drag::Chord => {
                self.scene.end = p;
                self.scene.reset_control();
                self.awaiting_control = self.scene.kind.has_control_point();
            }
            Drag::Control => {
                self.scene.control = p;
                self.awaiting_control = false;
            }
            Drag::None => {}
        }
        self.drag = Drag::None;
        self.mode = Mode::Click { idle: 0 };
        self.paint_shape()
    }

    fn on_right(&mut self) -> Result<(), JsValue> {
        if self.drag != Drag::None {
            return Ok(());
        }
        self.awaiting_control = false;
        self.scene.next_kind();
        if self.scene.kind.has_control_point() {
            self.scene.reset_control();
        }
        self.mode = Mode::Click { idle: 0 };
        self.paint_shape()
    }
}

fn request_frame(cb: &Closure<dyn FnMut()>) {
    let _ = web_sys::window()
        .unwrap()
        .request_animation_frame(cb.as_ref().unchecked_ref());
}

fn main() {
    start();
}

fn start() {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document
        .get_element_by_id("grid")
        .unwrap()
        .dyn_into::<HtmlCanvasElement>()
        .unwrap();

    let demo = Rc::new(RefCell::new(Demo::new(canvas.clone()).unwrap()));

    let down_demo = Rc::clone(&demo);
    let on_down = Closure::wrap(Box::new(move |event: MouseEvent| {
        let _ = down_demo.borrow_mut().on_down(&event);
    }) as Box<dyn FnMut(_)>);
    canvas.set_onmousedown(Some(on_down.as_ref().unchecked_ref()));
    on_down.forget();

    let move_demo = Rc::clone(&demo);
    let on_move = Closure::wrap(Box::new(move |event: MouseEvent| {
        let _ = move_demo.borrow_mut().on_move(&event);
    }) as Box<dyn FnMut(_)>);
    canvas.set_onmousemove(Some(on_move.as_ref().unchecked_ref()));
    on_move.forget();

    let up_demo = Rc::clone(&demo);
    let on_up = Closure::wrap(Box::new(move |event: MouseEvent| {
        let _ = up_demo.borrow_mut().on_up(&event);
    }) as Box<dyn FnMut(_)>);
    canvas.set_onmouseup(Some(on_up.as_ref().unchecked_ref()));
    on_up.forget();

    let leave_demo = Rc::clone(&demo);
    let on_leave = Closure::wrap(Box::new(move |event: MouseEvent| {
        let _ = leave_demo.borrow_mut().finish_drag(&event);
    }) as Box<dyn FnMut(_)>);
    canvas.set_onmouseleave(Some(on_leave.as_ref().unchecked_ref()));
    on_leave.forget();

    let on_menu = Closure::wrap(Box::new(move |event: MouseEvent| {
        event.prevent_default();
    }) as Box<dyn FnMut(_)>);
    canvas.set_oncontextmenu(Some(on_menu.as_ref().unchecked_ref()));
    on_menu.forget();

    let raf: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let raf_cb = raf.clone();
    let raf_demo = Rc::clone(&demo);
    *raf.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        let _ = raf_demo.borrow_mut().tick();
        if let Some(cb) = raf_cb.borrow().as_ref() {
            request_frame(cb);
        }
    }) as Box<dyn FnMut()>));
    request_frame(raf.borrow().as_ref().unwrap());
}
