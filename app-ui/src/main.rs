//! Native Iced desktop UI for Ferrous Mod Manager.
//!
//! Replaces the Svelte + Tauri webview. Talks to the `ferrous-mod-manager`
//! core library DIRECTLY via in-process calls — no IPC, no hand-maintained
//! TypeScript type mirror.
//!
//! Layout mirrors the previous app: a top bar (game + collection selectors,
//! apply/launch), a two-pane main area (installed mods <-> active collection),
//! and a status bar. Conflicts and achievement/ironman status come straight
//! from the core and are recomputed when the active loadout changes.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Instant;

use iced::font::{Family, Weight};
use iced::widget::{
    Space, button, checkbox, column, container, mouse_area, pick_list, row, scrollable, svg, text,
    text_input,
};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Shadow, Subscription, Task, Theme,
    Vector,
};

use ferrous_mod_manager::achievements::achievement_status_for_mods;
use ferrous_mod_manager::collections::{
    apply_mod_collection_for_game, create_collection_for_game, delete_collection_for_game,
    import_collection_for_game, load_or_create_collections_for_game, save_collection_for_game,
};
use ferrous_mod_manager::conflict::conflict_detection;
use ferrous_mod_manager::detector::{detect_games, discover_mods};
use ferrous_mod_manager::launch::launch_game;
use ferrous_mod_manager::models::{
    ConflictCategory, DetectedGame, ModCollection, ModDescriptor, ModEntry,
};

fn main() -> iced::Result {
    iced::application("Ferrous Mod Manager", App::update, App::view)
        .theme(App::theme)
        .subscription(App::subscription)
        // Static per-weight instances: cosmic-text/fontdb only matches discrete
        // faces by weight, so a single variable font would render every
        // non-400 label in the serif fallback. Medium/SemiBold register under
        // the typographic family ("Exo 2") via their name table.
        .font(include_bytes!("../assets/fonts/Exo2-Regular.ttf").as_slice())
        .font(include_bytes!("../assets/fonts/Exo2-Medium.ttf").as_slice())
        .font(include_bytes!("../assets/fonts/Exo2-SemiBold.ttf").as_slice())
        .font(include_bytes!("../assets/fonts/Exo2-Bold.ttf").as_slice())
        .default_font(ui_font(Weight::Normal))
        .window(iced::window::Settings {
            size: iced::Size::new(1280.0, 820.0),
            // Below this the two-pane layout collapses and content vanishes.
            min_size: Some(iced::Size::new(900.0, 600.0)),
            ..Default::default()
        })
        .run_with(App::new)
}

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

/// Resolved display info for one installed mod.
struct Installed {
    mod_id: String,
    name: String,
    /// Single-line secondary text (workshop id, or local mod's basename).
    subtitle: String,
    workshop: bool,
}

/// For a given mod: opposing-mod-name -> files they collide on (with category).
type ConflictsForMod = BTreeMap<String, Vec<(String, ConflictCategory)>>;

#[derive(Clone, Copy, PartialEq)]
enum Severity {
    High,
    Medium,
    Low,
}

#[derive(Clone, Copy)]
enum ToastKind {
    Error,
    Success,
}

struct Toast {
    message: String,
    kind: ToastKind,
}

/// Which top-level screen is shown.
#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Main,
    Collections,
}

struct App {
    games: Vec<DetectedGame>,
    selected_game: usize,

    // Per-selected-game data.
    descriptors: Vec<ModDescriptor>,
    installed: Vec<Installed>,
    /// mod_id -> achievements compatible.
    achievement_ok: HashMap<String, bool>,
    /// mod_id -> checksum-affecting categories (why achievements break).
    achievement_categories: HashMap<String, Vec<ConflictCategory>>,
    collections: Vec<ModCollection>,
    selected_collection: usize,
    /// Conflicts among the active collection's *enabled* mods, keyed by mod name.
    conflicts: HashMap<String, ConflictsForMod>,

    // UI state.
    screen: Screen,
    dark: bool,
    filter: String,
    new_collection: String,
    /// (index being renamed, edit buffer) on the collections screen.
    renaming: Option<(usize, String)>,
    /// Collection ids (as strings) checked for bulk delete on the collections screen.
    marked: HashSet<String>,
    dragging: Option<usize>,
    expanded: Option<usize>,
    /// Installed-list scroll offset (px) and last-known viewport height (px).
    installed_scroll: f32,
    installed_view_h: f32,
    toast: Option<Toast>,
    /// Seconds the current toast has been visible (drives fade in/out).
    toast_age: f32,
    /// Toast message last seen, to detect when a fresh toast appears.
    toast_key: Option<String>,
    /// Timestamp of the previous animation frame.
    last_tick: Option<Instant>,
}

#[derive(Debug, Clone)]
enum Message {
    SelectGame(String),
    SelectCollection(String),
    SelectCollectionAt(usize),
    OpenCollections,
    CloseCollections,
    NewCollectionChanged(String),
    CreateCollection,
    ToggleMark(String),
    DeleteMarked,
    ImportCollection,
    ExportCollection(usize),
    RenameStart(usize),
    RenameBufferChanged(String),
    RenameCommit,
    RenameCancel,
    ToggleTheme,
    FilterChanged(String),
    AddToCollection(String),
    RemoveFromCollection(String),
    ToggleEnabled(usize),
    MoveUp(usize),
    MoveDown(usize),
    DragStart(usize),
    DragEnterRow(usize),
    DragEnd,
    ToggleExpanded(usize),
    EnableAll,
    DisableAll,
    AddAll,
    RemoveAll,
    Apply,
    Launch,
    DismissToast,
    Tick(Instant),
    /// Installed list scrolled: (absolute y offset, viewport height).
    InstalledScrolled(f32, f32),
}

// Installed-list virtualization: fixed slot height + how many extra rows to
// render above/below the viewport so fast scrolls don't flash blank.
const INSTALLED_ROW_H: f32 = 72.0;
const INSTALLED_OVERSCAN: usize = 4;

// Toast animation timing (seconds).
const TOAST_FADE_IN: f32 = 0.22;
const TOAST_FADE_OUT: f32 = 0.45;
const TOAST_HOLD: f32 = 3.2;
const TOAST_TOTAL: f32 = TOAST_FADE_IN + TOAST_HOLD + TOAST_FADE_OUT;

impl App {
    fn new() -> (Self, Task<Message>) {
        let games = detect_games().unwrap_or_default();
        let mut app = App {
            games,
            selected_game: 0,
            descriptors: vec![],
            installed: vec![],
            achievement_ok: HashMap::new(),
            achievement_categories: HashMap::new(),
            collections: vec![],
            selected_collection: 0,
            conflicts: HashMap::new(),
            screen: Screen::Main,
            dark: true,
            filter: String::new(),
            new_collection: String::new(),
            renaming: None,
            marked: HashSet::new(),
            dragging: None,
            expanded: None,
            installed_scroll: 0.0,
            // Generous default so the first paint (before any scroll event)
            // fills the pane; corrected on the first real scroll.
            installed_view_h: 820.0,
            toast: None,
            toast_age: 0.0,
            toast_key: None,
            last_tick: None,
        };
        if !app.games.is_empty() {
            app.load_game();
        }
        (app, Task::none())
    }

    /// Drive toast fade in/out by requesting animation frames only while a
    /// toast is on screen; otherwise the app stays idle (event-driven).
    fn subscription(&self) -> Subscription<Message> {
        if self.toast.is_some() {
            iced::window::frames().map(Message::Tick)
        } else {
            Subscription::none()
        }
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

    // ----- data loading -----------------------------------------------------

    fn load_game(&mut self) {
        let (descriptors, app_id) = {
            let game = &self.games[self.selected_game];
            (discover_mods(game), game.app_id)
        };
        let statuses = achievement_status_for_mods(&descriptors);
        self.achievement_ok = statuses
            .iter()
            .map(|s| (s.mod_id.clone(), s.compatible))
            .collect();
        self.achievement_categories = statuses
            .into_iter()
            .map(|s| (s.mod_id, s.gameplay_categories))
            .collect();
        self.installed = descriptors
            .iter()
            .map(|d| {
                let mod_id = d.mod_id().to_string();
                let workshop = d.remote_file_id.is_some();
                // Keep the secondary line short and single-line so installed
                // rows have a uniform height (required by virtualization):
                // workshop id verbatim, local mods reduced to their basename.
                let subtitle = if workshop {
                    format!("#{mod_id}")
                } else {
                    std::path::Path::new(&mod_id)
                        .file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| mod_id.clone())
                };
                Installed {
                    mod_id,
                    name: d.name.clone().unwrap_or_else(|| "<unnamed>".into()),
                    subtitle,
                    workshop,
                }
            })
            .collect();
        self.descriptors = descriptors;
        self.collections = match load_or_create_collections_for_game(app_id) {
            Ok(cs) => cs,
            Err(e) => {
                self.toast = Some(Toast {
                    message: format!("Could not initialize collections: {e}"),
                    kind: ToastKind::Error,
                });
                Vec::new()
            }
        };
        self.selected_collection = 0;
        self.renaming = None;
        self.marked.clear();
        self.expanded = None;
        self.installed_scroll = 0.0;
        self.recompute_conflicts();
    }

    fn recompute_conflicts(&mut self) {
        let enabled: HashSet<String> = match self.collections.get(self.selected_collection) {
            Some(c) => c
                .mods
                .iter()
                .filter(|m| m.enabled)
                .map(|m| m.mod_id.clone())
                .collect(),
            None => HashSet::new(),
        };
        if enabled.is_empty() {
            self.conflicts.clear();
            return;
        }
        let mods: Vec<ModDescriptor> = self
            .descriptors
            .iter()
            .filter(|d| enabled.contains(d.mod_id()))
            .cloned()
            .collect();

        let mut map: HashMap<String, ConflictsForMod> = HashMap::new();
        for c in conflict_detection(mods) {
            let file = c.file_path.to_string_lossy().to_string();
            for a in &c.mod_list {
                for b in &c.mod_list {
                    if a == b {
                        continue;
                    }
                    map.entry(a.clone())
                        .or_default()
                        .entry(b.clone())
                        .or_default()
                        .push((file.clone(), c.category));
                }
            }
        }
        self.conflicts = map;
    }

    /// Persist the active collection and refresh derived conflict data.
    fn save_active(&mut self) {
        if let Some(c) = self.collections.get(self.selected_collection) {
            let app_id = self.games[self.selected_game].app_id;
            if let Err(e) = save_collection_for_game(app_id, c) {
                self.toast = Some(Toast {
                    message: format!("Save failed: {e}"),
                    kind: ToastKind::Error,
                });
            }
        }
        self.recompute_conflicts();
    }

    // ----- update -----------------------------------------------------------

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectGame(name) => {
                if let Some(i) = self.games.iter().position(|g| g.game_name == name) {
                    self.selected_game = i;
                    self.filter.clear();
                    self.load_game();
                }
            }
            Message::SelectCollection(name) => {
                if let Some(i) = self.collections.iter().position(|c| c.name == name) {
                    self.selected_collection = i;
                    self.expanded = None;
                    self.recompute_conflicts();
                }
            }
            Message::NewCollectionChanged(v) => self.new_collection = v,
            Message::CreateCollection => {
                let name = self.new_collection.trim().to_string();
                if !name.is_empty() {
                    let app_id = self.games[self.selected_game].app_id;
                    match create_collection_for_game(app_id, name) {
                        Ok(c) => {
                            self.collections.push(c);
                            self.selected_collection = self.collections.len() - 1;
                            self.new_collection.clear();
                            self.recompute_conflicts();
                        }
                        Err(e) => {
                            self.toast = Some(Toast {
                                message: format!("Create failed: {e}"),
                                kind: ToastKind::Error,
                            })
                        }
                    }
                }
            }
            Message::SelectCollectionAt(i) => {
                if i < self.collections.len() {
                    self.selected_collection = i;
                    self.expanded = None;
                    self.recompute_conflicts();
                }
            }
            Message::OpenCollections => {
                self.renaming = None;
                self.marked.clear();
                self.screen = Screen::Collections;
            }
            Message::CloseCollections => {
                self.renaming = None;
                self.marked.clear();
                self.screen = Screen::Main;
            }
            Message::ToggleMark(id) => {
                if !self.marked.remove(&id) {
                    self.marked.insert(id);
                }
            }
            Message::DeleteMarked => {
                // Always keep at least one collection; refuse a wipe-all.
                if self.marked.is_empty() {
                    // nothing to do
                } else if self.marked.len() >= self.collections.len() {
                    self.toast = Some(Toast {
                        message: "At least one collection must remain.".to_string(),
                        kind: ToastKind::Error,
                    });
                } else {
                    let app_id = self.games[self.selected_game].app_id;
                    let active_id = self
                        .collections
                        .get(self.selected_collection)
                        .map(|c| c.id.to_string());
                    let mut failed = 0;
                    for c in &self.collections {
                        if self.marked.contains(&c.id.to_string())
                            && delete_collection_for_game(app_id, c.id).is_err()
                        {
                            failed += 1;
                        }
                    }
                    self.collections
                        .retain(|c| !self.marked.contains(&c.id.to_string()));
                    // Restore the active selection by id; fall back to the first.
                    self.selected_collection = active_id
                        .and_then(|id| self.collections.iter().position(|c| c.id.to_string() == id))
                        .unwrap_or(0);
                    self.marked.clear();
                    self.renaming = None;
                    self.expanded = None;
                    self.recompute_conflicts();
                    if failed > 0 {
                        self.toast = Some(Toast {
                            message: format!("{failed} collection(s) could not be deleted"),
                            kind: ToastKind::Error,
                        });
                    }
                }
            }
            Message::ImportCollection => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Collection JSON", &["json"])
                    .set_title("Import collection")
                    .pick_file()
                {
                    let app_id = self.games[self.selected_game].app_id;
                    match import_collection_for_game(app_id, &path) {
                        Ok(c) => {
                            self.collections.push(c);
                            self.selected_collection = self.collections.len() - 1;
                            self.recompute_conflicts();
                            self.toast = Some(Toast {
                                message: "Collection imported".to_string(),
                                kind: ToastKind::Success,
                            });
                        }
                        Err(e) => {
                            self.toast = Some(Toast {
                                message: format!("Import failed: {e}"),
                                kind: ToastKind::Error,
                            });
                        }
                    }
                }
            }
            Message::ExportCollection(i) => {
                if let Some(c) = self.collections.get(i) {
                    let default_name = format!("{}.json", sanitize_filename(&c.name));
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Collection JSON", &["json"])
                        .set_file_name(default_name)
                        .set_title("Export collection")
                        .save_file()
                    {
                        self.toast = Some(match c.save(&path) {
                            Ok(()) => Toast {
                                message: format!("Exported \"{}\"", c.name),
                                kind: ToastKind::Success,
                            },
                            Err(e) => Toast {
                                message: format!("Export failed: {e}"),
                                kind: ToastKind::Error,
                            },
                        });
                    }
                }
            }
            Message::RenameStart(i) => {
                if let Some(c) = self.collections.get(i) {
                    self.renaming = Some((i, c.name.clone()));
                }
            }
            Message::RenameBufferChanged(v) => {
                if let Some((_, buf)) = self.renaming.as_mut() {
                    *buf = v;
                }
            }
            Message::RenameCommit => {
                if let Some((i, buf)) = self.renaming.take() {
                    let name = buf.trim().to_string();
                    if !name.is_empty()
                        && let Some(c) = self.collections.get_mut(i)
                    {
                        c.name = name;
                        let app_id = self.games[self.selected_game].app_id;
                        if let Err(e) = save_collection_for_game(app_id, c) {
                            self.toast = Some(Toast {
                                message: format!("Rename failed: {e}"),
                                kind: ToastKind::Error,
                            });
                        }
                    }
                }
            }
            Message::RenameCancel => self.renaming = None,
            Message::ToggleTheme => self.dark = !self.dark,
            Message::FilterChanged(v) => {
                self.filter = v;
                // Filtered length changes; start from the top so the virtual
                // window doesn't point past the new end.
                self.installed_scroll = 0.0;
            }
            Message::InstalledScrolled(offset, height) => {
                self.installed_scroll = offset;
                self.installed_view_h = height;
            }
            Message::AddToCollection(mod_id) => {
                if let Some(c) = self.collections.get_mut(self.selected_collection)
                    && !c.mods.iter().any(|m| m.mod_id == mod_id)
                {
                    c.mods.push(ModEntry {
                        mod_id,
                        enabled: true,
                    });
                }
                self.save_active();
            }
            Message::RemoveFromCollection(mod_id) => {
                if let Some(c) = self.collections.get_mut(self.selected_collection) {
                    c.mods.retain(|m| m.mod_id != mod_id);
                }
                self.expanded = None;
                self.save_active();
            }
            Message::ToggleEnabled(i) => {
                if let Some(c) = self.collections.get_mut(self.selected_collection)
                    && let Some(e) = c.mods.get_mut(i)
                {
                    e.enabled = !e.enabled;
                }
                self.save_active();
            }
            Message::MoveUp(i) => {
                if i > 0
                    && let Some(c) = self.collections.get_mut(self.selected_collection)
                {
                    c.mods.swap(i, i - 1);
                    self.expanded = None;
                    self.save_active();
                }
            }
            Message::MoveDown(i) => {
                if let Some(c) = self.collections.get_mut(self.selected_collection)
                    && i + 1 < c.mods.len()
                {
                    c.mods.swap(i, i + 1);
                    self.expanded = None;
                    self.save_active();
                }
            }
            Message::DragStart(i) => self.dragging = Some(i),
            Message::DragEnterRow(j) => {
                if let Some(i) = self.dragging
                    && i != j
                    && let Some(c) = self.collections.get_mut(self.selected_collection)
                    && j < c.mods.len()
                {
                    let entry = c.mods.remove(i);
                    c.mods.insert(j, entry);
                    self.dragging = Some(j);
                    self.expanded = None;
                    self.save_active();
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
            Message::EnableAll => {
                if let Some(c) = self.collections.get_mut(self.selected_collection) {
                    for e in &mut c.mods {
                        e.enabled = true;
                    }
                }
                self.save_active();
            }
            Message::DisableAll => {
                if let Some(c) = self.collections.get_mut(self.selected_collection) {
                    for e in &mut c.mods {
                        e.enabled = false;
                    }
                }
                self.save_active();
            }
            Message::AddAll => {
                let ids: Vec<String> = self.installed.iter().map(|m| m.mod_id.clone()).collect();
                if let Some(c) = self.collections.get_mut(self.selected_collection) {
                    let existing: HashSet<String> =
                        c.mods.iter().map(|m| m.mod_id.clone()).collect();
                    for id in ids {
                        if !existing.contains(&id) {
                            c.mods.push(ModEntry {
                                mod_id: id,
                                enabled: true,
                            });
                        }
                    }
                }
                self.save_active();
            }
            Message::RemoveAll => {
                if let Some(c) = self.collections.get_mut(self.selected_collection) {
                    c.mods.clear();
                }
                self.expanded = None;
                self.save_active();
            }
            Message::Apply => {
                let game = &self.games[self.selected_game];
                if let Some(c) = self.collections.get(self.selected_collection) {
                    self.toast = Some(match apply_mod_collection_for_game(game, c) {
                        Ok(()) => Toast {
                            message: format!("Applied \"{}\" to {}", c.name, game.game_name),
                            kind: ToastKind::Success,
                        },
                        Err(e) => Toast {
                            message: format!("Apply failed: {e}"),
                            kind: ToastKind::Error,
                        },
                    });
                }
            }
            Message::Launch => {
                let game = &self.games[self.selected_game];
                // Apply the active collection first so the game boots with it,
                // then hand off to Steam. Surface an apply failure and stop.
                let apply_err = self
                    .collections
                    .get(self.selected_collection)
                    .and_then(|c| apply_mod_collection_for_game(game, c).err());
                self.toast = Some(match apply_err {
                    Some(e) => Toast {
                        message: format!("Apply failed, not launching: {e}"),
                        kind: ToastKind::Error,
                    },
                    None => match launch_game(game.app_id) {
                        Ok(()) => Toast {
                            message: format!("Launching {}…", game.game_name),
                            kind: ToastKind::Success,
                        },
                        Err(e) => Toast {
                            message: format!("Launch failed: {e}"),
                            kind: ToastKind::Error,
                        },
                    },
                });
            }
            Message::DismissToast => self.toast = None,
            Message::Tick(now) => {
                let dt = self
                    .last_tick
                    .map(|prev| (now - prev).as_secs_f32())
                    .unwrap_or(0.0)
                    // Clamp so a long idle gap can't jump the toast straight out.
                    .min(0.1);
                self.last_tick = Some(now);
                self.toast_age += dt;
                if self.toast_age >= TOAST_TOTAL {
                    self.toast = None;
                }
            }
        }

        // Detect a freshly shown toast (set by any arm above) and restart its
        // animation clock — avoids resetting at every individual toast site.
        let key = self.toast.as_ref().map(|t| t.message.clone());
        if key != self.toast_key {
            self.toast_key = key;
            self.toast_age = 0.0;
            self.last_tick = None;
        }

        Task::none()
    }

    // ----- view -------------------------------------------------------------

    fn view(&self) -> Element<'_, Message> {
        let s = skin(self.dark);

        if self.games.is_empty() {
            return container(label(
                "No Paradox games detected.".into(),
                16.0,
                Weight::Medium,
                s.muted,
            ))
            .center(Length::Fill)
            .style(move |_: &Theme| bg_style(s.bg))
            .into();
        }

        match self.screen {
            Screen::Main => self.main_screen(s),
            Screen::Collections => self.collections_screen(s),
        }
    }

    fn main_screen(&self, s: Skin) -> Element<'_, Message> {
        let mut content = column![self.top_bar(s), divider(s.border)].spacing(16);

        let main = row![self.installed_pane(s), self.collection_pane(s)]
            .spacing(16)
            .height(Length::Fill);
        // Catch the pointer release anywhere in the work area so a drag-reorder
        // ends even if the cursor left the row it started on.
        content = content.push(mouse_area(main).on_release(Message::DragEnd));

        if let Some(t) = &self.toast {
            content = content.push(toast_banner(s, t, self.toast_age));
        }
        content = content.push(self.status_bar(s));

        container(content)
            .padding(24)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_: &Theme| bg_style(s.bg))
            .into()
    }

    fn top_bar(&self, s: Skin) -> Element<'_, Message> {
        let game_names: Vec<String> = self.games.iter().map(|g| g.game_name.clone()).collect();
        let current_game = self
            .games
            .get(self.selected_game)
            .map(|g| g.game_name.clone());
        let game_sel = pick_list(game_names, current_game, Message::SelectGame)
            .text_size(14.0)
            .padding([7, 11]);

        let col_names: Vec<String> = self.collections.iter().map(|c| c.name.clone()).collect();
        let current_col = self
            .collections
            .get(self.selected_collection)
            .map(|c| c.name.clone());
        let col_sel = pick_list(col_names, current_col, Message::SelectCollection)
            .placeholder("No collections")
            .text_size(14.0)
            .padding([7, 11]);

        let manage_btn = ghost_button(s, I_LAYERS, "Manage", Message::OpenCollections);

        let apply_btn = primary_button(s, I_CHECK, "Apply", Message::Apply);
        let launch_btn = ghost_button(s, I_PLAY, "Launch", Message::Launch);

        let (theme_icon, theme_lbl) = if self.dark {
            (I_MOON, "Dark")
        } else {
            (I_SUN, "Light")
        };
        let theme_btn = ghost_button(s, theme_icon, theme_lbl, Message::ToggleTheme);

        row![
            game_sel,
            vsep(s.border),
            col_sel,
            manage_btn,
            Space::with_width(Length::Fill),
            apply_btn,
            launch_btn,
            theme_btn,
        ]
        .spacing(9)
        .align_y(Alignment::Center)
        .into()
    }

    fn installed_pane(&self, s: Skin) -> Element<'_, Message> {
        let in_collection: HashSet<&str> = match self.collections.get(self.selected_collection) {
            Some(c) => c.mods.iter().map(|m| m.mod_id.as_str()).collect(),
            None => HashSet::new(),
        };
        let needle = self.filter.to_lowercase();

        let header = row![
            label("Installed".into(), 15.0, Weight::Semibold, s.text),
            label(
                format!("{}", self.installed.len()),
                12.0,
                Weight::Medium,
                s.faint
            ),
            Space::with_width(Length::Fill),
            ghost_button(s, I_PLUS, "Add all", Message::AddAll),
            ghost_button(s, I_MINUS, "Remove all", Message::RemoveAll),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let search = text_input("Filter mods…", &self.filter)
            .on_input(Message::FilterChanged)
            .padding([8, 11])
            .size(13.0);

        // Virtualize: only build the rows inside (and just around) the viewport,
        // padding above/below with fixed-height spacers so the scrollbar and
        // offsets behave as if the whole list were present.
        let filtered: Vec<&Installed> = self
            .installed
            .iter()
            .filter(|m| {
                needle.is_empty()
                    || m.name.to_lowercase().contains(&needle)
                    || m.mod_id.to_lowercase().contains(&needle)
            })
            .collect();
        let n = filtered.len();
        let row_h = INSTALLED_ROW_H;
        let first =
            ((self.installed_scroll / row_h).floor() as usize).saturating_sub(INSTALLED_OVERSCAN);
        let span = (self.installed_view_h / row_h).ceil() as usize + INSTALLED_OVERSCAN * 2;
        let end = (first + span).min(n);
        let top_pad = first as f32 * row_h;
        let bottom_pad = n.saturating_sub(end) as f32 * row_h;

        let mut list = column![Space::with_height(Length::Fixed(top_pad))].spacing(0);
        for m in &filtered[first..end] {
            let added = in_collection.contains(m.mod_id.as_str());
            list = list.push(
                container(self.installed_row(s, m, added))
                    .center_y(Length::Fixed(row_h))
                    .clip(true),
            );
        }
        list = list.push(Space::with_height(Length::Fixed(bottom_pad)));

        let body = scrollable(list)
            .height(Length::Fill)
            .on_scroll(|vp| Message::InstalledScrolled(vp.absolute_offset().y, vp.bounds().height));
        pane(s, column![header, search, body].spacing(12))
    }

    fn installed_row(&self, s: Skin, m: &Installed, added: bool) -> Element<'_, Message> {
        let source = if m.workshop {
            pill_plain("workshop", s.primary)
        } else {
            pill_plain("local", s.muted)
        };

        let ach: Element<'_, Message> = match self.achievement_ok.get(&m.mod_id) {
            Some(false) => pill_icon(I_X, "ironman".into(), s.warn),
            Some(true) => pill_icon(I_CHECK, "ironman".into(), s.success),
            None => Space::with_width(0).into(),
        };

        let toggle = if added {
            soft_icon_button(
                s,
                I_MINUS,
                Message::RemoveFromCollection(m.mod_id.clone()),
                s.danger,
            )
        } else {
            soft_icon_button(
                s,
                I_PLUS,
                Message::AddToCollection(m.mod_id.clone()),
                s.success,
            )
        };

        let main = row![
            column![
                label(m.name.clone(), 14.0, Weight::Semibold, s.text)
                    .wrapping(text::Wrapping::None),
                label(m.subtitle.clone(), 11.5, Weight::Normal, s.faint)
                    .wrapping(text::Wrapping::None),
            ]
            .spacing(2)
            .width(Length::Fill),
            ach,
            source,
            toggle,
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .padding([9, 12]);

        container(main)
            .style(move |_: &Theme| card_style(s, false))
            .into()
    }

    fn collection_pane(&self, s: Skin) -> Element<'_, Message> {
        let active = self.collections.get(self.selected_collection);
        let name = active.map(|c| c.name.clone()).unwrap_or_default();
        let count = active.map(|c| c.mods.len()).unwrap_or(0);

        let header = row![
            label(
                if name.is_empty() {
                    "Collection".into()
                } else {
                    name
                },
                15.0,
                Weight::Semibold,
                s.text
            ),
            label(format!("{count} mods"), 12.0, Weight::Medium, s.faint),
            Space::with_width(Length::Fill),
            ghost_button(s, I_CHECK, "Enable all", Message::EnableAll),
            ghost_button(s, I_X, "Disable all", Message::DisableAll),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let body: Element<'_, Message> = match active {
            Some(c) if !c.mods.is_empty() => {
                let by_id: HashMap<&str, &Installed> = self
                    .installed
                    .iter()
                    .map(|m| (m.mod_id.as_str(), m))
                    .collect();
                let mut list = column![].spacing(8);
                for (i, entry) in c.mods.iter().enumerate() {
                    list = list.push(self.collection_row(s, i, entry, &by_id));
                }
                scrollable(list).height(Length::Fill).into()
            }
            _ => container(label(
                "Add mods from the left, or create a collection.".into(),
                13.0,
                Weight::Normal,
                s.faint,
            ))
            .center(Length::Fill)
            .into(),
        };

        pane(s, column![header, body].spacing(12))
    }

    fn collection_row(
        &self,
        s: Skin,
        i: usize,
        entry: &ModEntry,
        by_id: &HashMap<&str, &Installed>,
    ) -> Element<'_, Message> {
        let dragging = self.dragging == Some(i);
        let expanded = self.expanded == Some(i);
        let name = by_id
            .get(entry.mod_id.as_str())
            .map(|m| m.name.clone())
            .unwrap_or_else(|| entry.mod_id.clone());

        let handle = mouse_area(icon(I_GRIP, 16.0, s.icon))
            .on_press(Message::DragStart(i))
            .interaction(iced::mouse::Interaction::Grab);

        let position = label(format!("{:>2}", i + 1), 13.0, Weight::Medium, s.faint);
        let enabled = checkbox("", entry.enabled)
            .on_toggle(move |_| Message::ToggleEnabled(i))
            .size(17);

        let name_color = if entry.enabled { s.text } else { s.faint };
        let title = label(name.clone(), 15.0, Weight::Semibold, name_color);
        let mod_id = label(format!("#{}", entry.mod_id), 12.0, Weight::Normal, s.faint);

        let ach: Element<'_, Message> = match self.achievement_ok.get(&entry.mod_id) {
            Some(false) => {
                let cats: Vec<&str> = self
                    .achievement_categories
                    .get(&entry.mod_id)
                    .map(|v| v.iter().map(|c| category_label(*c)).collect())
                    .unwrap_or_default();
                pill_icon(I_X, format!("ironman · {}", cats.join(", ")), s.warn)
            }
            Some(true) => pill_icon(I_CHECK, "ironman".into(), s.success),
            None => Space::with_width(0).into(),
        };

        let conflict_cell: Element<'_, Message> = match self.conflicts.get(&name) {
            Some(c) if !c.is_empty() => {
                let files: usize = c.values().map(|v| v.len()).sum();
                let color = severity_color(s, worst_severity(c));
                button(
                    row![
                        icon(I_ALERT, 13.0, color),
                        label(format!("{files} conflicts"), 12.0, Weight::Medium, color),
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
            _ => Space::with_width(0).into(),
        };

        let reorder = column![
            ghost_icon_btn(s, I_CHEV_UP, Message::MoveUp(i)),
            ghost_icon_btn(s, I_CHEV_DOWN, Message::MoveDown(i)),
        ]
        .spacing(1);

        let remove = ghost_icon_btn(s, I_X, Message::RemoveFromCollection(entry.mod_id.clone()));

        // Stack name, location and status vertically so the ironman/conflict
        // indicators no longer steal width from (and truncate) the name/path.
        let has_ach = self.achievement_ok.contains_key(&entry.mod_id);
        let has_conflict = self.conflicts.get(&name).is_some_and(|c| !c.is_empty());
        let mut info = column![title, mod_id].spacing(3).width(Length::Fill);
        if has_ach || has_conflict {
            let mut status = row![].spacing(8).align_y(Alignment::Center);
            if has_ach {
                status = status.push(ach);
            }
            if has_conflict {
                status = status.push(conflict_cell);
            }
            info = info.push(status);
        }

        let main = row![handle, position, enabled, info, reorder, remove]
            .spacing(13)
            .align_y(Alignment::Center)
            .padding([11, 14]);

        let card = container(main).style(move |_: &Theme| card_style(s, dragging));
        let row_area = mouse_area(card).on_enter(Message::DragEnterRow(i));

        if expanded && let Some(c) = self.conflicts.get(&name) {
            return column![row_area, conflict_detail(s, c)].spacing(0).into();
        }
        row_area.into()
    }

    fn status_bar(&self, s: Skin) -> Element<'_, Message> {
        let total = self.installed.len();
        let active = self.collections.get(self.selected_collection);
        let enabled = active
            .map(|c| c.mods.iter().filter(|m| m.enabled).count())
            .unwrap_or(0);

        let (significant, minor) = {
            let mut sig = 0;
            let mut min = 0;
            for c in self.conflicts.values() {
                match worst_severity(c) {
                    Severity::High | Severity::Medium => sig += 1,
                    Severity::Low => min += 1,
                }
            }
            (sig, min)
        };

        let blockers = active
            .map(|c| {
                c.mods
                    .iter()
                    .filter(|m| m.enabled && self.achievement_ok.get(&m.mod_id) == Some(&false))
                    .count()
            })
            .unwrap_or(0);

        let game = self
            .games
            .get(self.selected_game)
            .map(|g| g.game_name.as_str())
            .unwrap_or("");

        let item = |dot: Color, t: String| {
            row![meta_dot(dot), label(t, 12.0, Weight::Medium, s.muted)]
                .spacing(6)
                .align_y(Alignment::Center)
        };

        container(
            row![
                item(s.primary, format!("{enabled}/{total} enabled")),
                item(s.danger, format!("{significant} significant conflicts")),
                item(s.info, format!("{minor} minor conflicts")),
                item(s.warn, format!("{blockers} achievement blockers")),
                Space::with_width(Length::Fill),
                label(game.to_string(), 12.0, Weight::Semibold, s.faint),
            ]
            .spacing(16)
            .align_y(Alignment::Center),
        )
        .padding([8, 14])
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(s.surface)),
            border: Border {
                radius: 10.0.into(),
                width: 1.0,
                color: s.border,
            },
            ..container::Style::default()
        })
        .into()
    }

    // ----- collections management screen ------------------------------------

    fn collections_screen(&self, s: Skin) -> Element<'_, Message> {
        let back = ghost_button(s, I_CHEV_LEFT, "Back", Message::CloseCollections);
        let title = label("Manage Collections".into(), 18.0, Weight::Semibold, s.text);
        let game = self
            .games
            .get(self.selected_game)
            .map(|g| g.game_name.as_str())
            .unwrap_or("");
        let header = row![
            back,
            title,
            Space::with_width(Length::Fill),
            label(game.to_string(), 13.0, Weight::Semibold, s.faint),
        ]
        .spacing(14)
        .align_y(Alignment::Center);

        let new_input = text_input("New collection name…", &self.new_collection)
            .on_input(Message::NewCollectionChanged)
            .on_submit(Message::CreateCollection)
            .padding([8, 11])
            .size(14.0)
            .width(Length::Fixed(300.0));
        let create_btn = primary_button(s, I_PLUS, "Create", Message::CreateCollection);
        let import_btn = ghost_button(s, I_DOWNLOAD, "Import", Message::ImportCollection);

        // Bulk-delete button only when something is marked.
        let delete_marked: Element<'_, Message> = if self.marked.is_empty() {
            Space::with_width(0).into()
        } else {
            danger_button(
                s,
                I_TRASH,
                &format!("Delete {}", self.marked.len()),
                Message::DeleteMarked,
            )
        };

        let create_row = row![
            new_input,
            create_btn,
            import_btn,
            Space::with_width(Length::Fill),
            delete_marked,
        ]
        .spacing(9)
        .align_y(Alignment::Center);

        let mut list = column![].spacing(8);
        for (i, c) in self.collections.iter().enumerate() {
            list = list.push(self.collection_manage_row(s, i, c));
        }
        let body = scrollable(list).height(Length::Fill);

        let mut content = column![header, divider(s.border), create_row, body].spacing(18);
        if let Some(t) = &self.toast {
            content = content.push(toast_banner(s, t, self.toast_age));
        }

        container(content)
            .padding(24)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_: &Theme| bg_style(s.bg))
            .into()
    }

    fn collection_manage_row(&self, s: Skin, i: usize, c: &ModCollection) -> Element<'_, Message> {
        let is_active = self.selected_collection == i;
        let renaming_this = matches!(&self.renaming, Some((ri, _)) if *ri == i);
        let count = c.mods.len();
        let id = c.id.to_string();

        let mark = checkbox("", self.marked.contains(&id))
            .on_toggle({
                let id = id.clone();
                move |_| Message::ToggleMark(id.clone())
            })
            .size(17);

        let name_area: Element<'_, Message> =
            if let Some((_, buf)) = self.renaming.as_ref().filter(|(ri, _)| *ri == i) {
                row![
                    text_input("Collection name…", buf)
                        .on_input(Message::RenameBufferChanged)
                        .on_submit(Message::RenameCommit)
                        .padding([6, 10])
                        .size(14.0)
                        .width(Length::Fixed(300.0)),
                    soft_icon_button(s, I_CHECK, Message::RenameCommit, s.success),
                    soft_icon_button(s, I_X, Message::RenameCancel, s.muted),
                ]
                .spacing(6)
                .align_y(Alignment::Center)
                .into()
            } else {
                let color = if is_active { s.primary } else { s.text };
                mouse_area(label(c.name.clone(), 15.0, Weight::Semibold, color))
                    .on_press(Message::SelectCollectionAt(i))
                    .interaction(iced::mouse::Interaction::Pointer)
                    .into()
            };

        let active_badge: Element<'_, Message> = if is_active {
            pill_plain("active", s.primary)
        } else {
            Space::with_width(0).into()
        };

        let actions: Element<'_, Message> = if renaming_this {
            Space::with_width(0).into()
        } else {
            row![
                ghost_button(s, I_UPLOAD, "Export", Message::ExportCollection(i)),
                ghost_button(s, I_PENCIL, "Rename", Message::RenameStart(i)),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        };

        let main = row![
            mark,
            container(name_area).width(Length::Fill),
            active_badge,
            label(format!("{count} mods"), 12.5, Weight::Medium, s.faint),
            actions,
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .padding([12, 14]);

        container(main)
            .style(move |_: &Theme| card_style(s, false))
            .into()
    }
}

// ---------------------------------------------------------------------------
// Standalone view helpers
// ---------------------------------------------------------------------------

fn pane<'a>(s: Skin, inner: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(inner)
        .padding(16)
        .width(Length::FillPortion(1))
        .height(Length::Fill)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(s.surface_sunken)),
            border: Border {
                radius: 14.0.into(),
                width: 1.0,
                color: s.border,
            },
            ..container::Style::default()
        })
        .into()
}

/// Fade-in/out curve and slide offset for a toast of the given age (seconds).
fn toast_anim(age: f32) -> (f32, f32) {
    let fade_in = (age / TOAST_FADE_IN).clamp(0.0, 1.0);
    let fade_out = ((TOAST_TOTAL - age) / TOAST_FADE_OUT).clamp(0.0, 1.0);
    let opacity = fade_in.min(fade_out);
    // Slide up into place as it fades in.
    let slide = (1.0 - fade_in) * 8.0;
    (opacity, slide)
}

fn toast_banner<'a>(s: Skin, t: &Toast, age: f32) -> Element<'a, Message> {
    let (op, slide) = toast_anim(age);
    let base = match t.kind {
        ToastKind::Error => s.danger,
        ToastKind::Success => s.success,
    };
    let fg = alpha(base, op);
    let lead = if matches!(t.kind, ToastKind::Error) {
        I_ALERT
    } else {
        I_CHECK
    };
    let dismiss = button(icon(I_X, 13.0, fg))
        .padding(4)
        .on_press(Message::DismissToast)
        .style(|_: &Theme, _| button::Style {
            background: None,
            ..button::Style::default()
        });
    let banner = container(
        row![
            icon(lead, 15.0, fg),
            label(t.message.clone(), 13.0, Weight::Medium, fg).width(Length::Fill),
            dismiss,
        ]
        .spacing(9)
        .align_y(Alignment::Center),
    )
    .padding([8, 12])
    .style(move |_: &Theme| container::Style {
        background: Some(Background::Color(alpha(base, 0.14 * op))),
        border: Border {
            radius: 10.0.into(),
            width: 1.0,
            color: alpha(base, 0.4 * op),
        },
        ..container::Style::default()
    });

    // Top spacer shrinks to zero as the toast settles — a subtle upward slide.
    column![Space::with_height(Length::Fixed(slide)), banner]
        .spacing(0)
        .into()
}

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
        family: Family::Name("Exo 2"),
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

fn pill_icon<'a>(icon_src: &'static str, lbl: String, color: Color) -> Element<'a, Message> {
    container(
        row![
            icon(icon_src, 13.0, color),
            label(lbl, 12.0, Weight::Medium, color),
        ]
        .spacing(5)
        .align_y(Alignment::Center),
    )
    .padding([4, 9])
    .style(move |_: &Theme| pill_style(color))
    .into()
}

fn pill_plain<'a>(lbl: &str, color: Color) -> Element<'a, Message> {
    container(label(lbl.to_string(), 11.5, Weight::Medium, color))
        .padding([3, 8])
        .style(move |_: &Theme| pill_style(color))
        .into()
}

fn pill_style(color: Color) -> container::Style {
    container::Style {
        background: Some(Background::Color(alpha(color, 0.14))),
        border: Border {
            radius: 999.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

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

/// Filled primary action button (icon + label).
fn primary_button<'a>(s: Skin, src: &'static str, lbl: &str, msg: Message) -> Element<'a, Message> {
    button(
        row![
            icon(src, 14.0, Color::WHITE),
            label(lbl.to_string(), 13.0, Weight::Semibold, Color::WHITE),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([7, 13])
    .on_press(msg)
    .style(move |_: &Theme, status| {
        let bg = match status {
            button::Status::Hovered | button::Status::Pressed => lighten(s.primary, 0.06),
            _ => s.primary,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: Color::WHITE,
            border: Border {
                radius: 9.0.into(),
                ..Border::default()
            },
            ..button::Style::default()
        }
    })
    .into()
}

/// Filled danger action button (icon + label) for destructive actions.
fn danger_button<'a>(s: Skin, src: &'static str, lbl: &str, msg: Message) -> Element<'a, Message> {
    button(
        row![
            icon(src, 14.0, Color::WHITE),
            label(lbl.to_string(), 13.0, Weight::Semibold, Color::WHITE),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([7, 13])
    .on_press(msg)
    .style(move |_: &Theme, status| {
        let bg = match status {
            button::Status::Hovered | button::Status::Pressed => lighten(s.danger, 0.06),
            _ => s.danger,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: Color::WHITE,
            border: Border {
                radius: 9.0.into(),
                ..Border::default()
            },
            ..button::Style::default()
        }
    })
    .into()
}

/// Outlined/ghost button (icon + label).
fn ghost_button<'a>(s: Skin, src: &'static str, lbl: &str, msg: Message) -> Element<'a, Message> {
    button(
        row![
            icon(src, 14.0, s.icon),
            label(lbl.to_string(), 13.0, Weight::Medium, s.text),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([7, 11])
    .on_press(msg)
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
    })
    .into()
}

/// Small square icon-only button with a tinted background.
fn soft_icon_button<'a>(
    s: Skin,
    src: &'static str,
    msg: Message,
    color: Color,
) -> Element<'a, Message> {
    let _ = s;
    button(icon(src, 15.0, color))
        .padding(7)
        .on_press(msg)
        .style(move |_: &Theme, status| {
            let a = match status {
                button::Status::Hovered | button::Status::Pressed => 0.26,
                _ => 0.14,
            };
            button::Style {
                background: Some(Background::Color(alpha(color, a))),
                border: Border {
                    radius: 9.0.into(),
                    ..Border::default()
                },
                ..button::Style::default()
            }
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
                a: if dragging { 0.35 } else { 0.16 },
                ..Color::BLACK
            },
            offset: Vector::new(0.0, if dragging { 6.0 } else { 2.0 }),
            blur_radius: if dragging { 18.0 } else { 7.0 },
        },
        ..container::Style::default()
    }
}

fn bg_style(color: Color) -> container::Style {
    container::Style {
        background: Some(Background::Color(color)),
        ..container::Style::default()
    }
}

fn divider<'a>(color: Color) -> Element<'a, Message> {
    container(Space::with_height(Length::Fixed(1.0)))
        .width(Length::Fill)
        .style(move |_: &Theme| bg_style(color))
        .into()
}

fn vsep<'a>(color: Color) -> Element<'a, Message> {
    container(Space::with_height(Length::Fixed(24.0)))
        .width(Length::Fixed(1.0))
        .style(move |_: &Theme| bg_style(color))
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

/// Make a string safe to use as a file name: keep alphanumerics, space, dash and
/// underscore; collapse everything else to '_'. Falls back to "collection".
fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || ch == ' ' || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "collection".to_string()
    } else {
        trimmed.to_string()
    }
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}

fn alpha(c: Color, a: f32) -> Color {
    Color { a, ..c }
}

fn lighten(c: Color, amt: f32) -> Color {
    Color::from_rgb(
        (c.r + amt).min(1.0),
        (c.g + amt).min(1.0),
        (c.b + amt).min(1.0),
    )
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
const I_CHEV_LEFT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="m15 18-6-6 6-6"/></svg>"#;
const I_LAYERS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83Z"/><path d="M2 12a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 12"/><path d="M2 17a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 17"/></svg>"#;
const I_PENCIL: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z"/><path d="m15 5 4 4"/></svg>"#;
const I_DOWNLOAD: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" x2="12" y1="15" y2="3"/></svg>"#;
const I_UPLOAD: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" x2="12" y1="3" y2="15"/></svg>"#;
const I_ALERT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/><path d="M12 9v4"/><path d="M12 17h.01"/></svg>"#;
const I_CHECK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>"#;
const I_X: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>"#;
const I_PLUS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="M12 5v14"/></svg>"#;
const I_MINUS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/></svg>"#;
const I_TRASH: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/><path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>"#;
const I_PLAY: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"><polygon points="6 3 20 12 6 21 6 3"/></svg>"#;
const I_MOON: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"/></svg>"#;
const I_SUN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/></svg>"#;
const I_SWORDS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="14.5 17.5 3 6 3 3 6 3 17.5 14.5"/><line x1="13" x2="19" y1="19" y2="13"/><line x1="16" x2="20" y1="16" y2="20"/><line x1="19" x2="21" y1="21" y2="19"/></svg>"#;
