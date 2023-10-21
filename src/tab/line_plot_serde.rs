use std::sync::atomic::Ordering;

use super::TAB_MONOTONIC_ID;

use super::LinePlot;

use super::Curve;

#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct LinePlotDeProxy {
    title: String,
    curves: Vec<Curve>,
    show_settings: bool,
    x_axis_name: String,
}

#[derive(serde::Serialize, Default)]
#[serde(default)]
struct LinePlotSerProxy<'a> {
    title: &'a str,
    curves: &'a [Curve],
    show_settings: bool,
    x_axis_name: &'a str,
}

impl From<LinePlotDeProxy> for LinePlot {
    fn from(value: LinePlotDeProxy) -> Self {
        let LinePlotDeProxy {
            title,
            curves,
            show_settings,
            x_axis_name,
        } = value;
        Self {
            id: TAB_MONOTONIC_ID.fetch_add(1, Ordering::Relaxed),
            title,
            active_time_axis: x_axis_name,
            curves,
            show_settings,
            legend_corner: None,
        }
    }
}

impl<'a> From<&'a LinePlot> for LinePlotSerProxy<'a> {
    fn from(value: &'a LinePlot) -> Self {
        let LinePlot {
            title,
            curves,
            show_settings,
            active_time_axis: x_axis_name,
            ..
        } = value;
        Self {
            title,
            curves,
            x_axis_name,
            show_settings: *show_settings,
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
