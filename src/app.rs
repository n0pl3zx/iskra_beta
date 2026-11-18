//! Интерфейс «Искры»: вход, список чатов, переписка, отправка сообщений.

use crate::client::{Auth, ConnStatus, NetClient, NetEvent};
use crate::i18n;
use crate::protocol::*;
use crate::theme::*;
use crate::timefmt;
use egui::{
    Align, Align2, CentralPanel, Color32, Context, CursorIcon, FontFamily, FontId, Frame, Id, Key,
    Layout, Margin, Painter, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, TextEdit, Ui, Vec2,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Duration;

pub fn run(hosting: bool) {
    // Язык нужен ещё до создания окна: от него зависит заголовок.
    let cfg = load_config();
    let lang = cfg
        .language
        .as_deref()
        .and_then(crate::i18n::Lang::from_id)
        .unwrap_or(crate::i18n::Lang::Ru);
    crate::i18n::set(lang);
    let app_name = crate::i18n::s().app_name;

    let size = Vec2::new(1180.0, 620.0);
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([size.x, size.y])
        .with_min_inner_size([900.0, 560.0])
        .with_title(app_name)
        .with_icon(app_icon());
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    let result = eframe::run_native(
        app_name,
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, hosting)) as Box<dyn eframe::App>)),
    );
    if let Err(err) = result {
        eprintln!("Failed to start window: {err}");
    }
}

fn app_icon() -> egui::IconData {
    let size = 64usize;
    let mut rgba = vec![0u8; size * size * 4];
    let c = 31.5f32;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - c;
            let dy = y as f32 - c;
            let dist = (dx * dx + dy * dy).sqrt();
            let idx = (y * size + x) * 4;
            if dist <= 30.0 {
                let spark = (dx.abs() / 3.4 + dy.abs() / 16.0 <= 1.0)
                    || (dy.abs() / 3.4 + dx.abs() / 16.0 <= 1.0);
                let (r, g, b) = if spark { (255, 255, 255) } else { (24, 24, 26) };
                rgba[idx] = r;
                rgba[idx + 1] = g;
                rgba[idx + 2] = b;
                rgba[idx + 3] = 255;
            }
        }
    }
    egui::IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Config {
    server: Option<String>,
    username: Option<String>,
    theme: Option<String>,
    scale: Option<f32>,
    animations: Option<bool>,
    language: Option<String>,
}

fn config_path() -> PathBuf {
    data_dir().join("client.json")
}

fn load_config() -> Config {
    std::fs::read_to_string(config_path())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save_config(cfg: &Config) {
    if let Some(dir) = config_path().parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string_pretty(cfg) {
        let _ = std::fs::write(config_path(), text);
    }
}

fn normalize_addr(input: &str) -> String {
    let addr = input.trim();
    if addr.is_empty() {
        return format!("127.0.0.1:{}", DEFAULT_PORT);
    }
    if addr.contains(':') {
        addr.to_string()
    } else {
        format!("{}:{}", addr, DEFAULT_PORT)
    }
}

const EMOJI_SHEET: &[u8] = include_bytes!("../assets/emoji_sheet.rgba");
const EMOJI_CELL: usize = 72;
const EMOJI_COLS: usize = 8;
const EMOJI_ROWS: usize = 6;

/// Загружает готовый лист цветных эмодзи как текстуру.
fn emoji_texture(ctx: &Context) -> egui::TextureHandle {
    let pixels = EMOJI_SHEET
        .chunks_exact(4)
        .map(|c| Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]))
        .collect();
    ctx.load_texture(
        "iskra_emoji_sheet",
        egui::ColorImage {
            size: [EMOJI_CELL * EMOJI_COLS, EMOJI_CELL * EMOJI_ROWS],
            pixels,
        },
        egui::TextureOptions::LINEAR,
    )
}

pub struct App {
    net: NetClient,
    hosting: bool,
    my_ip: String,
    cfg: Config,

    login_addr: String,
    login_username: String,
    login_password: String,
    login_name: String,
    register_mode: bool,
    login_error: Option<String>,
    connecting: bool,

    me: Option<User>,
    chats: Vec<ChatSummary>,
    selected: Option<u64>,
    messages: HashMap<u64, Vec<Message>>,
    history_loaded: HashSet<u64>,
    has_more: HashMap<u64, bool>,
    older_pending: bool,
    read_by: HashMap<u64, i64>,
    typing_until: HashMap<u64, f64>,
    row_seen: HashMap<u64, f64>,
    unread_total: u32,

    input: String,
    focus_input: bool,
    focus_search: bool,
    scroll_bottom: bool,
    search: String,
    show_emoji: bool,
    emoji_open_time: f64,
    emoji_tex: egui::TextureHandle,
    show_profile: bool,
    show_settings: bool,
    settings_open_time: f64,
    show_profile_time: f64,
    theme_id: String,
    scale: f32,
    animations: bool,
    status: ConnStatus,
    notice: Option<(String, f64)>,
    last_typing: f64,
    time: f64,
    window_title: String,
    positioned: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, hosting: bool) -> Self {
        crate::theme::setup(&cc.egui_ctx);
        let cfg = load_config();
        let theme_id = cfg
            .theme
            .clone()
            .filter(|id| crate::theme::theme_exists(id))
            .unwrap_or_else(|| "graphite".to_string());
        let scale = cfg.scale.unwrap_or(1.0).clamp(0.8, 1.4);
        let animations = cfg.animations.unwrap_or(true);
        let lang = cfg
            .language
            .as_deref()
            .and_then(crate::i18n::Lang::from_id)
            .unwrap_or(crate::i18n::Lang::Ru);
        crate::i18n::set(lang);
        crate::theme::apply(&cc.egui_ctx, &theme_id, false);
        crate::theme::set_animation(&cc.egui_ctx, animations);
        cc.egui_ctx.set_zoom_factor(scale);
        let emoji_tex = emoji_texture(&cc.egui_ctx);
        let server = cfg
            .server
            .clone()
            .unwrap_or_else(|| format!("127.0.0.1:{}", DEFAULT_PORT));
        let username = cfg.username.clone().unwrap_or_default();
        let net = NetClient::new(cc.egui_ctx.clone());
        let my_ip = crate::server::local_ip().unwrap_or_else(|| "127.0.0.1".to_string());
        let app = App {
            net,
            hosting,
            my_ip,
            cfg,
            login_addr: server,
            login_username: username,
            login_password: String::new(),
            login_name: String::new(),
            register_mode: false,
            login_error: None,
            connecting: false,
            me: None,
            chats: Vec::new(),
            selected: None,
            messages: HashMap::new(),
            history_loaded: HashSet::new(),
            has_more: HashMap::new(),
            older_pending: false,
            read_by: HashMap::new(),
            typing_until: HashMap::new(),
            row_seen: HashMap::new(),
            unread_total: 0,
            input: String::new(),
            focus_input: false,
            search: String::new(),
            focus_search: false,
            show_emoji: false,
            emoji_open_time: 0.0,
            emoji_tex,
            show_profile: false,
            show_settings: false,
            settings_open_time: 0.0,
            show_profile_time: 0.0,
            theme_id,
            scale,
            animations,
            status: ConnStatus::Offline,
            notice: None,
            scroll_bottom: true,
            last_typing: 0.0,
            time: 0.0,
            window_title: String::new(),
            positioned: false,
        };
        app
    }

    fn me_id(&self) -> u64 {
        self.me.as_ref().map(|u| u.id).unwrap_or(0)
    }

    fn is_typing(&self, user: u64) -> bool {
        self.typing_until
            .get(&user)
            .map(|until| *until > self.time)
            .unwrap_or(false)
    }

    fn any_typing(&self) -> bool {
        self.typing_until.values().any(|until| *until > self.time)
    }

    fn toast(&mut self, text: impl Into<String>) {
        self.notice = Some((text.into(), self.time));
    }

    fn set_title(&mut self, ctx: &Context) {
        let app_name = crate::i18n::s().app_name;
        let title = if self.me.is_some() && self.unread_total > 0 {
            format!(
                "{} · {} {}",
                app_name,
                self.unread_total,
                crate::i18n::s().unread_suffix
            )
        } else {
            app_name.to_string()
        };
        if title != self.window_title {
            self.window_title = title.clone();
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
        }
    }

    // ---------------------------------------------------------------- сеть

    fn handle_net(&mut self) {
        for event in self.net.poll() {
            match event {
                NetEvent::Status(status) => {
                    self.status = status;
                    if status != ConnStatus::Connecting {
                        self.connecting = false;
                    }
                }
                NetEvent::Notice(text) => {
                    if self.me.is_none() {
                        self.login_error = Some(text.clone());
                        self.connecting = false;
                    }
                    self.toast(text);
                }
                NetEvent::AuthFailed(text) => {
                    self.connecting = false;
                    self.status = ConnStatus::Offline;
                    self.login_error = Some(crate::i18n::server_text(&text));
                }
                NetEvent::Server(msg) => self.on_server(msg),
            }
        }
    }

    fn on_server(&mut self, msg: ServerMsg) {
        match msg {
            ServerMsg::AuthOk { me, chats } => {
                self.login_password.clear();
                self.login_error = None;
                self.messages.clear();
                self.history_loaded.clear();
                self.has_more.clear();
                self.read_by.clear();
                self.typing_until.clear();
                self.me = Some(me);
                self.chats = chats;
                self.unread_total = self.chats.iter().map(|c| c.unread).sum();
                let target = self
                    .chats
                    .iter()
                    .find(|c| c.last_ts > 0)
                    .map(|c| c.user.id)
                    .or_else(|| self.chats.first().map(|c| c.user.id));
                self.selected = None;
                if let Some(id) = target {
                    self.open_chat(id);
                }
            }
            ServerMsg::Chats { chats } => {
                self.chats = chats;
                self.unread_total = self.chats.iter().map(|c| c.unread).sum();
                if let Some(sel) = self.selected {
                    if !self.chats.iter().any(|c| c.user.id == sel) {
                        self.selected = None;
                    }
                }
            }
            ServerMsg::Message { msg } => {
                let mine = msg.from == self.me_id();
                let peer = if mine { msg.to } else { msg.from };
                if peer == 0 {
                    return;
                }
                if let Some(list) = self.messages.get_mut(&peer) {
                    if !list.iter().any(|m| m.id == msg.id) {
                        list.push(msg.clone());
                    }
                }
                if self.selected == Some(peer) && !mine {
                    self.send_mark_read(peer);
                }
                if self.selected == Some(peer) {
                    self.scroll_bottom = true;
                }
            }
            ServerMsg::History {
                with,
                messages,
                has_more,
            } => {
                let my_id = self.me_id();
                let mut peer_read = 0;
                for m in &messages {
                    if m.from == my_id && m.read {
                        peer_read = peer_read.max(m.ts);
                    }
                }
                if peer_read > 0 {
                    let entry = self.read_by.entry(with).or_insert(0);
                    *entry = (*entry).max(peer_read);
                }
                if self.older_pending {
                    self.older_pending = false;
                    let list = self.messages.entry(with).or_default();
                    let mut merged = messages;
                    merged.extend(list.drain(..));
                    merged.sort_by_key(|m| m.id);
                    merged.dedup_by_key(|m| m.id);
                    *list = merged;
                } else {
                    self.messages.insert(with, messages);
                }
                self.history_loaded.insert(with);
                self.has_more.insert(with, has_more);
                if self.selected == Some(with) {
                    self.scroll_bottom = true;
                }
            }
            ServerMsg::Typing { from } => {
                self.typing_until.insert(from, self.time + 3.0);
            }
            ServerMsg::Read { by, up_to } => {
                let my_id = self.me_id();
                let entry = self.read_by.entry(by).or_insert(0);
                *entry = (*entry).max(up_to);
                if let Some(list) = self.messages.get_mut(&by) {
                    for m in list.iter_mut() {
                        if m.from == my_id && m.ts <= up_to {
                            m.read = true;
                        }
                    }
                }
            }
            ServerMsg::Presence {
                user_id,
                online,
                last_seen,
            } => {
                if let Some(chat) = self.chats.iter_mut().find(|c| c.user.id == user_id) {
                    chat.online = online;
                    chat.user.last_seen = last_seen;
                }
                if !online {
                    self.typing_until.remove(&user_id);
                }
            }
            ServerMsg::Error { message, fatal } => {
                let message = crate::i18n::server_text(&message);
                if self.me.is_none() {
                    self.login_error = Some(message);
                    self.connecting = false;
                } else if fatal {
                    self.toast(message);
                } else {
                    self.toast(message);
                }
            }
        }
    }

    fn open_chat(&mut self, id: u64) {
        self.selected = Some(id);
        self.show_emoji = false;
        self.focus_input = true;
        self.scroll_bottom = true;
        if !self.history_loaded.contains(&id) {
            self.net.send(ClientMsg::History {
                with: id,
                before_id: None,
                limit: 300,
            });
        }
        self.send_mark_read(id);
    }

    fn send_mark_read(&mut self, peer: u64) {
        if self.status == ConnStatus::Online {
            self.net.send(ClientMsg::MarkRead { with: peer });
        }
        if let Some(chat) = self.chats.iter_mut().find(|c| c.user.id == peer) {
            chat.unread = 0;
        }
        self.unread_total = self.chats.iter().map(|c| c.unread).sum();
    }

    fn send_message(&mut self) {
        let Some(peer) = self.selected else { return };
        let text = self.input.trim().to_string();
        if text.is_empty() {
            return;
        }
        if self.status != ConnStatus::Online {
            self.toast(i18n::s().no_connection);
            return;
        }
        self.net.send(ClientMsg::SendMessage { to: peer, text });
        self.input.clear();
        self.last_typing = 0.0;
        self.focus_input = true;
        self.scroll_bottom = true;
    }

    fn submit_login(&mut self) {
        if self.connecting {
            return;
        }
        let username = self.login_username.trim().to_lowercase();
        let password = self.login_password.clone();
        let name = self.login_name.trim().to_string();
        if username.is_empty() {
            self.login_error = Some(i18n::s().err_username_required.into());
            return;
        }
        if password.is_empty() {
            self.login_error = Some(i18n::s().err_password_required.into());
            return;
        }
        if self.register_mode {
            if !(3..=24).contains(&username.chars().count())
                || !username
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
            {
                self.login_error =
                    Some(i18n::s().err_username_format.into());
                return;
            }
            if password.chars().count() < 4 {
                self.login_error = Some(i18n::s().err_password_short.into());
                return;
            }
        }
        let addr = normalize_addr(&self.login_addr);
        self.login_addr = addr.clone();
        let auth = if self.register_mode {
            Auth::Register {
                username: username.clone(),
                name,
                password,
            }
        } else {
            Auth::Login {
                username: username.clone(),
                password,
            }
        };
        self.net.connect(addr.clone(), auth);
        self.connecting = true;
        self.login_error = None;
        self.cfg.server = Some(addr);
        self.cfg.username = Some(username);
        save_config(&self.cfg);
    }

    fn logout(&mut self) {
        self.net.disconnect();
        self.me = None;
        self.chats.clear();
        self.messages.clear();
        self.history_loaded.clear();
        self.has_more.clear();
        self.selected = None;
        self.input.clear();
        self.unread_total = 0;
        self.show_profile = false;
        self.show_emoji = false;
        self.status = ConnStatus::Offline;
        self.login_password.clear();
    }

    // -------------------------------------------------------------- экраны

    fn login_ui(&mut self, ctx: &Context) {
        let hosting = self.hosting;
        let my_ip = self.my_ip.clone();
        let port = DEFAULT_PORT;
        let status = self.status;
        let mut submit = false;

        CentralPanel::default()
            .frame(Frame::none().fill(pal().bg))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let painter = ui.painter();
                painter.circle_filled(
                    egui::pos2(
                        rect.left() + rect.width() * 0.15,
                        rect.top() + rect.height() * 0.08,
                    ),
                    rect.width() * 0.30,
                    glow(),
                );
                painter.circle_filled(
                    egui::pos2(
                        rect.right() - rect.width() * 0.08,
                        rect.bottom() - rect.height() * 0.02,
                    ),
                    rect.width() * 0.24,
                    glow_soft(),
                );

                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let top = ((ui.available_height() - 575.0) * 0.5).max(6.0);
                        ui.add_space(top);
                        ui.vertical_centered(|ui| {
                            Frame::none()
                                .fill(pal().panel)
                                .rounding(22.0)
                                .stroke(Stroke::new(1.0_f32, pal().line))
                                .shadow(egui::epaint::Shadow {
                                    offset: Vec2::new(0.0, 18.0),
                                    blur: 44.0,
                                    spread: 0.0,
                                    color: Color32::from_black_alpha(120),
                                })
                                .inner_margin(Margin::same(26.0))
                                .show(ui, |ui| {
                                    ui.set_width(352.0);
                                    ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                                    ui.vertical_centered(|ui| {
                                        let (logo_rect, _) = ui.allocate_exact_size(
                                            Vec2::new(64.0, 64.0),
                                            Sense::hover(),
                                        );
                                        ui.painter()
                                            .circle_filled(logo_rect.center(), 32.0, pal().accent);
                                        spark(
                                            ui.painter(),
                                            logo_rect.center(),
                                            17.0,
                                            pal().on_accent,
                                        );
                                        ui.add_space(13.0);
                                        ui.label(
                                            RichText::new(i18n::s().app_name)
                                                .font(FontId::new(31.0, bold()))
                                                .color(pal().text),
                                        );
                                        ui.add_space(1.0);
                                        ui.label(
                                            RichText::new(i18n::s().tagline)
                                                .size(12.5)
                                                .color(pal().text_dim),
                                        );
                                    });
                                    ui.add_space(22.0);

                                    field(
                                        ui,
                                        i18n::s().label_server,
                                        &mut self.login_addr,
                                        "127.0.0.1:8787",
                                        false,
                                    );
                                    field(
                                        ui,
                                        i18n::s().label_username,
                                        &mut self.login_username,
                                        i18n::s().hint_user,
                                        false,
                                    );
                                    if self.register_mode {
                                        field(
                                            ui,
                                            i18n::s().label_your_name,
                                            &mut self.login_name,
                                            i18n::s().hint_name,
                                            false,
                                        );
                                    }
                                    field(
                                        ui,
                                        i18n::s().label_password,
                                        &mut self.login_password,
                                        i18n::s().hint_password,
                                        true,
                                    );

                                    if let Some(err) = &self.login_error {
                                        ui.add_space(2.0);
                                        ui.label(
                                            RichText::new(err).size(12.5).color(pal().danger),
                                        );
                                    }
                                    ui.add_space(6.0);

                                    let label = if self.connecting {
                                        i18n::s().connecting
                                    } else if self.register_mode {
                                        i18n::s().create_account
                                    } else {
                                        i18n::s().sign_in
                                    };
                                    if primary_button(ui, label, 46.0, !self.connecting).clicked()
                                    {
                                        submit = true;
                                    }

                                    ui.add_space(12.0);
                                    ui.vertical_centered(|ui| {
                                        let prompt = if self.register_mode {
                                            i18n::s().have_account
                                        } else {
                                            i18n::s().no_account
                                        };
                                        let prompt_font =
                                            FontId::new(12.5, FontFamily::Proportional);
                                        let link_font = FontId::new(13.0, bold());
                                        let prompt_w = ui
                                            .painter()
                                            .layout_no_wrap(
                                                prompt.to_owned(),
                                                prompt_font.clone(),
                                                pal().text_dim,
                                            )
                                            .size()
                                            .x;
                                        let link_w = ui
                                            .painter()
                                            .layout_no_wrap(
                                                i18n::s().switch_mode.to_owned(),
                                                link_font.clone(),
                                                pal().accent,
                                            )
                                            .size()
                                            .x;
                                        let gap = 7.0;
                                        let (row, resp) = ui.allocate_exact_size(
                                            Vec2::new(prompt_w + gap + link_w, 22.0),
                                            Sense::click(),
                                        );
                                        let painter = ui.painter();
                                        painter.text(
                                            egui::pos2(row.left(), row.center().y),
                                            Align2::LEFT_CENTER,
                                            prompt,
                                            prompt_font,
                                            pal().text_dim,
                                        );
                                        painter.text(
                                            egui::pos2(row.left() + prompt_w + gap, row.center().y),
                                            Align2::LEFT_CENTER,
                                            i18n::s().switch_mode,
                                            link_font,
                                            if resp.hovered() {
                                                pal().accent_hover
                                            } else {
                                                pal().accent
                                            },
                                        );
                                        if resp
                                            .on_hover_cursor(CursorIcon::PointingHand)
                                            .clicked()
                                        {
                                            self.register_mode = !self.register_mode;
                                            self.login_error = None;
                                        }
                                    });
                                    });
                                });
                        });

                        ui.add_space(10.0);
                        ui.vertical_centered(|ui| {
                            if hosting {
                                status_pill(
                                    ui,
                                    pal().online,
                                    &format!("{} · {}:{}", i18n::s().server_running, my_ip, port),
                                );
                                ui.add_space(5.0);
                                ui.label(
                                    RichText::new(
                                        i18n::s().friends_connect,
                                    )
                                    .size(11.5)
                                    .color(pal().text_muted),
                                );
                            } else if status == ConnStatus::Offline {
                                status_pill(ui, pal().warn, i18n::s().waiting);
                            } else {
                                status_pill(ui, pal().online, i18n::s().connected);
                            }
                        });
                        ui.add_space(26.0);
                    });

                if ui.input(|i| i.key_pressed(Key::Enter)) {
                    submit = true;
                }

                // Настройки доступны и до входа: тему можно выбрать сразу.
                let gear_center = egui::pos2(rect.right() - 38.0, rect.top() + 38.0);
                let gear_rect =
                    Rect::from_center_size(gear_center, Vec2::new(40.0, 40.0));
                let gear_resp = ui
                    .interact(gear_rect, Id::new("login_settings"), Sense::click())
                    .on_hover_cursor(CursorIcon::PointingHand);
                if gear_resp.hovered() {
                    ui.painter().circle_filled(gear_center, 18.0, pal().hover);
                }
                gear_icon(
                    ui.painter(),
                    gear_center,
                    9.0,
                    if gear_resp.hovered() {
                        pal().text
                    } else {
                        pal().text_muted
                    },
                );
                if gear_resp.clicked() {
                    self.show_settings = true;
                }
            });

        if submit {
            self.submit_login();
        }
    }

    fn main_ui(&mut self, ctx: &Context) {
        egui::SidePanel::left("rail")
            .exact_width(56.0)
            .resizable(false)
            .frame(Frame::none().fill(pal().panel))
            .show(ctx, |ui| self.nav_rail(ui));

        egui::SidePanel::left("sidebar")
            .exact_width(346.0)
            .resizable(false)
            .frame(Frame::none().fill(pal().sidebar))
            .show(ctx, |ui| self.sidebar(ui));

        if self.selected.is_some() {
            egui::TopBottomPanel::bottom("composer")
                .exact_height(82.0)
                .frame(Frame::none().fill(pal().bg))
                .show(ctx, |ui| self.composer(ui));
        }

        CentralPanel::default()
            .frame(Frame::none().fill(pal().bg))
            .show(ctx, |ui| {
                if self.selected.is_none() {
                    self.empty_state(ui);
                } else {
                    self.chat_panel(ui);
                }
            });

        self.emoji_popup(ctx);
        self.profile_window(ctx);
        self.show_toast(ctx);
    }

    fn nav_rail(&mut self, ui: &mut Ui) {
        let x = 28.0;
        // Меню открывает профиль; остальные значки дублировали бы это действие.
        if rail_button(ui, "rail_menu", egui::pos2(x, 30.0), false, menu_icon) {
            self.show_profile = !self.show_profile;
        }
        rail_button(ui, "rail_chats", egui::pos2(x, 90.0), true, spark);
        // Шестерёнка внизу — единственный вход в настройки.
        let bottom = (ui.max_rect().bottom() - 30.0).max(120.0);
        if rail_button(
            ui,
            "rail_settings",
            egui::pos2(x, bottom),
            self.show_settings,
            gear_icon,
        ) {
            self.show_settings = !self.show_settings;
        }
    }

    fn sidebar(&mut self, ui: &mut Ui) {
        ui.add_space(13.0);
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            let (mark, _) = ui.allocate_exact_size(Vec2::new(28.0, 28.0), Sense::hover());
            spark(ui.painter(), mark.center(), 11.0, pal().accent);
            ui.add_space(6.0);
            ui.label(
                RichText::new(i18n::s().app_name)
                    .font(FontId::new(20.0, bold()))
                    .color(pal().text),
            );
        });
        ui.add_space(11.0);

        self.search_box(ui);
        ui.add_space(6.0);

        let list_height = (ui.available_height() - 76.0).max(80.0);
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(list_height)
            .show(ui, |ui| {
                let query = self.search.trim().to_lowercase();
                let mut open: Option<u64> = None;
                let visible: Vec<ChatSummary> = self
                    .chats
                    .iter()
                    .filter(|chat| {
                        query.is_empty()
                            || chat.user.name.to_lowercase().contains(&query)
                            || chat.user.username.to_lowercase().contains(&query)
                    })
                    .cloned()
                    .collect();
                let now = self.time;
                for (idx, chat) in visible.iter().enumerate() {
                    // Новые строки проявляются и чуть выезжают слева, каскадом.
                    let seen = *self
                        .row_seen
                        .entry(chat.user.id)
                        .or_insert(now + (idx.min(12) as f64) * 0.018);
                    let appear = if self.animations {
                        let a = ease_out(((now - seen) / 0.22) as f32);
                        if a < 1.0 {
                            ui.ctx().request_repaint();
                        }
                        a
                    } else {
                        1.0
                    };
                    if chat_row(
                        ui,
                        chat,
                        self.selected == Some(chat.user.id),
                        self.me_id(),
                        self.is_typing(chat.user.id),
                        appear,
                    ) {
                        open = Some(chat.user.id);
                    }
                }
                if let Some(id) = open {
                    self.open_chat(id);
                }
                if self.chats.is_empty() {
                    ui.add_space(24.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new(i18n::s().no_contacts)
                                .size(13.0)
                                .color(pal().text_muted),
                        );
                    });
                }
                ui.add_space(8.0);
            });

        self.user_card(ui);
    }

    fn search_box(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.add_space(14.0);
            let width = (ui.available_width() - 14.0).max(80.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 38.0), Sense::hover());
            ui.painter().rect_filled(rect, 12.0, pal().field);
            search_icon(
                ui.painter(),
                egui::pos2(rect.left() + 20.0, rect.center().y),
                7.0,
                pal().text_muted,
            );
            let text_rect = Rect::from_center_size(
                egui::pos2(rect.center().x + 14.0, rect.center().y),
                Vec2::new(width - 52.0, 22.0),
            );
            let search_resp = ui.put(
                text_rect,
                TextEdit::singleline(&mut self.search)
                    .frame(false)
                    .hint_text(RichText::new(i18n::s().search).size(14.0).color(pal().text_muted))
                    .desired_width(f32::INFINITY)
                    .text_color(pal().text)
                    .font(FontId::new(14.0, FontFamily::Proportional))
                    .margin(Margin::ZERO),
            );
            if self.focus_search {
                search_resp.request_focus();
                self.focus_search = false;
            }
        });
    }

    fn user_card(&mut self, ui: &mut Ui) {
        ui.add_space(4.0);
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), 66.0),
            Sense::hover(),
        );
        let painter = ui.painter();
        painter.line_segment(
            [
                egui::pos2(rect.left() + 14.0, rect.top()),
                egui::pos2(rect.right() - 14.0, rect.top()),
            ],
            Stroke::new(1.0_f32, pal().line),
        );
        if let Some(me) = self.me.clone() {
            let center = egui::pos2(rect.left() + 40.0, rect.center().y);
            avatar(painter, center, 20.0, &me.name, me.color, None);
            let name = elide(
                ui,
                &me.name,
                FontId::new(14.5, bold()),
                rect.width() - 130.0,
            );
            painter.text(
                egui::pos2(center.x + 28.0, rect.center().y - 9.0),
                Align2::LEFT_CENTER,
                name,
                FontId::new(14.5, bold()),
                pal().text,
            );
            let (status_text, status_color) = match self.status {
                ConnStatus::Online => (i18n::s().online.to_string(), pal().online),
                ConnStatus::Connecting => (i18n::s().connecting_lower.to_string(), pal().warn),
                ConnStatus::Offline => (i18n::s().offline.to_string(), pal().danger),
            };
            painter.text(
                egui::pos2(center.x + 28.0, rect.center().y + 9.0),
                Align2::LEFT_CENTER,
                status_text,
                FontId::new(11.5, FontFamily::Proportional),
                status_color,
            );
        }
    }

    fn empty_state(&mut self, ui: &mut Ui) {
        let rect = ui.available_rect_before_wrap();
        let painter = ui.painter();
        let center = rect.center() - Vec2::new(0.0, 24.0);
        painter.circle_filled(center, 46.0, pal().panel);
        spark(painter, center, 17.0, pal().accent);
        painter.text(
            center + Vec2::new(0.0, 74.0),
            Align2::CENTER_CENTER,
            i18n::s().choose_chat,
            FontId::new(17.0, bold()),
            pal().text,
        );
        painter.text(
            center + Vec2::new(0.0, 98.0),
            Align2::CENTER_CENTER,
            i18n::s().contacts_hint,
            FontId::new(13.0, FontFamily::Proportional),
            pal().text_muted,
        );
    }

    fn chat_panel(&mut self, ui: &mut Ui) {
        let Some(peer_id) = self.selected else { return };
        let Some(chat) = self.chats.iter().find(|c| c.user.id == peer_id).cloned() else {
            return;
        };
        let typing = self.is_typing(peer_id);

        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 66.0), Sense::hover());
        let painter = ui.painter();
        let avatar_center = egui::pos2(rect.left() + 40.0, rect.center().y);
        avatar(
            painter,
            avatar_center,
            21.0,
            &chat.user.name,
            chat.user.color,
            Some(chat.online),
        );
        let name = elide(
            ui,
            &chat.user.name,
            FontId::new(16.0, bold()),
            width - 80.0,
        );
        painter.text(
            egui::pos2(avatar_center.x + 32.0, rect.center().y - 9.0),
            Align2::LEFT_CENTER,
            name,
            FontId::new(16.0, bold()),
            pal().text,
        );
        let (status_text, status_color) = if typing {
            (i18n::s().typing.to_string(), pal().accent)
        } else if self.status != ConnStatus::Online {
            (i18n::s().offline.to_string(), pal().danger)
        } else if chat.online {
            (i18n::s().online.to_string(), pal().online)
        } else {
            (timefmt::last_seen(chat.user.last_seen), pal().text_muted)
        };
        painter.text(
            egui::pos2(avatar_center.x + 32.0, rect.center().y + 9.0),
            Align2::LEFT_CENTER,
            status_text,
            FontId::new(12.5, FontFamily::Proportional),
            status_color,
        );

        // Единственная кнопка в шапке — поиск по чатам.
        let search_c = egui::pos2(rect.right() - 28.0, rect.center().y);
        search_icon(painter, search_c, 8.0, pal().text_dim);
        if header_button(ui, "chat_search", search_c) {
            self.focus_search = true;
        }

        ui.painter().line_segment(
            [
                egui::pos2(rect.left(), rect.bottom()),
                egui::pos2(rect.right(), rect.bottom()),
            ],
            Stroke::new(1.0_f32, pal().line),
        );

        let my_id = self.me_id();
        let read_up = self.read_by.get(&peer_id).copied().unwrap_or(0);
        let has_more = self.has_more.get(&peer_id).copied().unwrap_or(false);
        let mut load_older = false;
        let mut load_older_id = None;

        let out = ScrollArea::vertical()
            .id_salt(("chat", peer_id))
            .auto_shrink([false, false])
            .animated(false)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() - 16.0).max(120.0));
                        let msgs = self.messages.get(&peer_id).cloned().unwrap_or_default();
                        if msgs.is_empty() {
                            ui.add_space(40.0);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    RichText::new(i18n::s().empty_chat)
                                        .size(13.0)
                                        .color(pal().text_muted),
                                );
                                ui.label(
                                    RichText::new(
                                        i18n::s().write_first.replace("{}", &chat.user.username),
                                    )
                                    .size(12.0)
                                    .color(pal().text_muted),
                                );
                            });
                        } else {
                            if has_more {
                                let first_id = msgs.first().map(|m| m.id);
                                let mut clicked = false;
                                ui.vertical_centered(|ui| {
                                    if link_button(ui, i18n::s().show_older, pal().accent).clicked() {
                                        clicked = true;
                                    }
                                });
                                ui.add_space(6.0);
                                if clicked {
                                    load_older = true;
                                    load_older_id = first_id;
                                }
                            }
                            paint_messages(ui, &msgs, my_id, read_up);
                        }
                        ui.add_space(10.0);
                    });
                });
            });

        // Прокручиваем вниз сразу и без анимации, когда открываем чат
        // или получаем новое сообщение.
        if self.scroll_bottom {
            let max_offset = (out.content_size.y - out.inner_rect.height()).max(0.0);
            if out.state.offset.y < max_offset - 0.5 {
                let mut state = out.state;
                state.offset.y = max_offset;
                state.store(ui.ctx(), out.id);
                ui.ctx().request_repaint();
            } else {
                self.scroll_bottom = false;
            }
        }

        if load_older {
            if self.status == ConnStatus::Online {
                self.older_pending = true;
                self.net.send(ClientMsg::History {
                    with: peer_id,
                    before_id: load_older_id,
                    limit: 200,
                });
            }
        }
    }

    fn composer(&mut self, ui: &mut Ui) {
        ui.add_space(10.0);
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 56.0), Sense::hover());

        let mut toggle_emoji = false;
        let mut do_send = false;

        let emoji_c = egui::pos2(rect.right() - 68.0, rect.center().y);
        let send_c = egui::pos2(rect.right() - 26.0, rect.center().y);
        let input_rect = Rect::from_min_max(
            egui::pos2(rect.left() + 16.0, rect.top()),
            egui::pos2(rect.right() - 100.0, rect.bottom()),
        );

        {
            let painter = ui.painter();
            painter.rect_filled(input_rect, 18.0, pal().field);
            smile_icon(
                painter,
                emoji_c,
                9.5,
                if self.show_emoji { pal().accent } else { pal().text_dim },
            );
            let active = !self.input.trim().is_empty();
            painter.circle_filled(
                send_c,
                21.0,
                if active { pal().accent } else { pal().field },
            );
            plane(
                painter,
                send_c + Vec2::new(0.0, 0.0),
                17.0,
                if active {
                    pal().on_accent
                } else {
                    pal().text_muted
                },
            );
        }

        if header_button(ui, "emoji_toggle", emoji_c) {
            toggle_emoji = true;
        }

        let text_area = input_rect.shrink2(Vec2::new(18.0, 10.0));
        let input_response = ui.put(
            text_area,
            TextEdit::multiline(&mut self.input)
                .frame(false)
                .hint_text(RichText::new(i18n::s().message_hint).size(15.0).color(pal().text_muted))
                .desired_width(f32::INFINITY)
                .text_color(pal().text)
                .font(FontId::new(15.0, FontFamily::Proportional))
                .margin(Margin::ZERO),
        );
        let typed = input_response.changed();

        if header_button(ui, "send_button", send_c) {
            do_send = true;
        }

        if self.focus_input {
            input_response.request_focus();
            self.focus_input = false;
        }

        let (enter, shift) = ui.input(|i| (i.key_pressed(Key::Enter), i.modifiers.shift));
        if enter && !shift && input_response.has_focus() {
            do_send = true;
        }

        if toggle_emoji {
            self.show_emoji = !self.show_emoji;
            if self.show_emoji {
                self.emoji_open_time = self.time;
            }
        }
        if do_send {
            self.send_message();
            input_response.request_focus();
            self.show_emoji = false;
        } else if typed {
            if !self.input.is_empty()
                && self.time - self.last_typing > 2.0
                && self.status == ConnStatus::Online
            {
                if let Some(peer) = self.selected {
                    self.net.send(ClientMsg::Typing { to: peer });
                }
                self.last_typing = self.time;
            }
        }
    }

    fn emoji_popup(&mut self, ctx: &Context) {
        if !self.show_emoji {
            return;
        }
        const EMOJIS: [&str; 48] = [
            "😀", "😄", "😁", "😊", "🙂", "😉", "😍", "🤔", "😎", "😢", "😭", "😡", "🤯", "👍",
            "👎", "👏", "🙏", "💪", "🤝", "🔥", "💯", "❤️", "💔", "🎉", "🎁", "☕", "🍕", "🌙",
            "☀️", "⚡", "⭐", "🚀", "✨", "🤗", "😅", "😴", "🤣", "😇", "🤩", "🥳", "😱", "🥺",
            "😬", "🙃", "🧠", "💎", "🎧", "🍺",
        ];
        let mut insert: Option<&str> = None;
        // Не даём клику, открывшему панель, сразу вставить эмодзи.
        let allow_click = self.time - self.emoji_open_time > 0.15;
        let appear = if animations_on() {
            ease_out(((self.time - self.emoji_open_time) / 0.16) as f32)
        } else {
            1.0
        };
        if appear < 1.0 {
            ctx.request_repaint();
        }
        let tex_id = self.emoji_tex.id();
        let sheet_w = (EMOJI_CELL * EMOJI_COLS) as f32;
        let sheet_h = (EMOJI_CELL * EMOJI_ROWS) as f32;

        egui::Area::new(Id::new("emoji_picker"))
            .order(egui::Order::Foreground)
            .anchor(
                Align2::RIGHT_BOTTOM,
                Vec2::new(-18.0, -104.0 + (1.0 - appear) * 12.0),
            )
            .show(ctx, |ui| {
                Frame::none()
                    .fill(pal().panel)
                    .rounding(16.0)
                    .stroke(Stroke::new(1.0_f32, pal().line))
                    .shadow(egui::epaint::Shadow {
                        offset: Vec2::new(0.0, 12.0),
                        blur: 30.0,
                        spread: 0.0,
                        color: Color32::from_black_alpha(150),
                    })
                    .inner_margin(Margin::same(10.0))
                    .show(ui, |ui| {
                        ui.set_opacity(appear);
                        // Жёсткая ширина и размеры ячеек: панель не меняет размер
                        // между кадрами и не «дёргается» при наведении.
                        ui.set_width(325.0);
                        egui::Grid::new("emoji_grid")
                            .spacing(Vec2::new(3.0, 3.0))
                            .min_col_width(38.0)
                            .min_row_height(36.0)
                            .show(ui, |ui| {
                                for (i, emoji) in EMOJIS.iter().enumerate() {
                                    let (cell, resp) = ui
                                        .allocate_exact_size(Vec2::new(38.0, 36.0), Sense::click());
                                    let hovered = resp.hovered();
                                    if hovered {
                                        ui.painter().rect_filled(
                                            cell.shrink(1.0),
                                            10.0,
                                            veil(),
                                        );
                                    }
                                    let col = i % EMOJI_COLS;
                                    let row = i / EMOJI_COLS;
                                    let uv = Rect::from_min_max(
                                        egui::pos2(
                                            (col * EMOJI_CELL) as f32 / sheet_w,
                                            (row * EMOJI_CELL) as f32 / sheet_h,
                                        ),
                                        egui::pos2(
                                            ((col + 1) * EMOJI_CELL) as f32 / sheet_w,
                                            ((row + 1) * EMOJI_CELL) as f32 / sheet_h,
                                        ),
                                    );
                                    let image = Rect::from_center_size(
                                        cell.center(),
                                        Vec2::new(32.0, 32.0),
                                    );
                                    ui.painter()
                                        .image(tex_id, image, uv, Color32::WHITE);
                                    if allow_click
                                        && resp
                                            .on_hover_cursor(CursorIcon::PointingHand)
                                            .clicked()
                                    {
                                        insert = Some(emoji);
                                    }
                                    if i % EMOJI_COLS == EMOJI_COLS - 1 {
                                        ui.end_row();
                                    }
                                }
                            });
                    });
            });

        if let Some(emoji) = insert {
            self.input.push_str(emoji);
            self.focus_input = true;
        }
        // Панель закрывается только кнопкой смайлика или Esc — так она не
        // мигает и не «дёргается» при случайных нажатиях мыши.
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.show_emoji = false;
        }
    }

    fn profile_window(&mut self, ctx: &Context) {
        if !self.show_profile {
            self.show_profile_time = 0.0;
            return;
        }
        let now = ctx.input(|i| i.time);
        if self.show_profile_time <= 0.0 {
            self.show_profile_time = now;
        }
        let appear = if animations_on() {
            ease_out(((now - self.show_profile_time) / 0.18) as f32)
        } else {
            1.0
        };
        if appear < 1.0 {
            ctx.request_repaint();
        }
        let mut open = true;
        let my_ip = self.my_ip.clone();
        let hosting = self.hosting;
        let mut logout = false;

        egui::Window::new(i18n::s().profile)
            .id(Id::new("profile"))
            .open(&mut open)
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(
                Align2::LEFT_BOTTOM,
                Vec2::new(18.0, -84.0 + (1.0 - appear) * 10.0),
            )
            .frame(
                Frame::none()
                    .fill(pal().panel)
                    .rounding(18.0)
                    .stroke(Stroke::new(1.0_f32, pal().line))
                    .inner_margin(Margin::same(20.0)),
            )
            .show(ctx, |ui| {
                ui.set_opacity(appear);
                ui.set_width(270.0);
                if let Some(me) = self.me.clone() {
                    ui.horizontal(|ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::new(54.0, 54.0), Sense::hover());
                        avatar(ui.painter(), rect.center(), 27.0, &me.name, me.color, None);
                        ui.add_space(10.0);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(&me.name)
                                    .font(FontId::new(16.0, bold()))
                                    .color(pal().text),
                            );
                            ui.label(
                                RichText::new(format!("@{}", me.username))
                                    .size(12.5)
                                    .color(pal().text_dim),
                            );
                        });
                    });
                    ui.add_space(14.0);
                    info_row(ui, i18n::s().id_label, &me.id.to_string());
                    info_row(ui, i18n::s().server_label, &self.login_addr);
                    if hosting {
                        info_row(
                            ui,
                            i18n::s().my_address,
                            &format!("{}:{}", my_ip, DEFAULT_PORT),
                        );
                    }
                    ui.add_space(12.0);
                    if soft_button(ui, i18n::s().logout, 40.0).clicked() {
                        logout = true;
                    }
                }
            });

        if logout {
            self.logout();
            return;
        }
        self.show_profile = open;
    }

    fn settings_window(&mut self, ctx: &Context) {
        if !self.show_settings {
            return;
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) && !self.show_emoji {
            self.show_settings = false;
            self.settings_open_time = 0.0;
            return;
        }
        let c = pal();
        let mut open = true;
        let mut chosen: Option<&'static str> = None;
        let mut lang_choice: Option<crate::i18n::Lang> = None;
        let mut scale = self.scale;
        let mut animations = self.animations;
        let mut close = false;

        // Появление: лёгкий подъём и растворение, как у окон macOS.
        let now = ctx.input(|i| i.time);
        if self.settings_open_time <= 0.0 {
            self.settings_open_time = now;
        }
        let appear = if animations_on() {
            ease_out(((now - self.settings_open_time) / 0.18) as f32)
        } else {
            1.0
        };
        if appear < 1.0 {
            ctx.request_repaint();
        }

        egui::Window::new(i18n::s().settings)
            .id(Id::new("settings"))
            .open(&mut open)
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(
                Align2::CENTER_CENTER,
                Vec2::new(0.0, (1.0 - appear) * 14.0),
            )
            .frame(
                Frame::none()
                    .fill(c.panel)
                    .rounding(22.0)
                    .stroke(Stroke::new(1.0_f32, c.line))
                    .shadow(window_shadow())
                    .inner_margin(Margin::same(22.0)),
            )
            .show(ctx, |ui| {
                ui.set_opacity(appear);
                ui.set_width(508.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(i18n::s().settings)
                                .font(FontId::new(20.0, bold()))
                                .color(c.text),
                        );
                        ui.label(
                            RichText::new(i18n::s().settings_sub)
                                .size(12.0)
                                .color(c.text_muted),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (rect, resp) =
                            ui.allocate_exact_size(Vec2::new(30.0, 30.0), Sense::click());
                        let hovered = resp.hovered();
                        if hovered {
                            ui.painter().circle_filled(rect.center(), 15.0, c.hover);
                        }
                        close_icon(
                            ui.painter(),
                            rect.center(),
                            5.0,
                            if hovered { c.text } else { c.text_muted },
                        );
                        if resp
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                        {
                            close = true;
                        }
                    });
                });
                ui.add_space(12.0);
                // Средняя часть прокручивается, если окно низкое.
                let list_h = (ctx.screen_rect().height() - 215.0).clamp(280.0, 420.0);
                ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .max_height(list_h)
                    .show(ui, |ui| {
                        section_label(ui, i18n::s().theme_section, c.text_muted);
                        ui.add_space(9.0);

                        ui.spacing_mut().item_spacing.x = 10.0;
                        for row in THEMES.chunks(2) {
                            ui.horizontal(|ui| {
                                for theme in row {
                                    if theme_card(ui, theme, &self.theme_id) {
                                        chosen = Some(theme.id);
                                    }
                                }
                            });
                            ui.add_space(10.0);
                        }

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            section_label(ui, i18n::s().language_section, c.text_muted);
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                for lang in crate::i18n::LANGS.iter().rev() {
                                    if lang_chip(ui, lang.native(), *lang == crate::i18n::lang()) {
                                        lang_choice = Some(*lang);
                                    }
                                }
                            });
                        });

                        ui.add_space(14.0);
                        section_label(ui, i18n::s().interface_section, c.text_muted);
                        ui.add_space(9.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(i18n::s().scale_label)
                                    .size(13.0)
                                    .color(c.text_dim),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                for (label, value) in [
                                    ("125 %", 1.25_f32),
                                    ("110 %", 1.10),
                                    ("100 %", 1.00),
                                    ("90 %", 0.90),
                                ] {
                                    if scale_chip(ui, label, value, scale) {
                                        scale = value;
                                    }
                                }
                            });
                        });
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(i18n::s().animations_label)
                                        .size(13.5)
                                        .color(c.text),
                                );
                                ui.label(
                                    RichText::new(i18n::s().animations_hint)
                                        .size(11.5)
                                        .color(c.text_muted),
                                );
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if toggle_switch(ui, animations) {
                                    animations = !animations;
                                }
                            });
                        });
                    });

                ui.add_space(12.0);
                let line_y = ui.cursor().top();
                ui.painter().line_segment(
                    [
                        egui::pos2(ui.max_rect().left(), line_y),
                        egui::pos2(ui.max_rect().right(), line_y),
                    ],
                    Stroke::new(1.0_f32, c.line),
                );
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "{} {}  ·  {}",
                            i18n::s().app_name,
                            env!("CARGO_PKG_VERSION"),
                            i18n::theme_name(&self.theme_id)
                        ))
                        .size(11.0)
                        .color(c.text_muted),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if settings_button(ui, i18n::s().done) {
                            close = true;
                        }
                    });
                });
            });

        if let Some(id) = chosen {
            self.theme_id = id.to_string();
            crate::theme::apply(ctx, id, true);
            self.cfg.theme = Some(self.theme_id.clone());
            save_config(&self.cfg);
        }
        if let Some(lang) = lang_choice {
            crate::i18n::set(lang);
            self.cfg.language = Some(lang.id().to_string());
            save_config(&self.cfg);
        }
        if (scale - self.scale).abs() > 0.001 {
            self.scale = scale;
            ctx.set_zoom_factor(scale);
            self.cfg.scale = Some(scale);
            save_config(&self.cfg);
        }
        if animations != self.animations {
            self.animations = animations;
            crate::theme::set_animation(ctx, animations);
            self.cfg.animations = Some(animations);
            save_config(&self.cfg);
        }
        if close || !open {
            self.show_settings = false;
            self.settings_open_time = 0.0;
        }
    }

    fn show_toast(&mut self, ctx: &Context) {
        if self.me.is_none() {
            return;
        }
        if let Some((text, at)) = self.notice.clone() {
            let appear = if animations_on() {
                ease_out(((self.time - at) / 0.18) as f32)
            } else {
                1.0
            };
            if appear < 1.0 {
                ctx.request_repaint();
            }
            egui::Area::new(Id::new("toast"))
                .order(egui::Order::Tooltip)
                .anchor(
                    Align2::CENTER_TOP,
                    Vec2::new(0.0, 18.0 - (1.0 - appear) * 10.0),
                )
                .show(ctx, |ui| {
                    ui.set_opacity(appear);
                    Frame::none()
                        .fill(pal().panel)
                        .rounding(12.0)
                        .stroke(Stroke::new(1.0_f32, pal().line))
                        .inner_margin(Margin::symmetric(14.0, 8.0))
                        .show(ui, |ui| {
                            ui.label(RichText::new(text).size(12.5).color(pal().text_dim));
                        });
                });
        }
    }
    /// Первый кадр: аккуратно ставим окно по центру экрана, а если оно почти
    /// во всю высоту — прижимаем к верху, чтобы не заезжать под панель задач.
    fn place_window(&mut self, ctx: &Context) {
        if self.positioned {
            return;
        }
        let info = ctx.input(|i| i.viewport().clone());
        let (Some(monitor), Some(outer)) = (info.monitor_size, info.outer_rect) else {
            return;
        };
        // egui отдаёт размер монитора в физических пикселях, а окно — в points.
        let ppp = info.native_pixels_per_point.unwrap_or(1.0).max(0.1);
        let monitor = monitor / ppp;
        let spare = monitor.y - outer.height();
        let x = ((monitor.x - outer.width()) * 0.5).max(0.0);
        let y = if spare > 80.0 {
            (spare * 0.5 - 20.0).max(0.0)
        } else {
            0.0
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(x, y)));
        self.positioned = true;
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.place_window(ctx);
        self.time = ctx.input(|i| i.time);
        self.handle_net();
        if let Some((_, at)) = &self.notice {
            if self.time - *at > 6.0 {
                self.notice = None;
            }
        }
        self.set_title(ctx);
        crate::theme::tick(ctx);
        if self.me.is_some() {
            self.main_ui(ctx);
        } else {
            self.login_ui(ctx);
        }
        self.settings_window(ctx);
        if self.show_emoji || self.connecting || self.any_typing() {
            ctx.request_repaint_after(Duration::from_millis(150));
        }
    }
}

// ---------------------------------------------------------------- элементы

fn field(
    ui: &mut Ui,
    label: &str,
    value: &mut String,
    hint: &str,
    password: bool,
) -> egui::Response {
    ui.label(RichText::new(label).size(11.5).color(pal().text_muted));
    ui.add_space(3.0);
    let mut response = None;
    Frame::none()
        .fill(pal().field)
        .rounding(12.0)
        .stroke(Stroke::new(1.0_f32, pal().line))
        .inner_margin(Margin::symmetric(13.0, 9.0))
        .show(ui, |ui| {
            let mut edit = TextEdit::singleline(value)
                .frame(false)
                .hint_text(RichText::new(hint).size(14.5).color(pal().text_muted))
                .desired_width(f32::INFINITY)
                .text_color(pal().text)
                .font(FontId::new(14.5, FontFamily::Proportional))
                .margin(Margin::ZERO);
            if password {
                edit = edit.password(true);
            }
            response = Some(ui.add(edit));
        });
    ui.add_space(11.0);
    response.unwrap_or_else(|| ui.allocate_response(Vec2::ZERO, Sense::hover()))
}

fn info_row(ui: &mut Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.0).color(pal().text_muted));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(value).size(12.0).color(pal().text_dim));
        });
    });
    ui.add_space(4.0);
}

fn rail_button(
    ui: &mut Ui,
    id: &str,
    center: Pos2,
    active: bool,
    icon: impl FnOnce(&Painter, Pos2, f32, Color32),
) -> bool {
    let rect = Rect::from_center_size(center, Vec2::new(42.0, 42.0));
    let resp = ui
        .interact(rect, Id::new(id), Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand);
    if active {
        ui.painter().circle_filled(center, 18.0, pal().selected);
    }
    let color = if active || resp.hovered() {
        if active {
            pal().accent
        } else {
            pal().text
        }
    } else {
        pal().text_muted
    };
    icon(ui.painter(), center, 10.0, color);
    resp.clicked()
}

fn header_button(ui: &mut Ui, id: &str, center: Pos2) -> bool {
    let rect = Rect::from_center_size(center, Vec2::new(38.0, 38.0));
    ui.interact(rect, Id::new(id), Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand)
        .clicked()
}

fn chat_row(
    ui: &mut Ui,
    chat: &ChatSummary,
    selected: bool,
    my_id: u64,
    typing: bool,
    appear: f32,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 64.0),
        Sense::click(),
    );
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    let clicked = resp.clicked();
    let appear = appear.clamp(0.0, 1.0);
    let rect = rect.translate(Vec2::new((1.0 - appear) * 14.0, 0.0));
    ui.set_opacity(appear);

    // У Телеграма выбранная строка — сплошная заливка без карточек и отступов.
    let hover = anim_bool(
        ui.ctx(),
        ui.id().with(("row", chat.user.id)),
        resp.hovered(),
        0.14,
    );
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, 0.0, pal().selected);
    } else if hover > 0.0 {
        painter.rect_filled(rect, 0.0, with_alpha(pal().hover, hover));
    }

    let avatar_center = egui::pos2(rect.left() + 34.0, rect.center().y);
    avatar(
        painter,
        avatar_center,
        21.0,
        &chat.user.name,
        chat.user.color,
        None,
    );

    let text_left = rect.left() + 66.0;
    let time_font = FontId::new(11.0, FontFamily::Proportional);
    let time_text = timefmt::list_time(chat.last_ts);
    let time_width = ui
        .painter()
        .layout_no_wrap(time_text.clone(), time_font.clone(), pal().text_muted)
        .size()
        .x;
    let name = elide(
        ui,
        &chat.user.name,
        FontId::new(15.0, bold()),
        rect.right() - text_left - time_width - 26.0,
    );
    painter.text(
        egui::pos2(text_left, rect.top() + 22.0),
        Align2::LEFT_CENTER,
        name,
        FontId::new(15.0, bold()),
        pal().text,
    );

    let preview_text = if typing {
        i18n::s().typing.to_string()
    } else if chat.last_ts == 0 {
        format!("@{}", chat.user.username)
    } else {
        let body = chat.last_text.replace('\n', " ");
        if chat.last_from == my_id {
            format!("{}{}", i18n::s().you_prefix, body)
        } else {
            body
        }
    };
    let preview_color = if typing { pal().accent } else { pal().text_dim };
    let preview_limit = if chat.unread > 0 {
        rect.right() - text_left - 46.0
    } else {
        rect.right() - text_left - 16.0
    };
    let preview = elide(
        ui,
        &preview_text,
        FontId::new(13.5, FontFamily::Proportional),
        preview_limit.max(24.0),
    );
    painter.text(
        egui::pos2(text_left, rect.top() + 43.0),
        Align2::LEFT_CENTER,
        preview,
        FontId::new(13.5, FontFamily::Proportional),
        preview_color,
    );

    painter.text(
        egui::pos2(rect.right() - 14.0, rect.top() + 22.0),
        Align2::RIGHT_CENTER,
        time_text,
        time_font,
        if chat.unread > 0 { pal().accent } else { pal().text_muted },
    );

    if chat.unread > 0 {
        let badge = egui::pos2(rect.right() - 22.0, rect.top() + 44.0);
        painter.circle_filled(badge, 10.0, pal().accent);
        painter.text(
            badge,
            Align2::CENTER_CENTER,
            if chat.unread > 99 {
                "99+".to_string()
            } else {
                chat.unread.to_string()
            },
            FontId::new(10.5, bold()),
            pal().on_accent,
        );
    }

    ui.set_opacity(1.0);
    clicked
}

fn paint_messages(ui: &mut Ui, msgs: &[Message], my_id: u64, read_up: i64) {
    let mut last_day = String::new();
    for msg in msgs {
        let day = timefmt::date_label(msg.ts);
        if day != last_day {
            last_day = day.clone();
            ui.add_space(9.0);
            chip(ui, &day);
            ui.add_space(9.0);
        }
        bubble(ui, msg, msg.from == my_id, read_up);
    }
}

/// Заголовок раздела в настройках.
fn section_label(ui: &mut Ui, text: &str, color: Color32) {
    ui.label(
        RichText::new(text)
            .font(FontId::new(11.0, bold()))
            .color(color),
    );
}

/// Карточка темы: мини-превью чата в её цветах и название.
fn theme_card(ui: &mut Ui, theme: &ThemeDef, current: &str) -> bool {
    let p = theme.pal;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(246.0, 92.0), Sense::click());
    let selected = theme.id == current;
    let painter = ui.painter();
    painter.rect_filled(rect, 14.0, p.panel);

    // Миниатюра: панель, список чатов и два пузыря сообщений.
    let prev = Rect::from_min_size(rect.min + Vec2::new(12.0, 12.0), Vec2::new(72.0, 68.0));
    painter.rect_filled(prev, 10.0, p.bg);
    painter.rect_filled(
        Rect::from_min_size(prev.min, Vec2::new(20.0, 68.0)),
        egui::Rounding {
            nw: 8.0,
            sw: 8.0,
            ne: 0.0,
            se: 0.0,
        },
        p.sidebar,
    );
    painter.rect_filled(
        Rect::from_min_size(prev.min + Vec2::new(26.0, 14.0), Vec2::new(36.0, 11.0)),
        5.5,
        p.in_bubble,
    );
    painter.rect_filled(
        Rect::from_min_size(prev.min + Vec2::new(33.0, 32.0), Vec2::new(32.0, 11.0)),
        5.5,
        p.out_bubble,
    );
    painter.rect_filled(
        Rect::from_min_size(prev.min + Vec2::new(26.0, 50.0), Vec2::new(26.0, 9.0)),
        4.5,
        p.field,
    );
    painter.circle_filled(prev.min + Vec2::new(59.0, 46.0), 5.5, p.accent);

    painter.text(
        rect.min + Vec2::new(96.0, 31.0),
        Align2::LEFT_CENTER,
        i18n::theme_name(theme.id),
        FontId::new(14.5, bold()),
        p.text,
    );
    painter.text(
        rect.min + Vec2::new(96.0, 51.0),
        Align2::LEFT_CENTER,
        i18n::theme_hint(theme.id),
        FontId::new(11.5, FontFamily::Proportional),
        p.text_dim,
    );

    let border = if selected {
        Stroke::new(2.0_f32, pal().accent)
    } else if resp.hovered() {
        Stroke::new(1.5_f32, pal().text_muted)
    } else {
        Stroke::new(1.0_f32, p.line)
    };
    painter.rect_stroke(rect, 14.0, border);

    if selected {
        let badge = rect.right_top() + Vec2::new(-18.0, 18.0);
        painter.circle_filled(badge, 9.0, pal().accent);
        painter.text(
            badge,
            Align2::CENTER_CENTER,
            "✓",
            FontId::new(10.0, bold()),
            pal().on_accent,
        );
    }

    resp.on_hover_cursor(CursorIcon::PointingHand).clicked()
}

/// Кнопка выбора масштаба интерфейса.
fn scale_chip(ui: &mut Ui, label: &str, value: f32, current: f32) -> bool {
    let c = pal();
    let selected = (current - value).abs() < 0.001;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(58.0, 28.0), Sense::click());
    let hover = anim_bool(
        ui.ctx(),
        ui.id().with(("scale", label)),
        resp.hovered(),
        0.14,
    );
    let fill = if selected {
        mix(c.accent, c.accent_hover, hover * 0.7)
    } else {
        mix(c.field, c.hover, hover)
    };
    ui.painter().rect_filled(rect, 9.0, fill);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::new(11.5, bold()),
        if selected { c.on_accent } else { c.text_dim },
    );
    resp.on_hover_cursor(CursorIcon::PointingHand).clicked()
}

/// Чип выбора языка: название всегда на самом языке.
fn lang_chip(ui: &mut Ui, label: &str, selected: bool) -> bool {
    let c = pal();
    let color = if selected { c.on_accent } else { c.text_dim };
    let font = FontId::new(12.0, bold());
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font.clone(), color);
    let width = (galley.size().x + 26.0).max(64.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 30.0), Sense::click());
    let hover = anim_bool(
        ui.ctx(),
        ui.id().with(("lang", label)),
        resp.hovered(),
        0.14,
    );
    let fill = if selected {
        mix(c.accent, c.accent_hover, hover * 0.7)
    } else {
        mix(c.field, c.hover, hover)
    };
    ui.painter().rect_filled(rect, 10.0, fill);
    ui.painter().galley(
        rect.center() - galley.size() * 0.5,
        galley,
        color,
    );
    resp.on_hover_cursor(CursorIcon::PointingHand).clicked()
}

/// Переключатель «вкл/выкл» с плавно переезжающим кружком.
fn toggle_switch(ui: &mut Ui, on: bool) -> bool {
    let c = pal();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(42.0, 24.0), Sense::click());
    let t = anim_bool(ui.ctx(), ui.id().with("toggle_switch"), on, 0.16);
    let painter = ui.painter();
    painter.rect_filled(rect, 12.0, mix(c.field, c.accent, t));
    let knob_x = rect.left() + 13.0 + (rect.width() - 26.0) * t;
    painter.circle_filled(
        egui::pos2(knob_x, rect.center().y),
        9.0,
        mix(c.text_muted, c.on_accent, t),
    );
    resp.on_hover_cursor(CursorIcon::PointingHand).clicked()
}

/// Небольшая акцентная кнопка для нижней строки настроек.
fn settings_button(ui: &mut Ui, label: &str) -> bool {
    let c = pal();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(96.0, 36.0), Sense::click());
    let hover = anim_bool(
        ui.ctx(),
        ui.id().with(("settings_btn", label)),
        resp.hovered(),
        0.14,
    );
    let down = anim_bool(
        ui.ctx(),
        ui.id().with(("settings_btn_down", label)),
        resp.is_pointer_button_down_on(),
        0.07,
    );
    let fill = mix(mix(c.accent, c.accent_hover, hover), c.text, down * 0.10);
    let k = 1.0 + 0.012 * hover - 0.018 * down;
    let draw = Rect::from_center_size(rect.center(), rect.size() * k);
    ui.painter().rect_filled(draw, 11.0, fill);
    ui.painter().text(
        draw.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::new(13.0 * k, bold()),
        c.on_accent,
    );
    resp.on_hover_cursor(CursorIcon::PointingHand).clicked()
}

fn bubble(ui: &mut Ui, msg: &Message, mine: bool, read_up: i64) {
    let avail = ui.available_width();
    let max_text_width = (avail * 0.66).clamp(120.0, 580.0);

    let text_color = pal().text;
    let font = FontId::new(15.0, FontFamily::Proportional);
    let galley = ui.fonts(|f| f.layout(msg.text.clone(), font, text_color, max_text_width));
    let text_size = galley.size();

    let read = read_up >= msg.ts;
    let raw_time = timefmt::hhmm(msg.ts);
    let meta_text = match (mine, read) {
        (true, true) => format!("✓✓  {}", raw_time),
        (true, false) => format!("✓  {}", raw_time),
        (false, _) => raw_time.clone(),
    };
    let meta_color = if mine {
        if read {
            pal().out_meta
        } else {
            pal().out_meta_dim
        }
    } else {
        pal().text_muted
    };
    let meta_galley = ui.fonts(|f| {
        f.layout_no_wrap(
            meta_text,
            FontId::new(11.0, FontFamily::Proportional),
            meta_color,
        )
    });
    let meta_size = meta_galley.size();

    let pad_x = 13.0;
    let pad_y = 8.0;
    let gap = 4.0;
    let bubble_w = text_size.x.max(meta_size.x) + pad_x * 2.0;
    let bubble_h = text_size.y + gap + meta_size.y + pad_y * 2.0;

    let layout = if mine {
        Layout::right_to_left(Align::TOP)
    } else {
        Layout::left_to_right(Align::TOP)
    };

    ui.with_layout(layout, |ui| {
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(bubble_w, bubble_h), Sense::click());
        let fill = if mine { pal().out_bubble } else { pal().in_bubble };
        let rounding = if mine {
            egui::Rounding {
                nw: 12.0,
                ne: 4.0,
                sw: 12.0,
                se: 12.0,
            }
        } else {
            egui::Rounding {
                nw: 4.0,
                ne: 12.0,
                sw: 12.0,
                se: 12.0,
            }
        };
        let painter = ui.painter();
        painter.rect_filled(rect, rounding, fill);
        painter.galley(
            rect.min + Vec2::new(pad_x, pad_y),
            galley,
            text_color,
        );
        painter.galley(
            egui::pos2(
                rect.right() - pad_x - meta_size.x,
                rect.bottom() - pad_y - meta_size.y,
            ),
            meta_galley,
            meta_color,
        );

        resp.on_hover_cursor(CursorIcon::PointingHand)
            .context_menu(|ui| {
                if ui.button(i18n::s().copy_text).clicked() {
                    ui.output_mut(|out| out.copied_text = msg.text.clone());
                    ui.close_menu();
                }
            });
    });
    ui.add_space(5.0);
}

