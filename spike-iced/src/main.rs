//! THROWAWAY Phase-0 spike for the UI re-platform — POLISH PASS.
//! See ~/.claude/plans/i-want-to-reconsider-curious-robin.md
//!
//! Rebuilds the Collection panel in Iced, wired DIRECTLY to the
//! `ferrous-mod-manager` core (no Tauri, no IPC). This revision is a deliberate
//! design pass over the first functional spike — bundled Inter typography, an
//! 8px spacing rhythm, a hand-tuned dark/light palette, Lucide SVG icons, soft
//! pill badges and elevated cards — to reveal Iced's *actual* aesthetic ceiling
//! rather than its defaults.

use std::collections::{BTreeMap, HashMap};

use iced::font::{Family, Weight};
use iced::widget::{
    Space, button, checkbox, column, container, mouse_area, row, scrollable, svg, text,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Shadow, Task, Theme, Vector};

use ferrous_mod_manager::achievements::achievement_status_for_mods;
use ferrous_mod_manager::conflict::conflict_detection;
use ferrous_mod_manager::detector::{detect_games, discover_mods};
use ferrous_mod_manager::models::ConflictCategory;

fn main() -> iced::Result {
    iced::application("Ferrous — Collection (Iced spike)", App::update, App::view)
        .theme(App::theme)
        .font(include_bytes!("../assets/fonts/Inter-Regular.ttf").as_slice())
        .font(include_bytes!("../assets/fonts/Inter-Medium.ttf").as_slice())
        .font(include_bytes!("../assets/fonts/Inter-SemiBold.ttf").as_slice())
        .font(include_bytes!("../assets/fonts/Inter-Bold.ttf").as_slice())
        .default_font(ui_font(Weight::Normal))
        .window_size(iced::Size::new(1120.0, 760.0))
        .run_with(App::new)
}

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

struct ModRow {
    name: String,
    mod_id: String,
    enabled: bool,
    achievement_ok: bool,
    achievement_categories: Vec<ConflictCategory>,
}

/// For a given mod: opposing-mod-name -> files they collide on (with category).
type ConflictsForMod = BTreeMap<String, Vec<(String, ConflictCategory)>>;

#[derive(Clone, Copy, PartialEq)]
enum Severity {
    High,
    Medium,
    Low,
}

struct App {
    game_name: String,
    rows: Vec<ModRow>,
    conflicts: HashMap<String, ConflictsForMod>,
    dark: bool,
    dragging: Option<usize>,
    expanded: Option<usize>,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    ToggleEnabled(usize),
    MoveUp(usize),
    MoveDown(usize),
    DragStart(usize),
    DragEnterRow(usize),
    DragEnd,
    ToggleExpanded(usize),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let (game_name, rows, conflicts) = load_from_core();
        (
            App {
                game_name,
                rows,
                conflicts,
                dark: true,
                dragging: None,
                expanded: None,
            },
            Task::none(),
        )
    }

    fn theme(&self) -> Theme {
        let s = skin(self.dark);
        Theme::custom(
            "Ferrous".to_string(),
            iced::theme::Palette {
                background: s.bg,
                text: s.text,
                primary: s.primary,
                success: s.success,
                danger: s.danger,
            },
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleTheme => self.dark = !self.dark,
            Message::ToggleEnabled(i) => {
                if let Some(r) = self.rows.get_mut(i) {
                    r.enabled = !r.enabled;
                }
            }
            Message::MoveUp(i) => {
                if i > 0 {
                    self.rows.swap(i, i - 1);
                    self.expanded = None;
                }
            }
            Message::MoveDown(i) => {
                if i + 1 < self.rows.len() {
                    self.rows.swap(i, i + 1);
                    self.expanded = None;
                }
            }
            Message::DragStart(i) => self.dragging = Some(i),
            Message::DragEnterRow(j) => {
                if let Some(i) = self.dragging
                    && i != j
                {
                    let r = self.rows.remove(i);
                    self.rows.insert(j, r);
                    self.dragging = Some(j);
                    self.expanded = None;
                }
            }
            Message::DragEnd => self.dragging = None,
            Message::ToggleExpanded(i) => {
                self.expanded = if self.expanded == Some(i) {
                    None
                } else {
                    Some(i)
                };
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let s = skin(self.dark);

        let list = self
            .rows
            .iter()
            .enumerate()
            .fold(column![].spacing(8), |col, (i, r)| {
                col.push(self.row_view(s, i, r))
            });

        let body = scrollable(container(list).padding([2, 4]))
            .height(Length::Fill)
            .width(Length::Fill);

        // Outer mouse_area catches the pointer release wherever it lands,
        // ending any in-flight drag.
        let body = mouse_area(body).on_release(Message::DragEnd);

        let content = column![self.header(s), divider(s.border), body].spacing(16);

        container(content)
            .padding(28)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_: &Theme| container::Style {
                background: Some(Background::Color(s.bg)),
                ..container::Style::default()
            })
            .into()
    }

    fn header(&self, s: Skin) -> Element<'_, Message> {
        let conflict_files: usize = self
            .conflicts
            .values()
            .flat_map(|m| m.values())
            .map(|v| v.len())
            .sum::<usize>()
            / 2;
        let blockers = self.rows.iter().filter(|r| !r.achievement_ok).count();

        let title = label(
            format!("{} — Collection", self.game_name),
            24.0,
            Weight::Semibold,
            s.text,
        );

        let subtitle = row![
            meta_dot(s.primary),
            label(
                format!("{} mods", self.rows.len()),
                13.0,
                Weight::Medium,
                s.muted
            ),
            meta_dot(s.danger),
            label(
                format!("{} conflicting files", conflict_files),
                13.0,
                Weight::Medium,
                s.muted
            ),
            meta_dot(s.warn),
            label(
                format!("{} achievement blockers", blockers),
                13.0,
                Weight::Medium,
                s.muted
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let (icon_src, lbl) = if self.dark {
            (I_MOON, "Dark")
        } else {
            (I_SUN, "Light")
        };
        let theme_btn = button(
            row![
                icon(icon_src, 15.0, s.icon),
                label(lbl.to_string(), 13.0, Weight::Medium, s.text),
            ]
            .spacing(7)
            .align_y(Alignment::Center),
        )
        .padding([7, 12])
        .on_press(Message::ToggleTheme)
        .style(move |_: &Theme, status| {
            let bg = match status {
                button::Status::Hovered | button::Status::Pressed => s.surface_hover,
                _ => s.surface,
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: s.text,
                border: Border {
                    radius: 9.0.into(),
                    width: 1.0,
                    color: s.border,
                },
                ..button::Style::default()
            }
        });

        row![
            column![title, subtitle].spacing(7),
            Space::with_width(Length::Fill),
            theme_btn,
        ]
        .align_y(Alignment::Center)
        .into()
    }

    fn row_view(&self, s: Skin, i: usize, r: &ModRow) -> Element<'_, Message> {
        let dragging = self.dragging == Some(i);
        let expanded = self.expanded == Some(i);

        let handle = mouse_area(icon(I_GRIP, 16.0, s.icon))
            .on_press(Message::DragStart(i))
            .interaction(iced::mouse::Interaction::Grab);

        let position = label(format!("{:>2}", i + 1), 13.0, Weight::Medium, s.faint);

        let enabled = checkbox("", r.enabled)
            .on_toggle(move |_| Message::ToggleEnabled(i))
            .size(17);

        let name_color = if r.enabled { s.text } else { s.faint };
        let title = label(r.name.clone(), 15.0, Weight::Semibold, name_color);
        let mod_id = label(format!("#{}", r.mod_id), 12.0, Weight::Normal, s.faint);

        let ach = if r.achievement_ok {
            pill(I_CHECK, "achievements".to_string(), s.success)
        } else {
            let cats: Vec<&str> = r
                .achievement_categories
                .iter()
                .map(|c| category_label(*c))
                .collect();
            pill(I_X, format!("achievements · {}", cats.join(", ")), s.warn)
        };

        let conflict_cell: Element<'_, Message> = match self.conflicts.get(&r.name) {
            Some(c) if !c.is_empty() => {
                let files: usize = c.values().map(|v| v.len()).sum();
                let color = severity_color(s, worst_severity(c));
                button(
                    row![
                        icon(I_ALERT, 13.0, color),
                        label(format!("{} conflicts", files), 12.0, Weight::Medium, color),
                        icon(
                            if expanded { I_CHEV_DOWN } else { I_CHEV_RIGHT },
                            12.0,
                            color
                        ),
                    ]
                    .spacing(5)
                    .align_y(Alignment::Center),
                )
                .padding([4, 9])
                .on_press(Message::ToggleExpanded(i))
                .style(move |_: &Theme, status| {
                    let a = match status {
                        button::Status::Hovered | button::Status::Pressed => 0.24,
                        _ => 0.14,
                    };
                    button::Style {
                        background: Some(Background::Color(alpha(color, a))),
                        text_color: color,
                        border: Border {
                            radius: 999.0.into(),
                            ..Border::default()
                        },
                        ..button::Style::default()
                    }
                })
                .into()
            }
            _ => label("no conflicts".into(), 12.0, Weight::Normal, s.faint).into(),
        };

        let reorder = column![
            ghost_icon_btn(s, I_CHEV_UP, Message::MoveUp(i)),
            ghost_icon_btn(s, I_CHEV_DOWN, Message::MoveDown(i)),
        ]
        .spacing(1);

        let main = row![
            handle,
            position,
            enabled,
            column![title, mod_id].spacing(3).width(Length::Fill),
            ach,
            conflict_cell,
            reorder,
        ]
        .spacing(14)
        .align_y(Alignment::Center)
        .padding([11, 14]);

        let card = container(main).style(move |_: &Theme| card_style(s, dragging));
        let row_area = mouse_area(card).on_enter(Message::DragEnterRow(i));

        if expanded && let Some(c) = self.conflicts.get(&r.name) {
            return column![row_area, conflict_detail(s, c)].spacing(0).into();
        }
        row_area.into()
    }
}

// ---------------------------------------------------------------------------
// Conflict detail panel (grouped by opposing mod, expandable)
// ---------------------------------------------------------------------------

fn conflict_detail(s: Skin, c: &ConflictsForMod) -> Element<'_, Message> {
    let mut groups = column![].spacing(12);
    for (opposing, files) in c {
        let header = row![
            icon(I_SWORDS, 14.0, s.muted),
            label(opposing.clone(), 13.5, Weight::Semibold, s.text),
            label(
                format!("{} files", files.len()),
                12.0,
                Weight::Normal,
                s.faint
            ),
        ]
        .spacing(7)
        .align_y(Alignment::Center);

        let mut file_list = column![].spacing(5);
        for (path, cat) in files {
            let color = severity_color(s, severity_of(*cat));
            file_list = file_list.push(
                row![
                    chip(category_label(*cat), color),
                    label(path.clone(), 12.5, Weight::Normal, s.muted),
                ]
                .spacing(9)
                .align_y(Alignment::Center),
            );
        }
        groups = groups.push(column![header, file_list].spacing(7));
    }

    // Inset, attached visually beneath the row card.
    container(groups)
        .padding([14, 16])
        .width(Length::Fill)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(s.surface_sunken)),
            border: Border {
                radius: 12.0.into(),
                width: 1.0,
                color: s.border,
            },
            ..container::Style::default()
        })
        .into()
}

// ---------------------------------------------------------------------------
// Skin — hand-tuned palette, theme-aware
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Skin {
    bg: Color,
    surface: Color,
    surface_hover: Color,
    surface_sunken: Color,
    border: Color,
    text: Color,
    muted: Color,
    faint: Color,
    icon: Color,
    primary: Color,
    success: Color,
    warn: Color,
    danger: Color,
    info: Color,
}

fn skin(dark: bool) -> Skin {
    if dark {
        Skin {
            bg: rgb(0x0D, 0x0F, 0x14),
            surface: rgb(0x16, 0x19, 0x22),
            surface_hover: rgb(0x1E, 0x22, 0x2E),
            surface_sunken: rgb(0x10, 0x13, 0x1A),
            border: rgb(0x27, 0x2C, 0x38),
            text: rgb(0xE8, 0xEA, 0xF0),
            muted: rgb(0x9A, 0xA0, 0xAD),
            faint: rgb(0x69, 0x6F, 0x7C),
            icon: rgb(0xAD, 0xB3, 0xBF),
            primary: rgb(0x70, 0x7C, 0xF2),
            success: rgb(0x37, 0xD3, 0x99),
            warn: rgb(0xF5, 0xA6, 0x23),
            danger: rgb(0xF2, 0x6D, 0x78),
            info: rgb(0x5B, 0xA8, 0xF5),
        }
    } else {
        Skin {
            bg: rgb(0xF4, 0xF5, 0xF8),
            surface: rgb(0xFF, 0xFF, 0xFF),
            surface_hover: rgb(0xEF, 0xF1, 0xF5),
            surface_sunken: rgb(0xFA, 0xFB, 0xFD),
            border: rgb(0xE3, 0xE6, 0xEB),
            text: rgb(0x1A, 0x1E, 0x27),
            muted: rgb(0x5C, 0x63, 0x70),
            faint: rgb(0x95, 0x9B, 0xA6),
            icon: rgb(0x53, 0x5B, 0x69),
            primary: rgb(0x55, 0x60, 0xE8),
            success: rgb(0x10, 0x8A, 0x5E),
            warn: rgb(0xB0, 0x72, 0x12),
            danger: rgb(0xCE, 0x3B, 0x49),
            info: rgb(0x2C, 0x77, 0xC9),
        }
    }
}

// ---------------------------------------------------------------------------
// Widget helpers
// ---------------------------------------------------------------------------

fn ui_font(weight: Weight) -> iced::Font {
    iced::Font {
        family: Family::Name("Inter"),
        weight,
        ..iced::Font::DEFAULT
    }
}

fn label<'a>(s: String, size: f32, weight: Weight, color: Color) -> iced::widget::Text<'a> {
    text(s)
        .size(size)
        .font(ui_font(weight))
        .style(move |_: &Theme| text::Style { color: Some(color) })
}

fn icon<'a>(src: &'static str, size: f32, color: Color) -> Element<'a, Message> {
    svg(svg::Handle::from_memory(src.as_bytes()))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |_: &Theme, _| svg::Style { color: Some(color) })
        .into()
}

/// Soft, rounded badge: tinted background + saturated icon/text.
fn pill<'a>(icon_src: &'static str, lbl: String, color: Color) -> Element<'a, Message> {
    container(
        row![
            icon(icon_src, 13.0, color),
            label(lbl, 12.0, Weight::Medium, color),
        ]
        .spacing(5)
        .align_y(Alignment::Center),
    )
    .padding([4, 9])
    .style(move |_: &Theme| container::Style {
        background: Some(Background::Color(alpha(color, 0.14))),
        border: Border {
            radius: 999.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    })
    .into()
}

/// Tiny category chip used inside the conflict detail.
fn chip<'a>(lbl: &str, color: Color) -> Element<'a, Message> {
    container(label(lbl.to_string(), 11.0, Weight::Medium, color))
        .padding([2, 7])
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(alpha(color, 0.16))),
            border: Border {
                radius: 6.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

fn ghost_icon_btn<'a>(s: Skin, src: &'static str, msg: Message) -> Element<'a, Message> {
    button(icon(src, 14.0, s.muted))
        .padding(3)
        .on_press(msg)
        .style(move |_: &Theme, status| {
            let bg = match status {
                button::Status::Hovered | button::Status::Pressed => {
                    Some(Background::Color(s.surface_hover))
                }
                _ => None,
            };
            button::Style {
                background: bg,
                border: Border {
                    radius: 7.0.into(),
                    ..Border::default()
                },
                ..button::Style::default()
            }
        })
        .into()
}

fn card_style(s: Skin, dragging: bool) -> container::Style {
    container::Style {
        background: Some(Background::Color(if dragging {
            s.surface_hover
        } else {
            s.surface
        })),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: if dragging { s.primary } else { s.border },
        },
        shadow: Shadow {
            color: Color {
                a: if dragging { 0.35 } else { 0.18 },
                ..Color::BLACK
            },
            offset: Vector::new(0.0, if dragging { 6.0 } else { 2.0 }),
            blur_radius: if dragging { 18.0 } else { 8.0 },
        },
        ..container::Style::default()
    }
}

fn divider<'a>(color: Color) -> Element<'a, Message> {
    container(Space::with_height(Length::Fixed(1.0)))
        .width(Length::Fill)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(color)),
            ..container::Style::default()
        })
        .into()
}

fn meta_dot<'a>(color: Color) -> Element<'a, Message> {
    container(Space::new(Length::Fixed(5.0), Length::Fixed(5.0)))
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(color)),
            border: Border {
                radius: 999.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}

fn alpha(c: Color, a: f32) -> Color {
    Color { a, ..c }
}

// ---------------------------------------------------------------------------
// Severity
// ---------------------------------------------------------------------------

fn severity_of(cat: ConflictCategory) -> Severity {
    match cat {
        ConflictCategory::Defines => Severity::High,
        ConflictCategory::GameData
        | ConflictCategory::Events
        | ConflictCategory::Map
        | ConflictCategory::Other => Severity::Medium,
        ConflictCategory::Localisation | ConflictCategory::Gfx | ConflictCategory::Sound => {
            Severity::Low
        }
    }
}

fn worst_severity(c: &ConflictsForMod) -> Severity {
    let mut worst = Severity::Low;
    for files in c.values() {
        for (_, cat) in files {
            match severity_of(*cat) {
                Severity::High => return Severity::High,
                Severity::Medium => worst = Severity::Medium,
                Severity::Low => {}
            }
        }
    }
    worst
}

fn severity_color(s: Skin, sev: Severity) -> Color {
    match sev {
        Severity::High => s.danger,
        Severity::Medium => s.warn,
        Severity::Low => s.info,
    }
}

fn category_label(c: ConflictCategory) -> &'static str {
    match c {
        ConflictCategory::Defines => "defines",
        ConflictCategory::GameData => "game data",
        ConflictCategory::Localisation => "localisation",
        ConflictCategory::Events => "events",
        ConflictCategory::Gfx => "gfx",
        ConflictCategory::Map => "map",
        ConflictCategory::Sound => "sound",
        ConflictCategory::Other => "other",
    }
}

// ---------------------------------------------------------------------------
// Lucide icons (ISC licensed) — inlined SVG
// ---------------------------------------------------------------------------

const I_GRIP: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor"><circle cx="9" cy="5" r="1.5"/><circle cx="9" cy="12" r="1.5"/><circle cx="9" cy="19" r="1.5"/><circle cx="15" cy="5" r="1.5"/><circle cx="15" cy="12" r="1.5"/><circle cx="15" cy="19" r="1.5"/></svg>"#;
const I_CHEV_UP: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="m18 15-6-6-6 6"/></svg>"#;
const I_CHEV_DOWN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>"#;
const I_CHEV_RIGHT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="m9 18 6-6-6-6"/></svg>"#;
const I_ALERT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/><path d="M12 9v4"/><path d="M12 17h.01"/></svg>"#;
const I_CHECK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>"#;
const I_X: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>"#;
const I_MOON: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"/></svg>"#;
const I_SUN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/></svg>"#;
const I_SWORDS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="14.5 17.5 3 6 3 3 6 3 17.5 14.5"/><line x1="13" x2="19" y1="19" y2="13"/><line x1="16" x2="20" y1="16" y2="20"/><line x1="19" x2="21" y1="21" y2="19"/></svg>"#;

// ---------------------------------------------------------------------------
// Core wiring — direct in-process calls, no IPC.
// ---------------------------------------------------------------------------

fn load_from_core() -> (String, Vec<ModRow>, HashMap<String, ConflictsForMod>) {
    let games = match detect_games() {
        Ok(g) => g,
        Err(_) => return ("No games detected".into(), vec![], HashMap::new()),
    };
    let Some(game) = games.first() else {
        return ("No games detected".into(), vec![], HashMap::new());
    };

    let mods = discover_mods(game);
    let statuses = achievement_status_for_mods(&mods);

    let rows: Vec<ModRow> = mods
        .iter()
        .zip(statuses.iter())
        .map(|(m, s)| ModRow {
            name: m.name.clone().unwrap_or_else(|| "<unnamed>".into()),
            mod_id: m.mod_id().to_string(),
            enabled: true,
            achievement_ok: s.compatible,
            achievement_categories: s.gameplay_categories.clone(),
        })
        .collect();

    // `conflict_detection` consumes the descriptors, so re-discover for it.
    let conflicts_raw = conflict_detection(discover_mods(game));
    let mut conflicts: HashMap<String, ConflictsForMod> = HashMap::new();
    for c in conflicts_raw {
        let file = c.file_path.to_string_lossy().to_string();
        for a in &c.mod_list {
            for b in &c.mod_list {
                if a == b {
                    continue;
                }
                conflicts
                    .entry(a.clone())
                    .or_default()
                    .entry(b.clone())
                    .or_default()
                    .push((file.clone(), c.category));
            }
        }
    }

    (game.game_name.clone(), rows, conflicts)
}
