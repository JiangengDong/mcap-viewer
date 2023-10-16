use std::path::{Path, PathBuf};

use egui::{Align2, Frame, Layout};
use egui_dock::{DockArea, DockState, Style};

use crate::loader;
use crate::tab::{LinePlot, Viewer};

enum FileOperation {
    None,
    Loading(PathBuf),
    Loaded(PathBuf),
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct McapViewer {
    #[serde(skip)] // This how you opt-out of serialization of a field
    storage: mcap_viewer_storage::DataStorage,
    #[serde(skip)]
    file_operation: FileOperation,

    /// The number of tabs created. This may overflow after a long time, but I don't want to think about it now.
    tab_monotonic_counter: usize,
    tree: DockState<LinePlot>,
}

impl Default for McapViewer {
    fn default() -> Self {
        Self {
            storage: mcap_viewer_storage::DataStorage::default(),
            file_operation: FileOperation::None,
            tree: DockState::new(vec![LinePlot::default()]),
            tab_monotonic_counter: 1,
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
        let default = Self::new(cc);

        let mut all_paths = loader::list_all_mcap_files(path.as_ref());
        let mut storage = mcap_viewer_storage::DataStorage::default();
        let decoder = mcap_ros2_decoder::Decoder::default();
        for path in &all_paths {
            let bytes = std::fs::read(path)?;
            loader::parse_all_schemas(&bytes, &decoder);
            loader::decode_single_thread(&bytes, &decoder, &mut storage);
        }
        let file_operation = all_paths
            .pop()
            .map_or(FileOperation::None, FileOperation::Loaded);
        Ok(Self {
            storage,
            file_operation,
            ..default
        })
    }

    fn new_tab(&mut self) -> LinePlot {
        let tab = LinePlot::new(self.tab_monotonic_counter);
        self.tab_monotonic_counter += 1;
        tab
    }

    fn commit_added_tabs(
        &mut self,
        added_tabs: Vec<(egui_dock::SurfaceIndex, egui_dock::NodeIndex)>,
    ) {
        for (surface, node) in added_tabs {
            self.tree.set_focused_node_and_surface((surface, node));
            let tab = self.new_tab();
            self.tree.push_to_focused_leaf(tab);
        }

        if self.tree.main_surface().is_empty() {
            let tab = self.new_tab();
            self.tree.main_surface_mut().push_to_first_leaf(tab);
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
                if ui.button("load file").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        self.file_operation = FileOperation::Loading(path);
                    }
                }
                egui::ComboBox::from_label("Layout")
                    .selected_text("")
                    .show_ui(ui, |ui| {});
            });
        });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| match &self.file_operation {
                FileOperation::None => {}
                FileOperation::Loading(path) => {
                    ui.spinner();
                    ui.label(format!("Loading {}", path.display()));
                }
                FileOperation::Loaded(path) => {
                    ui.label(format!("Loaded {}", path.display()));
                }
            });
        });
        egui::CentralPanel::default()
            .frame(Frame::central_panel(&ctx.style()).inner_margin(0.))
            .show(ctx, |ui| {
                let mut viewer = Viewer::new(&self.storage);
                DockArea::new(&mut self.tree)
                    .style(Style::from_egui(ctx.style().as_ref()))
                    .show_add_buttons(true)
                    .show_inside(ui, &mut viewer);
                self.commit_added_tabs(viewer.into_added_tabs());
            });
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
