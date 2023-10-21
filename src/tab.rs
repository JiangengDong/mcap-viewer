use std::{ops::RangeInclusive, sync::atomic::AtomicUsize};

use egui::{epaint::Hsva, Color32, Id};
use egui_autocomplete::AutoCompleteTextEdit;
use egui_plot::{Corner, Legend, Line, PlotPoint, PlotPoints, Points};
use mcap_viewer_storage::DataStorage;

use crate::{cache::PlotPointStorage, widgets::IconButton};

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
    pub x_axis_name: String,
    pub curves: Vec<Curve>,
    pub show_settings: bool,
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
            ..Default::default()
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, storage: &mut PlotPointStorage) {
        if self.show_settings {
            self.menu(ui, storage.inner());
        }
        self.plot(ui, storage);
    }

    fn menu(&mut self, ui: &mut egui::Ui, storage: &DataStorage) {
        ui.collapsing("Settings", |ui| {
            ui.horizontal(|ui| {
                ui.label("Title");
                ui.text_edit_singleline(&mut self.title);
                self.settings_copy_paste_buttons(ui);
            });

            // ui.horizontal(|ui| {
            //     ui.label("Time axis");
            //     let id = ui.auto_id_with("new-time-axis-name");
            //     let mut new_time_axis_name = ui
            //         .memory(|mem| mem.data.get_temp::<String>(id))
            //         .unwrap_or_default();
            //     if ui
            //         .text_edit_singleline(&mut new_time_axis_name)
            //         .lost_focus()
            //         && ui.input(|i| i.key_pressed(egui::Key::Enter))
            //         && !new_time_axis_name.is_empty()
            //     {
            //         self.x_axis_name = std::mem::take(&mut new_time_axis_name);
            //     }
            //     ui.memory_mut(|mem| mem.data.insert_temp(id, new_time_axis_name));
            // });

            self.curve_editor(ui, storage);

            self.legend_settings(ui);
        });
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

    fn legend_settings(&mut self, ui: &mut egui::Ui) -> egui::InnerResponse<()> {
        ui.horizontal(|ui| {
            ui.label("Legend position");
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
    }

    fn settings_copy_paste_buttons(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
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

    fn plot(&self, ui: &mut egui::Ui, storage: &mut PlotPointStorage) {
        let plot = egui_plot::Plot::new("plot")
            .link_axis("time", true, false)
            .link_cursor("time", true, false)
            // .link_axis(Id::new(&self.x_axis_name), true, false)
            // .link_cursor(Id::new(&self.x_axis_name), true, false)
            .auto_bounds_x()
            .auto_bounds_y()
            .x_axis_formatter(Self::x_label)
            .label_formatter(Self::format_label);

        let plot = if let Some(corner) = self.legend_corner {
            plot.legend(Legend::default().position(corner))
        } else {
            plot
        };

        let all_points = self.curves.iter().map(|curve| {
            let key = (curve.topic.as_str(), curve.field.as_str());
            storage.get(key)
        });

        plot.show(ui, move |ui| {
            let mut next_auto_color_idx = 0;
            for (curve, points) in self.curves.iter().zip(all_points) {
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

    fn context_menu(
        &mut self,
        ui: &mut egui::Ui,
        tab: &mut Self::Tab,
        _surface: egui_dock::SurfaceIndex,
        _node: egui_dock::NodeIndex,
    ) {
        if tab.show_settings {
            if ui.button("hide settings").clicked() {
                tab.show_settings = false;
                ui.close_menu();
            }
        } else if ui.button("show settings").clicked() {
            tab.show_settings = true;
            ui.close_menu();
        }
    }
}

impl<'a> Viewer<'a> {
    pub fn new(storage: &'a mut PlotPointStorage) -> Self {
        Self {
            storage,
            added_tabs: Vec::new(),
        }
    }

    pub fn into_added_tabs(self) -> Vec<(egui_dock::SurfaceIndex, egui_dock::NodeIndex)> {
        self.added_tabs
    }
}
