// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later
use fcitx5_ink_core::{model, recognize, socket};
use gtk::prelude::*;
use gtk::{
    gdk, glib, Application, ApplicationWindow, Box as GtkBox, Button, DrawingArea, FlowBox,
    HeaderBar, Label, Orientation,
};
use std::cell::RefCell;
use std::env;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{self, TryRecvError};
use std::time::Duration;

#[derive(Default)]
struct Ink {
    strokes: Vec<Vec<(f32, f32)>>,
    current: Vec<(f32, f32)>,
    drag_start: Option<(f64, f64)>,
    candidates: Vec<String>,
    recognizing: bool,
    committing: bool,
}

struct Config {
    socket: PathBuf,
    model: Option<PathBuf>,
}

#[derive(Clone)]
struct Ui {
    window: ApplicationWindow,
    canvas: DrawingArea,
    candidates: FlowBox,
    status: Label,
    ink: Rc<RefCell<Ink>>,
    config: Rc<Config>,
}

fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };

    if args.self_test {
        std::process::exit(run_self_test(&args));
    }

    // The panel is a normal application window. It must not become an fcitx5
    // input context of its own, or the addon could commit the text back into
    // this panel instead of the editor that was focused before it opened.
    env::set_var("GTK_IM_MODULE", "gtk-im-context-simple");

    let model = model::discover(args.model.as_deref());
    let config = Rc::new(Config {
        socket: args.socket,
        model,
    });
    let app = Application::builder()
        .application_id("app.fcitx5.Ink")
        .build();
    app.connect_activate(move |app| build_ui(app, config.clone()));
    // Our --socket/--model options are consumed above. Do not let
    // GApplication parse them again and reject them as unknown.
    let program = env::args()
        .next()
        .unwrap_or_else(|| "fcitx5-ink-panel".to_owned());
    let _ = app.run_with_args(&[program]);
}

fn parse_args() -> Result<Args, String> {
    let mut socket = socket::default_path()
        .ok_or_else(|| "XDG_RUNTIME_DIR must be an absolute path".to_owned())?;
    let mut model = None;
    let mut self_test = false;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--socket" => {
                socket = PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| "--socket needs a path".to_owned())?,
                );
            }
            "--model" => {
                model = Some(PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| "--model needs a path".to_owned())?,
                ));
            }
            "--self-test" => self_test = true,
            "--help" | "-h" => {
                println!("Usage: fcitx5-ink-panel [--socket PATH] [--model PATH] [--self-test]");
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(Args {
        socket,
        model,
        self_test,
    })
}

struct Args {
    socket: PathBuf,
    model: Option<PathBuf>,
    self_test: bool,
}

fn run_self_test(args: &Args) -> i32 {
    let Some(model) = model::discover(args.model.as_deref()) else {
        eprintln!("handwriting model not found");
        return 1;
    };
    match recognize::recognize(&model, &recognize::zhong_fixture()) {
        Ok(candidates) if !candidates.is_empty() => {
            println!("model: {}", model.display());
            println!("candidates: {}", candidates.join(" "));
            0
        }
        Ok(_) => {
            eprintln!("recognizer returned no candidates for the 中 fixture");
            1
        }
        Err(error) => {
            eprintln!("recognition failed: {error}");
            1
        }
    }
}

fn build_ui(app: &Application, config: Rc<Config>) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("墨笔手写")
        .default_width(620)
        .default_height(500)
        .build();

    let header = HeaderBar::new();
    let copy_button = Button::with_label("复制首选");
    header.pack_end(&copy_button);
    window.set_titlebar(Some(&header));

    let root = GtkBox::new(Orientation::Vertical, 8);
    root.set_margin_top(8);
    root.set_margin_bottom(8);
    root.set_margin_start(8);
    root.set_margin_end(8);

    let canvas = DrawingArea::new();
    canvas.set_size_request(420, 300);
    canvas.set_hexpand(true);
    canvas.set_vexpand(true);
    root.append(&canvas);

    let candidates = FlowBox::new();
    candidates.set_selection_mode(gtk::SelectionMode::None);
    candidates.set_max_children_per_line(6);
    candidates.set_column_spacing(6);
    candidates.set_row_spacing(6);
    candidates.set_hexpand(true);
    root.append(&candidates);

    let status = Label::new(Some("写一个字，松开后识别"));
    status.set_xalign(0.0);
    root.append(&status);
    window.set_child(Some(&root));

    let ui = Ui {
        window: window.clone(),
        canvas: canvas.clone(),
        candidates,
        status,
        ink: Rc::new(RefCell::new(Ink::default())),
        config,
    };

    let draw_ink = ui.ink.clone();
    canvas.set_draw_func(move |area, context, width, height| {
        let ink = draw_ink.borrow();
        draw_strokes(area, context, width, height, &ink.strokes);
        if !ink.current.is_empty() {
            draw_one_stroke(area, context, width, height, &ink.current);
        }
    });

    let drag = gtk::GestureDrag::new();
    drag.set_button(0);
    let drag_begin_ui = ui.clone();
    drag.connect_drag_begin(move |_, start_x, start_y| {
        let mut ink = drag_begin_ui.ink.borrow_mut();
        ink.drag_start = Some((start_x, start_y));
        ink.current.clear();
        if let Some(point) = model_point(
            start_x,
            start_y,
            drag_begin_ui.canvas.width(),
            drag_begin_ui.canvas.height(),
        ) {
            ink.current.push(point);
        }
        drag_begin_ui.canvas.queue_draw();
    });
    let drag_update_ui = ui.clone();
    drag.connect_drag_update(move |_, offset_x, offset_y| {
        let mut ink = drag_update_ui.ink.borrow_mut();
        let Some((start_x, start_y)) = ink.drag_start else {
            return;
        };
        let Some(point) = model_point(
            start_x + offset_x,
            start_y + offset_y,
            drag_update_ui.canvas.width(),
            drag_update_ui.canvas.height(),
        ) else {
            return;
        };
        if ink
            .current
            .last()
            .is_none_or(|last| distance(*last, point) >= 0.8)
        {
            ink.current.push(point);
        }
        drag_update_ui.canvas.queue_draw();
    });
    let drag_end_ui = ui.clone();
    drag.connect_drag_end(move |_, _offset_x, _offset_y| {
        let finished = {
            let mut ink = drag_end_ui.ink.borrow_mut();
            ink.drag_start = None;
            if ink.current.len() >= 2 {
                Some(std::mem::take(&mut ink.current))
            } else {
                ink.current.clear();
                None
            }
        };
        drag_end_ui.canvas.queue_draw();
        if let Some(stroke) = finished {
            drag_end_ui.ink.borrow_mut().strokes.push(stroke);
            drag_end_ui.canvas.queue_draw();
            drag_end_ui.recognize();
        }
    });
    canvas.add_controller(drag);

    let copy_ui = ui.clone();
    copy_button.connect_clicked(move |_| {
        let first = copy_ui.ink.borrow().candidates.first().cloned();
        match first {
            Some(candidate) => {
                copy_ui.copy_text(&candidate);
                copy_ui.set_status("已复制到剪贴板");
            }
            None => copy_ui.set_status("还没有候选"),
        }
    });

    window.present();
}

impl Ui {
    fn set_status(&self, text: &str) {
        self.status.set_text(text);
    }

    fn copy_text(&self, text: &str) {
        if let Some(display) = gdk::Display::default() {
            display.clipboard().set_text(text);
        }
    }

    fn clear_candidates(&self) {
        while let Some(child) = self.candidates.first_child() {
            self.candidates.remove(&child);
        }
        self.ink.borrow_mut().candidates.clear();
    }

    fn show_candidates(&self, list: &[String]) {
        self.clear_candidates();
        self.ink.borrow_mut().candidates = list.to_vec();
        for (index, candidate) in list.iter().enumerate() {
            let button = Button::with_label(&format!("{}. {}", index + 1, candidate));
            button.set_tooltip_text(Some(candidate));
            let ui = self.clone();
            let text = candidate.clone();
            button.connect_clicked(move |_| ui.commit(text.clone()));
            self.candidates.insert(&button, -1);
        }
        if list.is_empty() {
            self.set_status("未识别到内容，重写一次");
        } else {
            self.set_status("点击候选输入，或点“复制首选”");
        }
    }

    fn recognize(&self) {
        if self.ink.borrow().recognizing || self.ink.borrow().committing {
            return;
        }
        let strokes = self.ink.borrow().strokes.clone();
        if strokes.is_empty() {
            return;
        }
        let Some(model) = self.config.model.clone() else {
            self.set_status("未找到 handwriting-zh_CN.model");
            return;
        };

        self.ink.borrow_mut().recognizing = true;
        self.set_status("识别中…");
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(recognize::recognize(&model, &strokes));
        });

        let ui = self.clone();
        glib::timeout_add_local(Duration::from_millis(40), move || {
            match receiver.try_recv() {
                Ok(Ok(candidates)) => {
                    ui.ink.borrow_mut().recognizing = false;
                    ui.show_candidates(&candidates);
                    glib::ControlFlow::Break
                }
                Ok(Err(error)) => {
                    ui.ink.borrow_mut().recognizing = false;
                    ui.set_status(&format!("识别失败：{error}"));
                    glib::ControlFlow::Break
                }
                Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(TryRecvError::Disconnected) => {
                    ui.ink.borrow_mut().recognizing = false;
                    ui.set_status("识别线程已退出");
                    glib::ControlFlow::Break
                }
            }
        });
    }

    fn commit(&self, text: String) {
        if self.ink.borrow().committing {
            return;
        }
        self.ink.borrow_mut().committing = true;
        self.set_status("输入中…");

        // The panel has focus while the user picks a candidate. Hide it first
        // so focus can return to the editor; the addon's broker then commits
        // the text to that context.
        self.window.set_visible(false);
        let socket = self.config.socket.clone();
        let text_for_copy = text.clone();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(120));
            let _ = sender.send(socket::send_text(&socket, &text));
        });

        let ui = self.clone();
        glib::timeout_add_local(Duration::from_millis(40), move || {
            match receiver.try_recv() {
                Ok(Ok(())) => {
                    ui.ink.borrow_mut().committing = false;
                    ui.ink.borrow_mut().strokes.clear();
                    ui.ink.borrow_mut().current.clear();
                    ui.clear_candidates();
                    ui.canvas.queue_draw();
                    ui.window.set_visible(true);
                    ui.set_status("已输入，继续写下一个字");
                    glib::ControlFlow::Break
                }
                Ok(Err(error)) => {
                    ui.ink.borrow_mut().committing = false;
                    ui.window.set_visible(true);
                    ui.copy_text(&text_for_copy);
                    ui.set_status(&format!("输入失败（{error}），已复制到剪贴板"));
                    glib::ControlFlow::Break
                }
                Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(TryRecvError::Disconnected) => {
                    ui.ink.borrow_mut().committing = false;
                    ui.window.set_visible(true);
                    ui.copy_text(&text_for_copy);
                    ui.set_status("输入线程已退出，已复制到剪贴板");
                    glib::ControlFlow::Break
                }
            }
        });
    }
}

fn model_point(x: f64, y: f64, width: i32, height: i32) -> Option<(f32, f32)> {
    if width <= 0 || height <= 0 || !x.is_finite() || !y.is_finite() {
        return None;
    }
    let scale_x = recognize::CANVAS as f64 / f64::from(width);
    let scale_y = recognize::CANVAS as f64 / f64::from(height);
    let point = (
        (x * scale_x).clamp(0.0, f64::from(recognize::CANVAS)) as f32,
        (y * scale_y).clamp(0.0, f64::from(recognize::CANVAS)) as f32,
    );
    Some(point)
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

fn draw_strokes(
    area: &DrawingArea,
    context: &gtk::cairo::Context,
    width: i32,
    height: i32,
    strokes: &[Vec<(f32, f32)>],
) {
    for stroke in strokes {
        draw_one_stroke(area, context, width, height, stroke);
    }
}

fn draw_one_stroke(
    area: &DrawingArea,
    context: &gtk::cairo::Context,
    width: i32,
    height: i32,
    stroke: &[(f32, f32)],
) {
    if width <= 0 || height <= 0 || stroke.is_empty() {
        return;
    }
    let color = area.style_context().color();
    context.set_source_rgba(
        f64::from(color.red()),
        f64::from(color.green()),
        f64::from(color.blue()),
        f64::from(color.alpha()).max(0.85),
    );
    context.set_line_width(3.0);
    context.set_line_cap(gtk::cairo::LineCap::Round);
    context.set_line_join(gtk::cairo::LineJoin::Round);
    let scale_x = f64::from(width) / f64::from(recognize::CANVAS);
    let scale_y = f64::from(height) / f64::from(recognize::CANVAS);
    for (index, (x, y)) in stroke.iter().enumerate() {
        let x = f64::from(*x) * scale_x;
        let y = f64::from(*y) * scale_y;
        if index == 0 {
            context.move_to(x, y);
        } else {
            context.line_to(x, y);
        }
    }
    let _ = context.stroke();
}
