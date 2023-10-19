use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use egui::{Align2, Color32, Frame, Id, LayerId, Order, TextStyle};
use egui_dock::{DockArea, DockState, Style};

use std::fmt::Write as _;

use crate::cache::PlotPointStorage;
use crate::loader;
use crate::tab::{LinePlot, Viewer};
use crate::widgets::IconButton;

enum FileOperation {
    None,
    Loading(Vec<PathBuf>),
    Loaded(Vec<PathBuf>),
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct McapViewer {
    #[serde(skip)] // This how you opt-out of serialization of a field
    storage: PlotPointStorage,
    #[serde(skip)]
    file_operation: FileOperation,
    #[serde(skip)]
    loader: loader::Loader,

    /// The name of the active layout.
    active_layout_name: String,
    /// All the layouts.
    layouts: BTreeMap<String, DockState<LinePlot>>,
}

impl Default for McapViewer {
    fn default() -> Self {
        Self {
            storage: PlotPointStorage::default(),
            file_operation: FileOperation::None,
            loader: loader::Loader::new(),
            active_layout_name: String::new(),
            layouts: BTreeMap::new(),
        }
    }
}

impl McapViewer {
    /// Called once before the first frame.
    #[must_use]
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Self::default()
    }

    /// Load all mcap files from a directory or a single file, and start the viewer with the data.
    pub fn from_path<P: AsRef<Path>>(
        cc: &eframe::CreationContext<'_>,
        path: P,
    ) -> anyhow::Result<Self> {
        let mut obj = Self::new(cc);
        obj.try_start_loading(vec![path.as_ref().to_owned()], &cc.egui_ctx);

        Ok(obj)
    }

    fn commit_added_tabs(
        tree: &mut DockState<LinePlot>,
        added_tabs: Vec<(egui_dock::SurfaceIndex, egui_dock::NodeIndex)>,
    ) {
        for (surface, node) in added_tabs {
            tree.set_focused_node_and_surface((surface, node));
            tree.push_to_focused_leaf(LinePlot::new());
        }
    }

    fn try_start_loading(&mut self, paths: impl IntoIterator<Item = PathBuf>, ctx: &egui::Context) {
        if let FileOperation::Loading(_) = self.file_operation {
            log::warn!("Cannot start a new loading because it is already loading other files");
            return;
        }
        let paths: Vec<_> = paths.into_iter().collect();
        self.loader.send(paths.clone(), ctx);
        self.file_operation = FileOperation::Loading(paths);
    }

    fn try_finish_loading(&mut self) {
        self.file_operation = match std::mem::replace(&mut self.file_operation, FileOperation::None)
        {
            FileOperation::Loading(paths) => {
                if let Some(storage) = self.loader.try_recv() {
                    self.storage = PlotPointStorage::new(storage);
                    FileOperation::Loaded(paths)
                } else {
                    FileOperation::Loading(paths)
                }
            }
            other => other,
        }
    }

    fn layout_menu(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Layout: ");
            egui::ComboBox::from_id_source("Layout")
                .selected_text(&self.active_layout_name)
                .show_ui(ui, |ui| {
                    let mut removed_layouts = Vec::new();
                    for layout in self.layouts.keys() {
                        ui.horizontal(|ui| {
                            if ui
                                .add(IconButton::Close)
                                .on_hover_text("Remove layout")
                                .clicked()
                            {
                                removed_layouts.push(layout.clone());
                            }
                            if ui
                                .selectable_label(self.active_layout_name == *layout, layout)
                                .clicked()
                            {
                                self.active_layout_name = layout.clone();
                            }
                        });
                    }
                    for layout in removed_layouts {
                        self.layouts.remove(&layout);
                    }
                });

            ui.label("New layout: ");
            let id = ui.auto_id_with("new-layout-name");
            let mut new_layout_name = ui
                .memory(|mem| mem.data.get_temp::<String>(id))
                .unwrap_or_default();
            if ui.text_edit_singleline(&mut new_layout_name).lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                && !new_layout_name.is_empty()
            {
                self.layouts
                    .entry(new_layout_name.clone())
                    .or_insert_with(|| DockState::new(vec![]));
                self.active_layout_name = std::mem::take(&mut new_layout_name);
            }

            if ui
                .add(IconButton::Add)
                .on_hover_text("New empty layout")
                .clicked()
                && !new_layout_name.is_empty()
            {
                self.layouts
                    .entry(new_layout_name.clone())
                    .or_insert_with(|| DockState::new(vec![]));
                self.active_layout_name = std::mem::take(&mut new_layout_name);
            }

            if ui
                .add(IconButton::Copy)
                .on_hover_text("Copy from current layout")
                .clicked()
                && !new_layout_name.is_empty()
            {
                let new_layout_name = std::mem::take(&mut new_layout_name);
                let layout = self
                    .layouts
                    .get(&self.active_layout_name)
                    .cloned()
                    .unwrap_or_else(|| DockState::new(vec![]));
                self.layouts
                    .entry(new_layout_name.clone())
                    .or_insert_with(|| layout);
                self.active_layout_name = new_layout_name;
            }
            ui.memory_mut(|mem| mem.data.insert_temp(id, new_layout_name));
        });

        // ensure there is at least one layout
        if self.layouts.is_empty() {
            self.layouts
                .insert("Default".to_owned(), DockState::new(vec![]));
        }

        // ensure active layout is valid
        if !self.layouts.contains_key(&self.active_layout_name) {
            self.active_layout_name = self.layouts.keys().next().cloned().unwrap_or_default();
        }
    }
}

impl eframe::App for McapViewer {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.try_finish_loading();

        powered_by_egui_and_eframe_overlay(ctx);

        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.layout_menu(ui);
                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| match &self.file_operation {
                FileOperation::None => {}
                FileOperation::Loading(path) => {
                    ui.spinner();
                    if path.len() == 1 {
                        ui.label(format!("Loading {}", path[0].display()));
                    } else {
                        ui.label(format!("Loading {} files", path.len()));
                    }
                }
                FileOperation::Loaded(path) => {
                    if path.len() == 1 {
                        ui.label(format!("Loaded {}", path[0].display()));
                    } else {
                        ui.label(format!("Loaded {} files", path.len()));
                    }
                }
            });
        });

        egui::CentralPanel::default()
            .frame(Frame::central_panel(&ctx.style()).inner_margin(0.))
            .show(ctx, |ui| {
                let mut viewer = Viewer::new(&mut self.storage);
                let Some(tree) = self.layouts.get_mut(&self.active_layout_name) else {
                    log::warn!("Cannot find layout {}", self.active_layout_name);
                    return;
                };

                DockArea::new(tree)
                    .style(Style::from_egui(ctx.style().as_ref()))
                    .show_add_buttons(true)
                    .show_inside(ui, &mut viewer);

                Self::commit_added_tabs(tree, viewer.into_added_tabs());

                if tree.main_surface().is_empty() {
                    tree.main_surface_mut().push_to_first_leaf(LinePlot::new());
                }
            });

        preview_hovered_files(ctx);
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let paths = i.raw.dropped_files.iter().filter_map(|s| s.path.clone());
                self.try_start_loading(paths, ctx);
            }
        });

        self.storage.evice_cache();
    }
}

fn powered_by_egui_and_eframe_overlay(ctx: &egui::Context) {
    egui::Area::new("power by")
        .pivot(Align2::RIGHT_BOTTOM)
        .anchor(Align2::RIGHT_BOTTOM, [0.0, 0.0])
        .movable(false)
        .interactable(false)
        .show(ctx, |ui| {
            egui::warn_if_debug_build(ui);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("Powered by ");
                ui.hyperlink_to("egui", "https://github.com/emilk/egui");
                ui.label(" and ");
                ui.hyperlink_to(
                    "eframe",
                    "https://github.com/emilk/egui/tree/master/crates/eframe",
                );
                ui.label(".");
            });
        });
}

fn preview_hovered_files(ctx: &egui::Context) {
    if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
        let text = ctx.input(|i| {
            let mut text = "Dropping files:\n".to_owned();
            for file in &i.raw.hovered_files {
                if let Some(path) = &file.path {
                    write!(text, "\n{}", path.display()).ok();
                } else if !file.mime.is_empty() {
                    write!(text, "\n{}", file.mime).ok();
                } else {
                    text += "\n???";
                }
            }
            text
        });

        let painter =
            ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

        let screen_rect = ctx.screen_rect();
        painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
        painter.text(
            screen_rect.center(),
            Align2::CENTER_CENTER,
            text,
            TextStyle::Heading.resolve(&ctx.style()),
            Color32::WHITE,
        );
    }
}
