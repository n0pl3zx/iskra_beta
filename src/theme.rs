//! Оформление «Искры»: темы, шрифты, иконки и мелкие помощники отрисовки.

use egui::{
    Align2, Color32, Context, FontData, FontDefinitions, FontFamily, FontId, Painter, Pos2, Rect,
    Response, Sense, Shape, Stroke, Ui, Vec2,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;
use std::time::Instant;

// ------------------------------------------------------------------ темы

/// Набор цветов одного оформления. Значения копируются, поэтому палитру
/// можно безопасно брать из любого места интерфейса.
#[derive(Clone, Copy, PartialEq)]
pub struct Palette {
    pub dark: bool,
    pub bg: Color32,
    pub sidebar: Color32,
    pub panel: Color32,
    pub field: Color32,
    pub hover: Color32,
    pub selected: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub text_muted: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub on_accent: Color32,
    pub out_bubble: Color32,
    pub in_bubble: Color32,
    pub out_meta: Color32,
    pub out_meta_dim: Color32,
    pub online: Color32,
    pub line: Color32,
    pub danger: Color32,
    pub warn: Color32,
    pub avatars: [Color32; 8],
}

impl Palette {
    /// Смешивает две палитры: используется для плавной смены темы.
    fn blend(a: &Palette, b: &Palette, t: f32) -> Palette {
        Palette {
            dark: b.dark,
            bg: mix(a.bg, b.bg, t),
            sidebar: mix(a.sidebar, b.sidebar, t),
            panel: mix(a.panel, b.panel, t),
            field: mix(a.field, b.field, t),
            hover: mix(a.hover, b.hover, t),
            selected: mix(a.selected, b.selected, t),
            text: mix(a.text, b.text, t),
            text_dim: mix(a.text_dim, b.text_dim, t),
            text_muted: mix(a.text_muted, b.text_muted, t),
            accent: mix(a.accent, b.accent, t),
            accent_hover: mix(a.accent_hover, b.accent_hover, t),
            on_accent: mix(a.on_accent, b.on_accent, t),
            out_bubble: mix(a.out_bubble, b.out_bubble, t),
            in_bubble: mix(a.in_bubble, b.in_bubble, t),
            out_meta: mix(a.out_meta, b.out_meta, t),
            out_meta_dim: mix(a.out_meta_dim, b.out_meta_dim, t),
            online: mix(a.online, b.online, t),
            line: mix(a.line, b.line, t),
            danger: mix(a.danger, b.danger, t),
            warn: mix(a.warn, b.warn, t),
            avatars: std::array::from_fn(|i| mix(a.avatars[i], b.avatars[i], t)),
        }
    }
}

pub struct ThemeDef {
    pub id: &'static str,
    pub pal: Palette,
}

const GRAPHITE: Palette = Palette {
    dark: true,
    bg: Color32::from_rgb(10, 10, 11),
    sidebar: Color32::from_rgb(17, 17, 19),
    panel: Color32::from_rgb(23, 23, 25),
    field: Color32::from_rgb(30, 30, 33),
    hover: Color32::from_rgb(37, 37, 40),
    selected: Color32::from_rgb(51, 51, 56),
    text: Color32::from_rgb(244, 244, 244),
    text_dim: Color32::from_rgb(163, 163, 168),
    text_muted: Color32::from_rgb(112, 112, 118),
    accent: Color32::from_rgb(232, 232, 232),
    accent_hover: Color32::from_rgb(255, 255, 255),
    on_accent: Color32::from_rgb(16, 16, 18),
    out_bubble: Color32::from_rgb(62, 62, 66),
    in_bubble: Color32::from_rgb(29, 29, 32),
    out_meta: Color32::from_rgb(232, 232, 232),
    out_meta_dim: Color32::from_rgb(180, 180, 184),
    online: Color32::from_rgb(216, 216, 216),
    line: Color32::from_rgb(42, 42, 46),
    danger: Color32::from_rgb(255, 255, 255),
    warn: Color32::from_rgb(158, 158, 163),
    avatars: [
        Color32::from_rgb(86, 86, 91),
        Color32::from_rgb(118, 118, 123),
        Color32::from_rgb(150, 150, 155),
        Color32::from_rgb(70, 70, 75),
        Color32::from_rgb(102, 102, 108),
        Color32::from_rgb(134, 134, 140),
        Color32::from_rgb(166, 166, 171),
        Color32::from_rgb(94, 94, 99),
    ],
};

const LIGHT: Palette = Palette {
    dark: false,
    bg: Color32::from_rgb(240, 240, 244),
    sidebar: Color32::from_rgb(255, 255, 255),
    panel: Color32::from_rgb(255, 255, 255),
    field: Color32::from_rgb(228, 228, 234),
    hover: Color32::from_rgb(222, 222, 229),
    selected: Color32::from_rgb(208, 208, 217),
    text: Color32::from_rgb(24, 24, 28),
    text_dim: Color32::from_rgb(92, 92, 100),
    text_muted: Color32::from_rgb(140, 140, 148),
    accent: Color32::from_rgb(26, 26, 30),
    accent_hover: Color32::from_rgb(54, 54, 60),
    on_accent: Color32::from_rgb(252, 252, 252),
    out_bubble: Color32::from_rgb(212, 212, 221),
    in_bubble: Color32::from_rgb(234, 234, 240),
    out_meta: Color32::from_rgb(58, 58, 66),
    out_meta_dim: Color32::from_rgb(100, 100, 108),
    online: Color32::from_rgb(38, 38, 44),
    line: Color32::from_rgb(222, 222, 228),
    danger: Color32::from_rgb(168, 42, 42),
    warn: Color32::from_rgb(150, 116, 30),
    avatars: [
        Color32::from_rgb(112, 112, 120),
        Color32::from_rgb(146, 146, 154),
        Color32::from_rgb(94, 94, 102),
        Color32::from_rgb(162, 162, 169),
        Color32::from_rgb(124, 124, 132),
        Color32::from_rgb(104, 104, 112),
        Color32::from_rgb(176, 176, 182),
        Color32::from_rgb(136, 136, 144),
    ],
};

const EMERALD: Palette = Palette {
    dark: true,
    bg: Color32::from_rgb(7, 14, 11),
    sidebar: Color32::from_rgb(12, 22, 18),
    panel: Color32::from_rgb(16, 28, 22),
    field: Color32::from_rgb(22, 38, 31),
    hover: Color32::from_rgb(29, 48, 39),
    selected: Color32::from_rgb(38, 66, 52),
    text: Color32::from_rgb(236, 246, 240),
    text_dim: Color32::from_rgb(156, 180, 166),
    text_muted: Color32::from_rgb(108, 134, 119),
    accent: Color32::from_rgb(47, 191, 113),
    accent_hover: Color32::from_rgb(82, 216, 143),
    on_accent: Color32::from_rgb(6, 22, 13),
    out_bubble: Color32::from_rgb(31, 78, 56),
    in_bubble: Color32::from_rgb(19, 34, 27),
    out_meta: Color32::from_rgb(202, 240, 218),
    out_meta_dim: Color32::from_rgb(164, 210, 182),
    online: Color32::from_rgb(88, 220, 146),
    line: Color32::from_rgb(30, 52, 41),
    danger: Color32::from_rgb(255, 124, 124),
    warn: Color32::from_rgb(228, 190, 96),
    avatars: [
        Color32::from_rgb(64, 132, 98),
        Color32::from_rgb(90, 162, 122),
        Color32::from_rgb(48, 108, 78),
        Color32::from_rgb(112, 178, 138),
        Color32::from_rgb(76, 148, 112),
        Color32::from_rgb(132, 190, 156),
        Color32::from_rgb(58, 120, 90),
        Color32::from_rgb(100, 168, 130),
    ],
};

const INDIGO: Palette = Palette {
    dark: true,
    bg: Color32::from_rgb(8, 8, 15),
    sidebar: Color32::from_rgb(13, 13, 23),
    panel: Color32::from_rgb(17, 17, 29),
    field: Color32::from_rgb(24, 24, 39),
    hover: Color32::from_rgb(30, 30, 48),
    selected: Color32::from_rgb(42, 42, 68),
    text: Color32::from_rgb(240, 240, 252),
    text_dim: Color32::from_rgb(158, 158, 184),
    text_muted: Color32::from_rgb(112, 112, 138),
    accent: Color32::from_rgb(120, 132, 250),
    accent_hover: Color32::from_rgb(150, 160, 255),
    on_accent: Color32::from_rgb(10, 10, 26),
    out_bubble: Color32::from_rgb(45, 52, 106),
    in_bubble: Color32::from_rgb(21, 22, 39),
    out_meta: Color32::from_rgb(216, 220, 255),
    out_meta_dim: Color32::from_rgb(170, 178, 226),
    online: Color32::from_rgb(130, 202, 255),
    line: Color32::from_rgb(31, 31, 52),
    danger: Color32::from_rgb(255, 122, 142),
    warn: Color32::from_rgb(232, 192, 104),
    avatars: [
        Color32::from_rgb(92, 94, 150),
        Color32::from_rgb(118, 120, 180),
        Color32::from_rgb(74, 76, 126),
        Color32::from_rgb(140, 142, 198),
        Color32::from_rgb(104, 106, 164),
        Color32::from_rgb(158, 160, 212),
        Color32::from_rgb(84, 86, 138),
        Color32::from_rgb(128, 130, 186),
    ],
};

pub const THEMES: [ThemeDef; 4] = [
    ThemeDef {
        id: "graphite",
        pal: GRAPHITE,
    },
    ThemeDef {
        id: "light",
        pal: LIGHT,
    },
    ThemeDef {
        id: "emerald",
        pal: EMERALD,
    },
    ThemeDef {
        id: "indigo",
        pal: INDIGO,
    },
];

static CURRENT: RwLock<Palette> = RwLock::new(GRAPHITE);

struct Transition {
    from: Palette,
    to: Palette,
    start: Instant,
}

static TRANSITION: RwLock<Option<Transition>> = RwLock::new(None);
static ANIMATIONS: AtomicBool = AtomicBool::new(true);

const FADE: f32 = 0.30;

/// Плавное замедление в конце — как у системных окон macOS.
pub fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Смешивает два цвета, включая прозрачность.
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(
        f(a.r(), b.r()),
        f(a.g(), b.g()),
        f(a.b(), b.b()),
        f(a.a(), b.a()),
    )
}

/// Копия цвета с другой прозрачностью (0..1).
pub fn with_alpha(c: Color32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        c.r(),
        c.g(),
        c.b(),
        (a.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

fn sample_transition() -> Option<(Palette, Palette, f32)> {
    let guard = TRANSITION.read().unwrap_or_else(|e| e.into_inner());
    let tr = guard.as_ref()?;
    let t = (tr.start.elapsed().as_secs_f32() / FADE).clamp(0.0, 1.0);
    if t >= 1.0 {
        drop(guard);
        *TRANSITION.write().unwrap_or_else(|e| e.into_inner()) = None;
        return None;
    }
    Some((tr.from, tr.to, t))
}

/// Текущая палитра. Во время смены темы цвета плавно смешиваются.
pub fn pal() -> Palette {
    if let Some((from, to, t)) = sample_transition() {
        return Palette::blend(&from, &to, ease_out(t));
    }
    *CURRENT.read().unwrap_or_else(|e| e.into_inner())
}

pub fn is_animating() -> bool {
    sample_transition().is_some()
}

/// Включены ли плавные переходы (переключатель в настройках).
pub fn animations_on() -> bool {
    ANIMATIONS.load(Ordering::Relaxed)
}

/// Анимация «вкл/выкл» с учётом общего переключателя анимаций.
pub fn anim_bool(ctx: &Context, id: egui::Id, value: bool, time: f32) -> f32 {
    if animations_on() {
        ctx.animate_bool_with_time(id, value, time)
    } else if value {
        1.0
    } else {
        0.0
    }
}

/// Кадровое обновление: пока идёт смена темы, перекрашиваем и виджеты egui.
pub fn tick(ctx: &Context) {
    if is_animating() {
        sync_visuals(ctx);
        ctx.request_repaint();
    }
}

pub fn theme_exists(id: &str) -> bool {
    THEMES.iter().any(|t| t.id == id)
}

fn shadow_color(alpha: u8) -> Color32 {
    Color32::from_black_alpha(if pal().dark { alpha } else { alpha / 3 })
}

/// Мягкая подсветка под курсором (на светлой теме — затемнение).
pub fn veil() -> Color32 {
    if pal().dark {
        Color32::from_white_alpha(26)
    } else {
        Color32::from_black_alpha(16)
    }
}

/// Едва заметная плашка: разделители дат, статусы.
pub fn wash() -> Color32 {
    if pal().dark {
        Color32::from_white_alpha(9)
    } else {
        Color32::from_black_alpha(7)
    }
}

/// Декоративное свечение на больших поверхностях.
pub fn glow() -> Color32 {
    if pal().dark {
        Color32::from_white_alpha(7)
    } else {
        Color32::from_black_alpha(6)
    }
}

pub fn glow_soft() -> Color32 {
    if pal().dark {
        Color32::from_white_alpha(5)
    } else {
        Color32::from_black_alpha(4)
    }
}

pub fn window_shadow() -> egui::epaint::Shadow {
    egui::epaint::Shadow {
        offset: Vec2::new(0.0, 12.0),
        blur: 36.0,
        spread: 0.0,
        color: shadow_color(150),
    }
}

// ---------------------------------------------------------------- шрифты

pub fn bold() -> FontFamily {
    FontFamily::Name("iskra-bold".into())
}

fn read_first(paths: &[&str]) -> Option<Vec<u8>> {
    for p in paths {
        if let Ok(bytes) = std::fs::read(p) {
            return Some(bytes);
        }
    }
    None
}

/// Загружает шрифты и базовые отступы. Цвета ставятся отдельно, в `apply`.
pub fn setup(ctx: &Context) {
    let mut fonts = FontDefinitions::default();

    let regular = read_first(&[
        "C:/Windows/Fonts/segoeui.ttf",
        "C:/Windows/Fonts/arial.ttf",
        "C:/Windows/Fonts/tahoma.ttf",
    ]);
    if let Some(bytes) = regular {
        fonts
            .font_data
            .insert("iskra-ui".to_owned(), FontData::from_owned(bytes));
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            fonts
                .families
                .entry(family)
                .or_default()
                .insert(0, "iskra-ui".to_owned());
        }
    }

    let fallback = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();

    // Эмодзи: цветной Segoe UI Emoji (отрисовывается контуром) + символы (✓ и т.п.)
    let has_emoji = if let Some(bytes) = read_first(&["C:/Windows/Fonts/seguiemj.ttf"]) {
        fonts
            .font_data
            .insert("iskra-emoji".to_owned(), FontData::from_owned(bytes));
        true
    } else {
        false
    };
    let has_symbols = if let Some(bytes) = read_first(&["C:/Windows/Fonts/seguisym.ttf"]) {
        fonts
            .font_data
            .insert("iskra-symbol".to_owned(), FontData::from_owned(bytes));
        true
    } else {
        false
    };

    let bold_font = read_first(&[
        "C:/Windows/Fonts/seguisb.ttf",
        "C:/Windows/Fonts/segoeuib.ttf",
        "C:/Windows/Fonts/arialbd.ttf",
    ]);
    let mut bold_family: Vec<String> = Vec::new();
    if let Some(bytes) = bold_font {
        fonts
            .font_data
            .insert("iskra-bold-font".to_owned(), FontData::from_owned(bytes));
        bold_family.push("iskra-bold-font".to_owned());
    }
    if fonts.font_data.contains_key("iskra-ui") {
        bold_family.push("iskra-ui".to_owned());
    }
    bold_family.extend(fallback.clone());
    if has_emoji {
        bold_family.insert(bold_family.len().min(2), "iskra-emoji".to_owned());
    }
    if has_symbols {
        bold_family.insert(bold_family.len().min(3), "iskra-symbol".to_owned());
    }
    if !bold_family.is_empty() {
        fonts.families.insert(bold(), bold_family);
    }
    {
        let prop = fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default();
        // Сразу после основного шрифта: эмодзи, затем символы (✓ и т.п.).
        if has_emoji && !prop.iter().any(|f| f == "iskra-emoji") {
            prop.insert(prop.len().min(1), "iskra-emoji".to_owned());
        }
        if has_symbols && !prop.iter().any(|f| f == "iskra-symbol") {
            prop.insert(prop.len().min(2), "iskra-symbol".to_owned());
        }
    }
    ctx.set_fonts(fonts);

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (
            egui::TextStyle::Body,
            FontId::new(15.0, FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Button,
            FontId::new(15.0, FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Small,
            FontId::new(12.0, FontFamily::Proportional),
        ),
        (egui::TextStyle::Heading, FontId::new(22.0, bold())),
        (
            egui::TextStyle::Monospace,
            FontId::new(13.0, FontFamily::Monospace),
        ),
    ]
    .into();

    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(10.0, 7.0);
    style.spacing.scroll.bar_width = 6.0;
    style.spacing.scroll.floating = true;
    style.animation_time = 0.12;
    ctx.set_style(style);
}

/// Делает тему активной. Если `animate`, цвета перетекают друг в друга.
pub fn apply(ctx: &Context, id: &str, animate: bool) {
    let from = pal();
    let theme = THEMES.iter().find(|t| t.id == id).unwrap_or(&THEMES[0]);
    let target = theme.pal;
    *CURRENT.write().unwrap_or_else(|e| e.into_inner()) = target;
    let animate = animate && ANIMATIONS.load(Ordering::Relaxed) && from != target;
    *TRANSITION.write().unwrap_or_else(|e| e.into_inner()) = if animate {
        Some(Transition {
            from,
            to: target,
            start: Instant::now(),
        })
    } else {
        None
    };
    sync_visuals(ctx);
}

/// Красит стандартные элементы egui (поля, меню, полосы прокрутки).
pub fn sync_visuals(ctx: &Context) {
    let c = pal();

    let mut style = (*ctx.style()).clone();
    let mut visuals = if c.dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.panel_fill = c.bg;
    visuals.window_fill = c.panel;
    visuals.extreme_bg_color = c.field;
    visuals.faint_bg_color = c.hover;
    visuals.override_text_color = Some(c.text);
    visuals.hyperlink_color = c.accent;
    visuals.selection.bg_fill = c.accent.gamma_multiply(if c.dark { 0.55 } else { 0.28 });
    visuals.selection.stroke = Stroke::new(1.0_f32, c.text);
    visuals.widgets.noninteractive.bg_fill = c.panel;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, c.line);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, c.text_dim);
    visuals.widgets.noninteractive.rounding = 10.0.into();
    visuals.widgets.inactive.bg_fill = c.field;
    visuals.widgets.inactive.weak_bg_fill = c.field;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, c.text);
    visuals.widgets.inactive.rounding = 10.0.into();
    visuals.widgets.hovered.bg_fill = c.hover;
    visuals.widgets.hovered.weak_bg_fill = c.hover;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, c.text);
    visuals.widgets.hovered.rounding = 10.0.into();
    visuals.widgets.active.bg_fill = c.selected;
    visuals.widgets.active.weak_bg_fill = c.selected;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, c.text);
    visuals.widgets.active.rounding = 10.0.into();
    visuals.window_rounding = 16.0.into();
    visuals.menu_rounding = 12.0.into();
    visuals.window_shadow = window_shadow();
    visuals.popup_shadow = visuals.window_shadow;
    style.visuals = visuals;
    ctx.set_style(style);
}

pub fn set_animation(ctx: &Context, enabled: bool) {
    ANIMATIONS.store(enabled, Ordering::Relaxed);
    if !enabled {
        *TRANSITION.write().unwrap_or_else(|e| e.into_inner()) = None;
    }
    ctx.style_mut(|style| style.animation_time = if enabled { 0.12 } else { 0.0 });
    sync_visuals(ctx);
}

// ---------------------------------------------------------------- элементы

/// Инициалы для аватарки: «Дмитрий Ковалёв» -> «ДК».
pub fn initials(name: &str) -> String {
    let mut letters = Vec::new();
    for word in name.split_whitespace() {
        if let Some(ch) = word.chars().find(|c| c.is_alphabetic() || c.is_numeric()) {
            letters.push(ch);
            if letters.len() == 2 {
                break;
            }
        }
    }
    if letters.is_empty() {
        return "?".to_string();
    }
    letters
        .into_iter()
        .collect::<String>()
        .to_uppercase()
}

pub fn avatar_colors(idx: u8) -> Color32 {
    let avatars = pal().avatars;
    avatars[(idx as usize) % avatars.len()]
}

/// Круглая аватарка с инициалами и (опционально) точкой присутствия.
pub fn avatar(
    p: &Painter,
    center: Pos2,
    radius: f32,
    name: &str,
    color_idx: u8,
    online: Option<bool>,
) {
    let c = pal();
    let base = avatar_colors(color_idx);
    p.circle_filled(center, radius, base);
    p.circle_filled(
        center - Vec2::new(radius * 0.22, radius * 0.28),
        radius * 0.62,
        Color32::from_rgba_unmultiplied(255, 255, 255, 20),
    );
    p.text(
        center,
        Align2::CENTER_CENTER,
        initials(name),
        FontId::new(radius * 0.8, bold()),
        Color32::WHITE,
    );
    if let Some(is_online) = online {
        let dot = center + Vec2::new(radius * 0.72, radius * 0.72);
        p.circle_filled(dot, radius * 0.30 + 2.5, c.sidebar);
        p.circle_filled(
            dot,
            radius * 0.30,
            if is_online {
                c.online
            } else {
                c.text_muted.gamma_multiply(0.86)
            },
        );
    }
}

/// Искра — четырёхлучевая звезда из двух ромбов (прежний логотип).
pub fn spark(p: &Painter, center: Pos2, r: f32, color: Color32) {
    let a = 0.30 * r;
    p.add(Shape::convex_polygon(
        vec![
            Pos2::new(center.x, center.y - r),
            Pos2::new(center.x + a, center.y),
            Pos2::new(center.x, center.y + r),
            Pos2::new(center.x - a, center.y),
        ],
        color,
        Stroke::NONE,
    ));
    p.add(Shape::convex_polygon(
        vec![
            Pos2::new(center.x - r, center.y),
            Pos2::new(center.x, center.y - a),
            Pos2::new(center.x + r, center.y),
            Pos2::new(center.x, center.y + a),
        ],
        color,
        Stroke::NONE,
    ));
}

/// Бумажный самолёт — символ отправки.
pub fn plane(p: &Painter, center: Pos2, size: f32, color: Color32) {
    let s = size;
    let left_wing = center + Vec2::new(-s * 0.52, -s * 0.24);
    let nose = center + Vec2::new(s * 0.58, -s * 0.52);
    let fold = center + Vec2::new(-s * 0.04, -s * 0.01);
    let bottom = center + Vec2::new(-s * 0.20, s * 0.54);
    p.add(Shape::convex_polygon(
        vec![left_wing, nose, fold],
        color,
        Stroke::NONE,
    ));
    p.add(Shape::convex_polygon(
        vec![left_wing, fold, bottom],
        color.gamma_multiply(0.78),
        Stroke::NONE,
    ));
}

/// Три полоски — меню слева вверху.
pub fn menu_icon(p: &Painter, center: Pos2, r: f32, color: Color32) {
    for i in -1..=1 {
        let y = center.y + i as f32 * r * 0.52;
        p.line_segment(
            [Pos2::new(center.x - r, y), Pos2::new(center.x + r, y)],
            Stroke::new(1.8_f32, color),
        );
    }
}

/// Шестерёнка — вход в настройки.
pub fn gear_icon(p: &Painter, center: Pos2, r: f32, color: Color32) {
    for i in 0..8 {
        let a = i as f32 * std::f32::consts::PI / 4.0;
        let dir = Vec2::new(a.cos(), a.sin());
        p.line_segment(
            [
                center + dir * (r * 0.82),
                center + dir * (r * 1.16),
            ],
            Stroke::new(2.2_f32, color),
        );
    }
    p.circle_stroke(center, r * 0.78, Stroke::new(r * 0.36, color));
}

pub fn smile_icon(p: &Painter, center: Pos2, r: f32, color: Color32) {
    p.circle_stroke(center, r, Stroke::new(1.6_f32, color));
    p.circle_filled(center + Vec2::new(-r * 0.34, -r * 0.20), 1.4, color);
    p.circle_filled(center + Vec2::new(r * 0.34, -r * 0.20), 1.4, color);
    let mut pts = Vec::new();
    for i in 0..=14 {
        let t = i as f32 / 14.0;
        let angle = 0.62 + t * (std::f32::consts::PI - 1.24);
        pts.push(center + Vec2::new(angle.cos() * r * 0.48, angle.sin() * r * 0.48));
    }
    p.add(Shape::line(pts, Stroke::new(1.5_f32, color)));
}

pub fn search_icon(p: &Painter, center: Pos2, r: f32, color: Color32) {
    p.circle_stroke(
        center + Vec2::new(-r * 0.15, -r * 0.15),
        r * 0.62,
        Stroke::new(1.6_f32, color),
    );
    p.line_segment(
        [
            center + Vec2::new(r * 0.33, r * 0.33),
            center + Vec2::new(r * 0.85, r * 0.85),
        ],
        Stroke::new(1.8_f32, color),
    );
}

/// Крестик для закрытия панелей.
pub fn close_icon(p: &Painter, center: Pos2, r: f32, color: Color32) {
    p.line_segment(
        [
            center + Vec2::new(-r, -r),
            center + Vec2::new(r, r),
        ],
        Stroke::new(1.8_f32, color),
    );
    p.line_segment(
        [
            center + Vec2::new(r, -r),
            center + Vec2::new(-r, r),
        ],
        Stroke::new(1.8_f32, color),
    );
}

/// Обрезает текст по ширине, добавляя многоточие.
pub fn elide(ui: &Ui, text: &str, font: FontId, max_width: f32) -> String {
    if max_width <= 8.0 {
        return String::new();
    }
    let measure = |t: &str| {
        ui.fonts(|f| {
            f.layout_no_wrap(t.to_owned(), font.clone(), Color32::WHITE)
                .size()
                .x
        })
    };
    if measure(text) <= max_width {
        return text.to_owned();
    }
    let mut out = String::new();
    for ch in text.chars() {
        let candidate = format!("{}{}…", out, ch);
        if measure(&candidate) > max_width {
            break;
        }
        out.push(ch);
    }
    format!("{}…", out.trim_end())
}

/// Акцентная кнопка во всю ширину.
pub fn primary_button(ui: &mut Ui, label: &str, height: f32, enabled: bool) -> Response {
    let c = pal();
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let hover = anim_bool(
        ui.ctx(),
        ui.id().with(("primary_hover", label)),
        resp.hovered(),
        0.14,
    );
    let down = anim_bool(
        ui.ctx(),
        ui.id().with(("primary_down", label)),
        resp.is_pointer_button_down_on(),
        0.07,
    );
    let fill = if !enabled {
        c.field
    } else {
        let base = mix(c.accent, c.accent_hover, hover);
        mix(base, c.text, down * 0.10)
    };
    // Лёгкое «дыхание» при наведении и сжатие при нажатии, как в macOS.
    let k = 1.0 + 0.012 * hover - 0.018 * down;
    let draw = Rect::from_center_size(rect.center(), rect.size() * k);
    ui.painter().rect_filled(draw, 13.0, fill);
    ui.painter().text(
        draw.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::new(14.0 * k, bold()),
        if enabled { c.on_accent } else { c.text_muted },
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Кнопка со спокойным фоном.
pub fn soft_button(ui: &mut Ui, label: &str, height: f32) -> Response {
    let c = pal();
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let hover = anim_bool(
        ui.ctx(),
        ui.id().with(("soft_hover", label)),
        resp.hovered(),
        0.14,
    );
    let fill = mix(c.field, c.hover, hover);
    ui.painter().rect_filled(rect, 12.0, fill);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::new(14.0, bold()),
        c.text,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Текстовая кнопка-ссылка.
pub fn link_button(ui: &mut Ui, label: &str, color: Color32) -> Response {
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        FontId::new(13.0, bold()),
        color,
    );
    let size = galley.size();
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hover = anim_bool(
        ui.ctx(),
        ui.id().with(("link_hover", label)),
        resp.hovered(),
        0.14,
    );
    ui.painter().galley(
        rect.min,
        galley,
        mix(color, color.gamma_multiply(1.22), hover),
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Полупрозрачная плашка с текстом (для разделителей дат).
pub fn chip(ui: &mut Ui, text: &str) {
    if text.is_empty() {
        return;
    }
    let c = pal();
    let font = FontId::new(11.0, bold());
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font, c.text_dim);
    let text_size = galley.size();
    let pill_size = Vec2::new(text_size.x + 20.0, text_size.y + 7.0);
    let full_width = ui.available_width();
    let (row, _) = ui.allocate_exact_size(Vec2::new(full_width, pill_size.y), Sense::hover());
    let pill = Rect::from_center_size(row.center(), pill_size);
    ui.painter().rect_filled(pill, pill_size.y * 0.5, wash());
    ui.painter().galley(
        egui::pos2(
            pill.center().x - text_size.x * 0.5,
            pill.center().y - text_size.y * 0.5,
        ),
        galley,
        c.text_dim,
    );
}

/// Плашка со светодиодом и текстом (состояние сервера/соединения).
pub fn status_pill(ui: &mut Ui, dot: Color32, text: &str) {
    let c = pal();
    let font = FontId::new(11.5, FontFamily::Proportional);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font, c.text_dim);
    let text_size = galley.size();
    let pill_size = Vec2::new(text_size.x + 34.0, 23.0);
    let full_width = ui.available_width();
    let (row, _) = ui.allocate_exact_size(Vec2::new(full_width, pill_size.y), Sense::hover());
    let pill = Rect::from_center_size(row.center(), pill_size);
    ui.painter().rect_filled(pill, pill_size.y * 0.5, wash());
    ui.painter()
        .circle_filled(egui::pos2(pill.left() + 11.0, pill.center().y), 3.5, dot);
    ui.painter().galley(
        egui::pos2(pill.left() + 22.0, pill.center().y - text_size.y * 0.5),
        galley,
        c.text_dim,
    );
}
