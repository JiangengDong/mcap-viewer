use std::collections::HashMap;
use std::path::{Path, PathBuf};

use egui::{Align2, Color32, Frame, Id, LayerId, Order, TextStyle};
use egui_dock::{DockArea, DockState, Style};
use std::fmt::Write as _;

use crate::loader;
use crate::tab::{LinePlot, Viewer};

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
    storage: mcap_viewer_storage::DataStorage,
    #[serde(skip)]
    file_operation: FileOperation,
    #[serde(skip)]
    loader: loader::Loader,

    /// The number of tabs created. This may overflow after a long time, but I don't want to think about it now.
    tab_monotonic_counter: usize,
    active_layout_name: String,
    new_layout_name: String,
    layouts: HashMap<String, DockState<LinePlot>>,
}

impl Default for McapViewer {
    fn default() -> Self {
        Self {
            storage: mcap_viewer_storage::DataStorage::default(),
            file_operation: FileOperation::None,
            loader: loader::Loader::new(),
            tab_monotonic_counter: 0,
            active_layout_name: String::new(),
            new_layout_name: String::new(),
            layouts: HashMap::new(),
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
        &mut self,
        added_tabs: Vec<(egui_dock::SurfaceIndex, egui_dock::NodeIndex)>,
    ) {
        let Some(tree) = self.layouts.get_mut(&self.active_layout_name) else {
            return;
        };
        for (surface, node) in added_tabs {
            tree.set_focused_node_and_surface((surface, node));
            let tab = LinePlot::new(self.tab_monotonic_counter);
            self.tab_monotonic_counter += 1;
            tree.push_to_focused_leaf(tab);
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
                    self.storage = storage;
                    FileOperation::Loaded(paths)
                } else {
                    FileOperation::Loading(paths)
                }
            }
            other => other,
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
        egui::Area::new("power by")
            .pivot(Align2::RIGHT_BOTTOM)
            .anchor(Align2::RIGHT_BOTTOM, [0.0, 0.0])
            .movable(false)
            .interactable(false)
            .show(ctx, |ui| {
                egui::warn_if_debug_build(ui);
                powered_by_egui_and_eframe(ui);
            });
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Layout: ");
                egui::ComboBox::from_id_source("Layout")
                    .selected_text(&self.active_layout_name)
                    .show_ui(ui, |ui| {
                        let mut removed_layouts = Vec::new();
                        for layout in self.layouts.keys() {
                            ui.horizontal(|ui| {
                                if ui.button("-").clicked() {
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
                if ui
                    .text_edit_singleline(&mut self.new_layout_name)
                    .lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    && !self.new_layout_name.is_empty()
                {
                    self.layouts
                        .entry(self.new_layout_name.clone())
                        .or_insert_with(|| {
                            let new_layout =
                                DockState::new(vec![LinePlot::new(self.tab_monotonic_counter)]);
                            self.tab_monotonic_counter += 1;
                            new_layout
                        });
                    self.active_layout_name = std::mem::take(&mut self.new_layout_name);
                }

                if !self.active_layout_name.is_empty()
                    && !self.layouts.contains_key(&self.active_layout_name)
                {
                    self.active_layout_name =
                        self.layouts.keys().next().cloned().unwrap_or_default();
                }
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
                let mut viewer = Viewer::new(&self.storage);
                let Some(tree) = self.layouts.get_mut(&self.active_layout_name) else {
                    return;
                };
                DockArea::new(tree)
                    .style(Style::from_egui(ctx.style().as_ref()))
                    .show_add_buttons(true)
                    .show_inside(ui, &mut viewer);
                self.commit_added_tabs(viewer.into_added_tabs());
                // TODO: ensure main surface has at least one tab
            });

        preview_hovered_files(ctx);
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let paths = i.raw.dropped_files.iter().filter_map(|s| s.path.clone());
                self.try_start_loading(paths, ctx);
            }
        });
    }
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

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
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
}
