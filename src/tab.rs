use std::ops::RangeInclusive;

use egui::{Color32, Id};
use egui_autocomplete::AutoCompleteTextEdit;
use egui_plot::{Corner, Legend, Line, PlotPoint, Points};
use mcap_viewer_storage::DataStorage;

use crate::utils::IconButton;

#[derive(serde::Deserialize, serde::Serialize, Clone)]
#[serde(default)]
pub struct Curve {
    pub topic: String,
    pub field: String,
    pub color: Color32,
}

impl Default for Curve {
    fn default() -> Self {
        Self {
            topic: String::new(),
            field: String::new(),
            color: Color32::from_rgba_unmultiplied(255, 0, 0, 128),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
#[serde(default)]
pub struct LinePlot {
    pub id: usize,
    pub title: String,
    pub curves: Vec<Curve>,

    #[serde(skip)]
    pub legend_corner: Option<Corner>,
}

impl LinePlot {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, storage: &DataStorage) {
        self.menu(ui, storage);
        self.plot(ui, storage);
    }

    fn menu(&mut self, ui: &mut egui::Ui, storage: &DataStorage) {
        ui.collapsing("Settings", |ui| {
            self.settings_title(ui);

            let mut keep = vec![true; self.curves.len()];
            let mut insert = vec![];
            for (i, curve) in self.curves.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    if ui
                        .add(IconButton::Close)
                        .on_hover_text("Remove curve")
                        .clicked()
                    {
                        keep[i] = false;
                    }

                    ui.color_edit_button_srgba(&mut curve.color);

                    let all_topics = storage.keys();
                    ui.add(
                        AutoCompleteTextEdit::new(&mut curve.topic, all_topics)
                            .max_suggestions(10)
                            .highlight_matches(true)
                            .set_text_edit_properties(|text| {
                                text.clip_text(false).hint_text("topic")
                            }),
                    );

                    if let Some(selected_topic) = storage.get(&curve.topic) {
                        let all_fields = selected_topic.keys();
                        ui.add(
                            AutoCompleteTextEdit::new(&mut curve.field, all_fields)
                                .max_suggestions(10)
                                .highlight_matches(true)
                                .set_text_edit_properties(|text| {
                                    text.clip_text(false).hint_text("field")
                                }),
                        );
                    }

                    if ui.add(IconButton::Add).on_hover_text("Add curve").clicked() {
                        insert.push((i, false));
                    }
                    if ui
                        .add(IconButton::Copy)
                        .on_hover_text("Copy curve")
                        .clicked()
                    {
                        insert.push((i, true));
                    }
                });
            }
            let mut keep_iter = keep.into_iter();
            self.curves.retain(|_| keep_iter.next().unwrap());
            for (insert_place, copy) in insert.into_iter().rev() {
                if copy {
                    self.curves
                        .insert(insert_place + 1, self.curves[insert_place].clone());
                } else {
                    self.curves.insert(insert_place + 1, Curve::default());
                }
            }
            if self.curves.is_empty() {
                self.curves.push(Curve::default());
            }

            ui.horizontal(|ui| {
                ui.label("Legend position: ");
                ui.selectable_value(
                    &mut self.legend_corner,
                    Some(Corner::LeftBottom),
                    "left bottom",
                );
                ui.selectable_value(&mut self.legend_corner, Some(Corner::LeftTop), "left top");
                ui.selectable_value(
                    &mut self.legend_corner,
                    Some(Corner::RightBottom),
                    "right bottom",
                );
                ui.selectable_value(&mut self.legend_corner, Some(Corner::RightTop), "right top");
                ui.selectable_value(&mut self.legend_corner, None, "none");
            })
        });
    }

    fn settings_title(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Title");
            ui.text_edit_singleline(&mut self.title);
            if ui
                .add(IconButton::Copy)
                .on_hover_text("Copy settings")
                .clicked()
            {
                let setting_string = ron::to_string(self).unwrap_or_default();
                ui.memory_mut(|mem| {
                    mem.data
                        .insert_temp(Id::new("mcap-viewer-plot-settings-copy"), setting_string);
                });
            }
            if ui
                .add(IconButton::Paste)
                .on_hover_text("Paste settings")
                .clicked()
            {
                let setting_string = ui.memory(|mem| {
                    mem.data
                        .get_temp::<String>(Id::new("mcap-viewer-plot-settings-copy"))
                });
                if let Some(setting_string) = setting_string {
                    if let Ok(settings) = ron::from_str::<Self>(&setting_string) {
                        let old_id = self.id;
                        *self = settings;
                        self.id = old_id;
                    }
                }
            }
        });
    }

    fn plot(&self, ui: &mut egui::Ui, storage: &DataStorage) {
        let plot = egui_plot::Plot::new("plot")
            .link_axis("time", true, false)
            .link_cursor("time", true, false)
            .auto_bounds_x()
            .auto_bounds_y()
            .x_axis_formatter(Self::x_label)
            .label_formatter(Self::format_label);

        let plot = if let Some(corner) = self.legend_corner {
            plot.legend(Legend::default().position(corner))
        } else {
            plot
        };

        plot.show(ui, move |ui| {
            for curve in &self.curves {
                if let Some(points) = storage
                    .get(&curve.topic)
                    .and_then(|topic| topic.get(&curve.field))
                {
                    let name = curve.topic.clone() + &curve.field;
                    ui.line(Line::new(points).name(name.clone()).color(curve.color));
                    ui.points(Points::new(points).name(name).color(curve.color));
                }
            }
        });
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::needless_pass_by_value
    )]
    fn x_label(value: f64, _limit: usize, _range: &RangeInclusive<f64>) -> String {
        let timestamp = (value / 1000.0) as i64;
        if let Some(timestamp) = chrono::NaiveDateTime::from_timestamp_micros(timestamp) {
            timestamp.to_string()
        } else {
            timestamp.to_string()
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn format_label(_series_name: &str, point: &PlotPoint) -> String {
        let timestamp = (point.x / 1000.0) as i64;
        if let Some(timestamp) = chrono::NaiveDateTime::from_timestamp_micros(timestamp) {
            format!("x:{}\ny:{}", timestamp, point.y)
        } else {
            format!("x:{}\ny:{}", point.x, point.y)
        }
    }
}

pub struct Viewer<'a> {
    storage: &'a DataStorage,
    added_tabs: Vec<(egui_dock::SurfaceIndex, egui_dock::NodeIndex)>,
}

impl egui_dock::TabViewer for Viewer<'_> {
    type Tab = LinePlot;

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
        tab.ui(ui, self.storage);
    }

    fn on_add(&mut self, surface: egui_dock::SurfaceIndex, node: egui_dock::NodeIndex) {
        self.added_tabs.push((surface, node));
    }
}

impl<'a> Viewer<'a> {
    pub fn new(storage: &'a DataStorage) -> Self {
        Self {
            storage,
            added_tabs: Vec::new(),
        }
    }

    pub fn into_added_tabs(self) -> Vec<(egui_dock::SurfaceIndex, egui_dock::NodeIndex)> {
        self.added_tabs
    }
}
