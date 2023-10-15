use egui::Frame;
use egui_dock::{DockArea, DockState, Style};

#[derive(serde::Deserialize, serde::Serialize, Default)]
struct TabState {
    pub id: usize,
    pub title: String,
    pub topic: String,
    pub field: String,
}

struct TabViewer<'a> {
    storage: &'a mcap_viewer_storage::DataStorage,
}

impl<'a> TabViewer<'a> {
    fn new(storage: &'a mcap_viewer_storage::DataStorage) -> Self {
        Self { storage }
    }
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = TabState;

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::new(tab.id)
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        if tab.title.is_empty() {
            format!("Tab {}", tab.id).into()
        } else {
            tab.title.as_str().into()
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        ui.collapsing("Settings", |ui| {
            ui.horizontal(|ui| {
                ui.label("Title");
                ui.text_edit_singleline(&mut tab.title);
                if ui.button("Add curve").clicked() {}
            });
            ui.horizontal(|ui| {
                ui.label("Topic: ");
                if !self.storage.0.contains_key(&tab.topic) {
                    tab.topic.clear();
                }
                let all_topics = self.storage.0.keys();
                egui::containers::ComboBox::from_id_source("topic")
                    .selected_text(&tab.topic)
                    .show_ui(ui, |ui| {
                        for topic in all_topics {
                            if ui.selectable_label(topic == &tab.topic, topic).clicked() {
                                tab.topic.clone_from(topic);
                            }
                        }
                    });
                if let Some(selected_topic) = self.storage.0.get(&tab.topic) {
                    ui.label("Field: ");
                    if !selected_topic.0.contains_key(&tab.field) {
                        tab.field.clear();
                    }
                    let all_fields = selected_topic.0.keys();
                    egui::containers::ComboBox::from_id_source("field")
                        .selected_text(&tab.field)
                        .show_ui(ui, |ui| {
                            for field in all_fields {
                                if ui.selectable_label(field == &tab.field, field).clicked() {
                                    tab.field.clone_from(field);
                                }
                            }
                        });
                }
            });
        });
    }

    fn allowed_in_windows(&self, _tab: &mut Self::Tab) -> bool {
        false
    }
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct McapViewer {
    #[serde(skip)] // This how you opt-out of serialization of a field
    storage: mcap_viewer_storage::DataStorage,

    /// The number of tabs created. This may overflow after a long time, but I don't want to think about it now.
    tab_monotonic_counter: usize,
    tree: DockState<TabState>,
}

impl McapViewer {
    fn new_tab(&mut self) -> TabState {
        let tab = TabState {
            id: self.tab_monotonic_counter,
            ..Default::default()
        };
        self.tab_monotonic_counter += 1;
        tab
    }
}

impl Default for McapViewer {
    fn default() -> Self {
        let mut storage = mcap_viewer_storage::DataStorage::default();
        storage
            .0
            .entry("topic1".to_owned())
            .or_default()
            .0
            .entry("field1".to_owned())
            .or_default();
        storage
            .0
            .entry("topic2".to_owned())
            .or_default()
            .0
            .entry("field2".to_owned())
            .or_default();
        Self {
            storage,
            tree: DockState::new(vec![TabState::default()]),
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
}

impl eframe::App for McapViewer {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(Frame::central_panel(&ctx.style()).inner_margin(0.))
            .show(ctx, |ui| {
                DockArea::new(&mut self.tree)
                    .style(Style::from_egui(ctx.style().as_ref()))
                    .show_add_buttons(true)
                    .show_inside(ui, &mut TabViewer::new(&self.storage));

                if cfg!(debug_assertions) {
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                        powered_by_egui_and_eframe(ui);
                        egui::warn_if_debug_build(ui);
                    });
                }
            });
        if self.tree.main_surface().is_empty() {
            let tab = self.new_tab();
            self.tree.main_surface_mut().push_to_first_leaf(tab);
        }
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
