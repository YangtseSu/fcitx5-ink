// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later
mod editor;
mod recognition;

use editor::InkEditor;
use fcitx5_ink_core::{config, model, recognize, socket};
use gtk::prelude::*;
use gtk::{
    gdk, glib, Application, ApplicationWindow, Box as GtkBox, Button, DrawingArea, Entry,
    EventControllerKey, FlowBox, HeaderBar, Label, Orientation, SpinButton, Window,
};
use recognition::{Finish, RecognitionState, Start};
use std::cell::RefCell;
use std::collections::HashSet;
use std::env;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{self, TryRecvError};
use std::time::Duration;

#[derive(Default)]
struct Ink {
    editor: InkEditor,
    current: Vec<(f32, f32)>,
    drag_start: Option<(f64, f64)>,
    candidates: Vec<String>,
    recognition: RecognitionState,
    committing: bool,
}

struct Paths {
    socket: PathBuf,
    model: Option<PathBuf>,
}

#[derive(Clone)]
struct Ui {
    window: ApplicationWindow,
    canvas: DrawingArea,
    candidates: FlowBox,
    status: Label,
    copy_button: Button,
    undo_button: Button,
    redo_button: Button,
    delete_button: Button,
    clear_button: Button,
    ink: Rc<RefCell<Ink>>,
    paths: Rc<Paths>,
    settings: Rc<RefCell<config::Config>>,
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
    let paths = Rc::new(Paths {
        socket: args.socket,
        model,
    });
    let settings = config::load();
    let app = Application::builder()
        .application_id("app.fcitx5.Ink")
        .build();
    app.connect_activate(move |app| build_ui(app, paths.clone(), settings.clone()));
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

fn build_ui(app: &Application, paths: Rc<Paths>, settings: config::Config) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("墨笔手写")
        .default_width(620)
        .default_height(560)
        .build();

    let header = HeaderBar::new();
    let settings_button = Button::with_label("设置");
    header.pack_start(&settings_button);
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

    let actions = GtkBox::new(Orientation::Horizontal, 6);
    let undo_button = Button::with_label("撤销");
    let redo_button = Button::with_label("重做");
    let delete_button = Button::with_label("删末笔");
    let clear_button = Button::with_label("清空");
    undo_button.set_tooltip_text(Some("撤销上一笔（Ctrl+Z）"));
    redo_button.set_tooltip_text(Some("重做（Ctrl+Shift+Z 或 Ctrl+Y）"));
    delete_button.set_tooltip_text(Some("删除最后一笔且不能重做（BackSpace）"));
    clear_button.set_tooltip_text(Some("清空全部笔迹（Delete）"));
    actions.append(&undo_button);
    actions.append(&redo_button);
    actions.append(&delete_button);
    actions.append(&clear_button);
    root.append(&actions);

    let candidate_box = FlowBox::new();
    candidate_box.set_selection_mode(gtk::SelectionMode::None);
    candidate_box.set_max_children_per_line(6);
    candidate_box.set_column_spacing(6);
    candidate_box.set_row_spacing(6);
    candidate_box.set_hexpand(true);
    root.append(&candidate_box);

    let status = Label::new(Some("写一个字，松开后识别"));
    status.set_xalign(0.0);
    root.append(&status);
    window.set_child(Some(&root));

    let ui = Ui {
        window: window.clone(),
        canvas: canvas.clone(),
        candidates: candidate_box,
        status,
        copy_button: copy_button.clone(),
        undo_button,
        redo_button,
        delete_button,
        clear_button,
        ink: Rc::new(RefCell::new(Ink::default())),
        paths,
        settings: Rc::new(RefCell::new(settings)),
    };

    let draw_ui = ui.clone();
    canvas.set_draw_func(move |area, context, width, height| {
        let ink = draw_ui.ink.borrow();
        let width_px = f64::from(draw_ui.settings.borrow().panel.stroke_width);
        draw_strokes(area, context, width, height, ink.editor.strokes(), width_px);
        if !ink.current.is_empty() {
            draw_one_stroke(area, context, width, height, &ink.current, width_px);
        }
    });

    let drag = gtk::GestureDrag::new();
    drag.set_button(0);
    let drag_begin_ui = ui.clone();
    drag.connect_drag_begin(move |_, start_x, start_y| {
        {
            let mut ink = drag_begin_ui.ink.borrow_mut();
            if ink.committing {
                return;
            }
            ink.recognition.cancel();
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
        }
        drag_begin_ui.set_status("书写中，松开后自动识别");
        drag_begin_ui.canvas.queue_draw();
    });
    let drag_update_ui = ui.clone();
    drag.connect_drag_update(move |_, offset_x, offset_y| {
        let mut ink = drag_update_ui.ink.borrow_mut();
        if ink.committing {
            return;
        }
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
        drop(ink);
        drag_update_ui.canvas.queue_draw();
    });
    let drag_end_ui = ui.clone();
    drag.connect_drag_end(move |_, _offset_x, _offset_y| {
        let added = {
            let mut ink = drag_end_ui.ink.borrow_mut();
            if ink.committing {
                return;
            }
            ink.drag_start = None;
            if ink.current.len() >= 2 {
                let stroke = std::mem::take(&mut ink.current);
                ink.editor.push_stroke(stroke)
            } else {
                ink.current.clear();
                false
            }
        };
        drag_end_ui.canvas.queue_draw();
        if added {
            drag_end_ui.refresh_recognition();
        } else {
            drag_end_ui.restore_recognition_status();
        }
    });
    canvas.add_controller(drag);

    let copy_ui = ui.clone();
    copy_button.connect_clicked(move |_| copy_ui.copy_first());
    let settings_ui = ui.clone();
    settings_button.connect_clicked(move |_| settings_ui.open_settings());
    for (button, action) in [
        (&ui.undo_button, Ui::undo as fn(&Ui)),
        (&ui.redo_button, Ui::redo as fn(&Ui)),
        (&ui.delete_button, Ui::delete_last as fn(&Ui)),
        (&ui.clear_button, Ui::clear_all as fn(&Ui)),
    ] {
        let ui = ui.clone();
        button.connect_clicked(move |_| action(&ui));
    }

    let key_controller = EventControllerKey::new();
    let key_ui = ui.clone();
    key_controller.connect_key_pressed(move |_, key, _, modifiers| {
        use gdk::ModifierType;
        if modifiers.contains(ModifierType::ALT_MASK) {
            return glib::Propagation::Proceed;
        }
        let control = modifiers.contains(ModifierType::CONTROL_MASK);
        let shift = modifiers.contains(ModifierType::SHIFT_MASK);
        match key {
            gdk::Key::Escape => {
                key_ui.window.close();
                glib::Propagation::Stop
            }
            gdk::Key::BackSpace if !control && !shift => {
                key_ui.delete_last();
                glib::Propagation::Stop
            }
            gdk::Key::Delete if !control && !shift => {
                key_ui.clear_all();
                glib::Propagation::Stop
            }
            gdk::Key::z | gdk::Key::Z if control => {
                if shift {
                    key_ui.redo();
                } else {
                    key_ui.undo();
                }
                glib::Propagation::Stop
            }
            gdk::Key::y | gdk::Key::Y if control => {
                key_ui.redo();
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        }
    });
    window.add_controller(key_controller);

    ui.sync_actions();
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

    fn copy_first(&self) {
        let first = self.ink.borrow().candidates.first().cloned();
        match first {
            Some(candidate) => {
                self.copy_text(&candidate);
                self.set_status("已复制到剪贴板");
            }
            None => self.set_status("还没有候选"),
        }
    }

    fn clear_candidates(&self) {
        while let Some(child) = self.candidates.first_child() {
            self.candidates.remove(&child);
        }
        self.ink.borrow_mut().candidates.clear();
    }

    fn apply_candidates(&self, list: &[String]) {
        let mut seen = HashSet::new();
        let normalized: Vec<String> = list
            .iter()
            .filter(|candidate| !candidate.is_empty() && seen.insert((*candidate).clone()))
            .cloned()
            .collect();
        self.ink.borrow_mut().candidates = normalized;
        self.render_candidates();
        if self.ink.borrow().candidates.is_empty() {
            self.set_status("未识别到内容，重写一次");
        } else {
            self.set_status("点击候选输入，或点“复制首选”");
        }
        self.sync_actions();
    }

    fn render_candidates(&self) {
        while let Some(child) = self.candidates.first_child() {
            self.candidates.remove(&child);
        }
        let candidates = self.ink.borrow().candidates.clone();
        let limit = self.settings.borrow().panel.max_candidates;
        for candidate in candidates.into_iter().take(limit) {
            let button = Button::with_label(&candidate);
            button.set_tooltip_text(Some(&format!("输入 {candidate}")));
            let ui = self.clone();
            button.connect_clicked(move |_| ui.commit(candidate.clone()));
            self.candidates.insert(&button, -1);
        }
    }

    fn sync_actions(&self) {
        let ink = self.ink.borrow();
        let busy = ink.committing;
        self.undo_button
            .set_sensitive(ink.editor.can_undo() && !busy);
        self.redo_button
            .set_sensitive(ink.editor.can_redo() && !busy);
        self.delete_button
            .set_sensitive(ink.editor.can_delete_last() && !busy);
        self.clear_button
            .set_sensitive(ink.editor.can_clear() && !busy);
        self.copy_button
            .set_sensitive(!ink.candidates.is_empty() && !busy);
        self.canvas.set_sensitive(!busy);
    }

    fn restore_recognition_status(&self) {
        if self.ink.borrow().candidates.is_empty() {
            if self.ink.borrow().editor.strokes().is_empty() {
                self.set_status("写一个字，松开后识别");
            } else {
                self.refresh_recognition();
            }
        } else {
            self.set_status("点击候选输入，或点“复制首选”");
        }
    }

    fn refresh_recognition(&self) {
        if self.ink.borrow().committing {
            return;
        }
        let strokes = self.ink.borrow().editor.strokes().to_vec();
        self.clear_candidates();
        if strokes.is_empty() {
            self.ink.borrow_mut().recognition.cancel();
            self.set_status("写一个字，松开后识别");
            self.sync_actions();
            return;
        }
        let Some(model) = self.paths.model.clone() else {
            self.ink.borrow_mut().recognition.cancel();
            self.set_status("未找到 handwriting-zh_CN.model");
            self.sync_actions();
            return;
        };
        let start = self.ink.borrow_mut().recognition.request();
        match start {
            Start::Start(generation) => {
                self.set_status("识别中…");
                self.start_recognition(generation, strokes, model);
            }
            Start::AlreadyRunning => self.set_status("识别中…"),
        }
        self.sync_actions();
    }

    fn start_recognition(&self, generation: u64, strokes: Vec<Vec<(f32, f32)>>, model: PathBuf) {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(recognize::recognize(&model, &strokes));
        });
        let ui = self.clone();
        glib::timeout_add_local(Duration::from_millis(40), move || {
            match receiver.try_recv() {
                Ok(result) => {
                    ui.finish_recognition(generation, result);
                    glib::ControlFlow::Break
                }
                Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(TryRecvError::Disconnected) => {
                    ui.finish_recognition(generation, Err("识别线程已退出".to_owned()));
                    glib::ControlFlow::Break
                }
            }
        });
    }

    fn finish_recognition(&self, generation: u64, result: Result<Vec<String>, String>) {
        let finish = self.ink.borrow_mut().recognition.finish(generation);
        match finish {
            Finish::Apply => match result {
                Ok(candidates) => self.apply_candidates(&candidates),
                Err(error) => {
                    self.clear_candidates();
                    self.set_status(&format!("识别失败：{error}"));
                }
            },
            Finish::Restart(next) => {
                let strokes = self.ink.borrow().editor.strokes().to_vec();
                let Some(model) = self.paths.model.clone() else {
                    self.ink.borrow_mut().recognition.cancel();
                    self.set_status("未找到 handwriting-zh_CN.model");
                    return;
                };
                self.set_status("识别中…");
                self.start_recognition(next, strokes, model);
            }
            Finish::Ignore => {}
        }
        self.sync_actions();
    }

    fn undo(&self) {
        if self.ink.borrow().committing {
            return;
        }
        let changed = self.ink.borrow_mut().editor.undo();
        if !changed {
            return;
        }
        self.canvas.queue_draw();
        self.refresh_recognition();
    }

    fn redo(&self) {
        if self.ink.borrow().committing {
            return;
        }
        let changed = self.ink.borrow_mut().editor.redo();
        if !changed {
            return;
        }
        self.canvas.queue_draw();
        self.refresh_recognition();
    }

    fn delete_last(&self) {
        if self.ink.borrow().committing {
            return;
        }
        let changed = self.ink.borrow_mut().editor.delete_last();
        if !changed {
            return;
        }
        self.canvas.queue_draw();
        self.refresh_recognition();
    }

    fn clear_all(&self) {
        if self.ink.borrow().committing {
            return;
        }
        let changed = self.ink.borrow_mut().editor.clear();
        let had_candidates = !self.ink.borrow().candidates.is_empty();
        if !changed && !had_candidates {
            return;
        }
        self.ink.borrow_mut().recognition.cancel();
        self.ink.borrow_mut().current.clear();
        self.clear_candidates();
        self.canvas.queue_draw();
        self.set_status("写一个字，松开后识别");
        self.sync_actions();
    }

    fn commit(&self, text: String) {
        if self.ink.borrow().committing {
            return;
        }
        {
            let mut ink = self.ink.borrow_mut();
            ink.committing = true;
            ink.recognition.cancel();
        }
        self.sync_actions();
        self.set_status("输入中…");

        // The panel has focus while the user picks a candidate. Hide it first
        // so focus can return to the editor; the addon's broker then commits
        // the text to that context.
        self.window.set_visible(false);
        let socket = self.paths.socket.clone();
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
                    {
                        let mut ink = ui.ink.borrow_mut();
                        ink.committing = false;
                        ink.editor.clear();
                        ink.current.clear();
                    }
                    ui.clear_candidates();
                    ui.canvas.queue_draw();
                    ui.window.set_visible(true);
                    ui.set_status("已输入，继续写下一个字");
                    ui.sync_actions();
                    glib::ControlFlow::Break
                }
                Ok(Err(error)) => {
                    ui.ink.borrow_mut().committing = false;
                    ui.window.set_visible(true);
                    ui.copy_text(&text_for_copy);
                    ui.set_status(&format!("输入失败（{error}），已复制到剪贴板"));
                    ui.sync_actions();
                    glib::ControlFlow::Break
                }
                Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(TryRecvError::Disconnected) => {
                    ui.ink.borrow_mut().committing = false;
                    ui.window.set_visible(true);
                    ui.copy_text(&text_for_copy);
                    ui.set_status("输入线程已退出，已复制到剪贴板");
                    ui.sync_actions();
                    glib::ControlFlow::Break
                }
            }
        });
    }

    fn open_settings(&self) {
        let current = self.settings.borrow().clone();
        let window = Window::builder()
            .title("墨笔设置")
            .transient_for(&self.window)
            .modal(true)
            .default_width(400)
            .build();
        let header = HeaderBar::new();
        let cancel = Button::with_label("取消");
        let save = Button::with_label("保存");
        header.pack_start(&cancel);
        header.pack_end(&save);
        window.set_titlebar(Some(&header));

        let root = GtkBox::new(Orientation::Vertical, 10);
        root.set_margin_top(16);
        root.set_margin_bottom(16);
        root.set_margin_start(16);
        root.set_margin_end(16);

        let hotkey_entry = Entry::new();
        hotkey_entry.set_text(&current.hotkey);
        hotkey_entry.set_hexpand(true);
        root.append(&settings_row("快捷键", &hotkey_entry));
        let hotkey_help = Label::new(Some(
            "fcitx5 键名，例如 Control+Alt+H 或 Super+i；至少包含一个修饰键。",
        ));
        hotkey_help.set_xalign(0.0);
        hotkey_help.set_wrap(true);
        hotkey_help.add_css_class("dim-label");
        root.append(&hotkey_help);
        if env::var("FCITX5_INK_HOTKEY").is_ok_and(|value| !value.is_empty()) {
            let note = Label::new(Some("环境变量 FCITX5_INK_HOTKEY 覆盖了快捷键设置。"));
            note.set_xalign(0.0);
            note.set_wrap(true);
            root.append(&note);
        }

        let width = SpinButton::with_range(
            f64::from(config::MIN_STROKE_WIDTH),
            f64::from(config::MAX_STROKE_WIDTH),
            1.0,
        );
        width.set_value(f64::from(current.panel.stroke_width));
        root.append(&settings_row("笔画粗细", &width));

        let count = SpinButton::with_range(
            config::MIN_CANDIDATES as f64,
            config::MAX_CANDIDATES as f64,
            1.0,
        );
        count.set_value(current.panel.max_candidates as f64);
        root.append(&settings_row("候选数量", &count));

        let status = Label::new(None);
        status.set_xalign(0.0);
        status.set_wrap(true);
        root.append(&status);
        window.set_child(Some(&root));

        let cancel_window = window.clone();
        cancel.connect_clicked(move |_| cancel_window.close());

        let key_controller = EventControllerKey::new();
        let escape_window = window.clone();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                escape_window.close();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        window.add_controller(key_controller);

        let ui = self.clone();
        let settings_window = window.clone();
        save.connect_clicked(move |save_button| {
            let hotkey = hotkey_entry.text().trim().to_owned();
            if let Err(error) = config::validate_hotkey(&hotkey) {
                status.set_text(&error);
                return;
            }
            let mut next = ui.settings.borrow().clone();
            next.hotkey = hotkey;
            next.panel.stroke_width = width.value().round().clamp(
                f64::from(config::MIN_STROKE_WIDTH),
                f64::from(config::MAX_STROKE_WIDTH),
            ) as u8;
            next.panel.max_candidates = count
                .value_as_int()
                .clamp(config::MIN_CANDIDATES as i32, config::MAX_CANDIDATES as i32)
                as usize;
            if let Err(error) = next.validate() {
                status.set_text(&error);
                return;
            }
            let Some(path) = config::config_path() else {
                status.set_text("无法确定配置目录（XDG_CONFIG_HOME 或 HOME）");
                return;
            };
            save_button.set_sensitive(false);
            status.set_text("保存中…");

            let socket_path = ui.paths.socket.clone();
            let saved = next.clone();
            let (sender, receiver) = mpsc::channel();
            std::thread::spawn(move || {
                let outcome = save_settings(&path, &socket_path, &saved);
                let _ = sender.send(outcome);
            });

            let ui = ui.clone();
            let settings_window = settings_window.clone();
            let save_button = save_button.clone();
            let status = status.clone();
            glib::timeout_add_local(Duration::from_millis(40), move || {
                match receiver.try_recv() {
                    Ok(SaveOutcome::Saved { warning }) => {
                        *ui.settings.borrow_mut() = next.clone();
                        ui.canvas.queue_draw();
                        ui.render_candidates();
                        match warning {
                            Some(warning) => ui.set_status(&warning),
                            None => ui.set_status("设置已保存"),
                        }
                        settings_window.close();
                        glib::ControlFlow::Break
                    }
                    Ok(SaveOutcome::Rejected(message) | SaveOutcome::Failed(message)) => {
                        status.set_text(&message);
                        save_button.set_sensitive(true);
                        glib::ControlFlow::Break
                    }
                    Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(TryRecvError::Disconnected) => {
                        status.set_text("保存线程已退出");
                        save_button.set_sensitive(true);
                        glib::ControlFlow::Break
                    }
                }
            });
        });

        window.present();
    }
}

enum SaveOutcome {
    Saved { warning: Option<String> },
    Rejected(String),
    Failed(String),
}

fn save_settings(
    path: &std::path::Path,
    socket_path: &std::path::Path,
    settings: &config::Config,
) -> SaveOutcome {
    if socket_path.exists() {
        match socket::validate_hotkey(socket_path, &settings.hotkey) {
            Ok(()) => {}
            Err(error) if error == "invalid_hotkey" || error == "invalid" => {
                return SaveOutcome::Rejected("fcitx5 不接受这个快捷键".to_owned());
            }
            Err(_) => {}
        }
    }
    if let Err(error) = config::save_to(path, settings) {
        return SaveOutcome::Failed(format!("保存失败：{error}"));
    }
    if socket_path.exists() {
        match socket::reload(socket_path) {
            Ok(_) => SaveOutcome::Saved { warning: None },
            Err(error) => SaveOutcome::Saved {
                warning: Some(format!(
                    "已保存，但重新加载失败（{error}）；重启 fcitx5 后生效"
                )),
            },
        }
    } else {
        SaveOutcome::Saved {
            warning: Some("已保存；fcitx5 未运行，重启后生效".to_owned()),
        }
    }
}

fn settings_row<W: IsA<gtk::Widget>>(title: &str, widget: &W) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 12);
    let label = Label::new(Some(title));
    label.set_xalign(0.0);
    label.set_width_chars(10);
    row.append(&label);
    row.append(widget);
    row
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
    line_width: f64,
) {
    for stroke in strokes {
        draw_one_stroke(area, context, width, height, stroke, line_width);
    }
}

fn draw_one_stroke(
    area: &DrawingArea,
    context: &gtk::cairo::Context,
    width: i32,
    height: i32,
    stroke: &[(f32, f32)],
    line_width: f64,
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
    context.set_line_width(line_width.max(1.0));
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
