use std::{collections::BTreeSet, ops::RangeInclusive, sync::atomic::AtomicUsize};

use egui::{epaint::Hsva, Color32, Id};
use egui_autocomplete::AutoCompleteTextEdit;
use egui_plot::{Corner, Legend, Line, PlotBounds, PlotPoint, PlotPoints, Points};
use mcap_viewer_storage::DataStorage;

use crate::{
    cache::{Key, PlotPointStorage},
    widgets::{IconButton, MemorizeTextEdit},
};

mod line_plot_serde;

static TAB_MONOTONIC_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(serde::Deserialize, serde::Serialize, Clone, Default)]
#[serde(default)]
pub struct Curve {
    pub topic: String,
    pub field: String,
    pub color: Color32,
}

#[derive(Default, Clone)]
pub struct LinePlot {
    id: usize,
    pub title: String,
    pub active_time_axis: String,
    pub curves: Vec<Curve>,
    pub show_curve_editor: bool,
    pub legend_corner: Option<Corner>,
}

impl LinePlot {
    fn auto_color(next_auto_color_idx: &mut u16) -> Color32 {
        let i = *next_auto_color_idx;
        *next_auto_color_idx += 1;
        let golden_ratio = (5.0_f32.sqrt() - 1.0) / 2.0; // 0.61803398875
        let h = f32::from(i) * golden_ratio;
        Hsva::new(h, 0.85, 0.5, 0.5).into() // TODO(emilk): OkLab or some other perspective color space
    }

    pub fn new() -> Self {
        Self {
            id: TAB_MONOTONIC_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            active_time_axis: "time".to_owned(),
            ..Default::default()
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, viewer: &mut Viewer<'_>) {
        if self.show_curve_editor {
            self.curve_editor(ui, viewer.storage.inner());
        }
        self.plot(ui, viewer.storage);
    }

    fn context_menu(&mut self, ui: &mut egui::Ui, viewer: &mut Viewer<'_>) {
        ui.horizontal(|ui| {
            if self.settings_copy_paste_buttons(ui).clicked() {
                ui.close_menu();
            }
            if self.show_curve_editor_button(ui).clicked() {
                ui.close_menu();
            }
        });

        ui.horizontal(|ui| {
            ui.label("Title");
            ui.text_edit_singleline(&mut self.title);
        });

        ui.menu_button("Time axis group", |ui| self.time_axis_selector(ui, viewer));
        self.ensure_time_axis(viewer);

        ui.menu_button("Legend", |ui| self.legend_settings(ui));
    }

    fn show_curve_editor_button(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let hover_text = if self.show_curve_editor {
            "Hide curve editor"
        } else {
            "show curve editor"
        };
        let button = ui
            .add(IconButton::Plot.selected(self.show_curve_editor))
            .on_hover_text(hover_text);
        if button.clicked() {
            self.show_curve_editor = !self.show_curve_editor;
        }
        button
    }

    fn time_axis_selector(&mut self, ui: &mut egui::Ui, viewer: &mut Viewer<'_>) {
        let text_response = MemorizeTextEdit::new().hint("New time axis group").show(ui);
        if !text_response.text.is_empty() {
            let new_time_axis = text_response.text;

            if text_response.confirmed {
                viewer.time_axis_set.insert(new_time_axis.clone());
                self.active_time_axis = new_time_axis;
            }
        }

        let mut removed_time_axis = Vec::new();
        for time_axis in viewer.time_axis_set.iter() {
            ui.horizontal(|ui| {
                if ui
                    .add(IconButton::Close)
                    .on_hover_text("Remove time axis group")
                    .clicked()
                {
                    removed_time_axis.push(time_axis.clone());
                }
                if ui
                    .selectable_label(self.active_time_axis == *time_axis, time_axis)
                    .clicked()
                {
                    self.active_time_axis = time_axis.clone();
                }
            });
        }
        for time_axis in removed_time_axis {
            viewer.time_axis_set.remove(&time_axis);
        }
    }

    fn ensure_time_axis(&mut self, viewer: &mut Viewer<'_>) {
        // ensure there is at least one time axis
        if viewer.time_axis_set.is_empty() {
            viewer.time_axis_set.insert("time".to_owned());
        }

        // ensure the active time axis is valid
        if !viewer.time_axis_set.contains(&self.active_time_axis) {
            self.active_time_axis = viewer.time_axis_set.first().unwrap().clone();
        }
    }

    fn curve_editor(&mut self, ui: &mut egui::Ui, storage: &DataStorage) {
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
                        .set_text_edit_properties(|text| text.clip_text(false).hint_text("topic")),
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
    }

    fn legend_settings(&mut self, ui: &mut egui::Ui) {
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
    }

    fn settings_copy_paste_buttons(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let copy_button = ui.add(IconButton::Copy).on_hover_text("Copy tab settings");
        if copy_button.clicked() {
            let setting_string = ron::to_string(self).unwrap_or_default();
            ui.memory_mut(|mem| {
                mem.data
                    .insert_temp(Id::new("mcap-viewer-plot-settings-copy"), setting_string);
            });
        }
        let paste_button = ui
            .add(IconButton::Paste)
            .on_hover_text("Paste tab settings");
        if paste_button.clicked() {
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
        copy_button.union(paste_button)
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn plot(&self, ui: &mut egui::Ui, storage: &mut PlotPointStorage) {
        let plot = egui_plot::Plot::new("plot")
            .link_axis(Id::new(&self.active_time_axis), true, false)
            .link_cursor(Id::new("cursor"), true, false)
            .allow_double_click_reset(false)
            .x_axis_formatter(Self::x_label)
            .label_formatter(Self::format_label);

        let plot = if let Some(corner) = self.legend_corner {
            plot.legend(Legend::default().position(corner))
        } else {
            plot
        };

        plot.show(ui, move |ui| {
            let transform = *ui.transform();
            let mut bounds = *transform.bounds();
            if storage.dirty() || ui.response().double_clicked() {
                let new_bounds = self.collect_bounds(storage.inner());
                if new_bounds.is_finite() {
                    bounds = new_bounds;
                    ui.set_plot_bounds(new_bounds);
                }
            }
            let frame = transform.frame();

            let mut next_auto_color_idx = 0;
            for curve in &self.curves {
                let key = Key {
                    topic: &curve.topic,
                    field: &curve.field,
                    time_range: [bounds.min()[0] as i64, bounds.max()[0] as i64],
                    num_points: frame.width() as usize * 2,
                };
                let points = storage.get(&key);

                let name = curve.topic.clone() + &curve.field;
                let color = if curve.color == Color32::TRANSPARENT {
                    Self::auto_color(&mut next_auto_color_idx)
                } else {
                    curve.color
                };
                ui.line(
                    Line::new(PlotPoints::new(points.clone()))
                        .name(name.clone())
                        .color(color),
                );
                ui.points(Points::new(PlotPoints::new(points)).name(name).color(color));
            }
        });
    }

    fn collect_bounds(&self, storage: &DataStorage) -> PlotBounds {
        let (mut min, mut max) = ([f64::INFINITY; 2], [-f64::INFINITY; 2]);
        for curve in &self.curves {
            if let Some(curve) = storage.get_field(&curve.topic, &curve.field) {
                let curve = Vec::<[f64; 2]>::from(curve);
                (min, max) = curve.into_iter().fold((min, max), |bound, p| {
                    let (mut min, mut max) = bound;
                    min[0] = min[0].min(p[0]);
                    min[1] = min[1].min(p[1]);
                    max[0] = max[0].max(p[0]);
                    max[1] = max[1].max(p[1]);
                    (min, max)
                });
            }
        }
        PlotBounds::from_min_max(min, max)
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
    storage: &'a mut PlotPointStorage,
    time_axis_set: &'a mut BTreeSet<String>,
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
        tab.ui(ui, self);
    }

    fn on_add(&mut self, surface: egui_dock::SurfaceIndex, node: egui_dock::NodeIndex) {
        self.added_tabs.push((surface, node));
    }

    fn context_menu(
        &mut self,
        ui: &mut egui::Ui,
        tab: &mut Self::Tab,
        _surface: egui_dock::SurfaceIndex,
        _node: egui_dock::NodeIndex,
    ) {
        tab.context_menu(ui, self);
    }
}

impl<'a> Viewer<'a> {
    pub fn new(storage: &'a mut PlotPointStorage, time_axis_set: &'a mut BTreeSet<String>) -> Self {
        Self {
            storage,
            time_axis_set,
            added_tabs: Vec::new(),
        }
    }

    pub fn into_added_tabs(self) -> Vec<(egui_dock::SurfaceIndex, egui_dock::NodeIndex)> {
        self.added_tabs
    }
}
