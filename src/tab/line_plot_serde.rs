use std::sync::atomic::Ordering;

use super::TAB_MONOTONIC_ID;

use super::LinePlot;

use super::Curve;

#[derive(serde::Serialize, serde::Deserialize, Copy, Clone)]
enum Corner {
    LeftTop,
    RightTop,
    LeftBottom,
    RightBottom,
}

impl From<Corner> for egui_plot::Corner {
    fn from(value: Corner) -> Self {
        match value {
            Corner::LeftTop => Self::LeftTop,
            Corner::RightTop => Self::RightTop,
            Corner::LeftBottom => Self::LeftBottom,
            Corner::RightBottom => Self::RightBottom,
        }
    }
}

impl From<egui_plot::Corner> for Corner {
    fn from(value: egui_plot::Corner) -> Self {
        match value {
            egui_plot::Corner::LeftTop => Self::LeftTop,
            egui_plot::Corner::RightTop => Self::RightTop,
            egui_plot::Corner::LeftBottom => Self::LeftBottom,
            egui_plot::Corner::RightBottom => Self::RightBottom,
        }
    }
}

#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct LinePlotDeProxy {
    title: String,
    curves: Vec<Curve>,
    show_settings: bool,
    x_axis_name: String,
    legend_corner: Option<Corner>,
}

#[derive(serde::Serialize, Default)]
#[serde(default)]
struct LinePlotSerProxy<'a> {
    title: &'a str,
    curves: &'a [Curve],
    show_settings: bool,
    x_axis_name: &'a str,
    legend_corner: Option<Corner>,
}

impl From<LinePlotDeProxy> for LinePlot {
    fn from(value: LinePlotDeProxy) -> Self {
        let LinePlotDeProxy {
            title,
            curves,
            show_settings,
            x_axis_name,
            legend_corner,
        } = value;
        Self {
            id: TAB_MONOTONIC_ID.fetch_add(1, Ordering::Relaxed),
            title,
            active_time_axis: x_axis_name,
            curves,
            show_curve_editor: show_settings,
            legend_corner: legend_corner.map(Into::into),
        }
    }
}

impl<'a> From<&'a LinePlot> for LinePlotSerProxy<'a> {
    fn from(value: &'a LinePlot) -> Self {
        let LinePlot {
            title,
            curves,
            show_curve_editor: show_settings,
            active_time_axis: x_axis_name,
            legend_corner,
            ..
        } = value;
        Self {
            title,
            curves,
            x_axis_name,
            show_settings: *show_settings,
            legend_corner: legend_corner.map(Into::into),
        }
    }
}

impl serde::Serialize for LinePlot {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let proxy = LinePlotSerProxy::from(self);
        proxy.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for LinePlot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let proxy = LinePlotDeProxy::deserialize(deserializer)?;
        Ok(LinePlot::from(proxy))
    }
}
