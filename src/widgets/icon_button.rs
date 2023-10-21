use egui::Widget;

pub enum IconButton {
    Add,
    Close,
    Copy,
    Paste,
    Plot,
}

impl IconButton {
    pub fn light_image(&self) -> egui::ImageSource<'_> {
        match self {
            Self::Add => egui::include_image!("../../assets/light/add.png"),
            Self::Close => egui::include_image!("../../assets/light/close.png"),
            Self::Copy => egui::include_image!("../../assets/light/copy.png"),
            Self::Paste => egui::include_image!("../../assets/light/paste.png"),
            Self::Plot => egui::include_image!("../../assets/light/plot.png"),
        }
    }

    pub fn dark_image(&self) -> egui::ImageSource<'_> {
        match self {
            Self::Add => egui::include_image!("../../assets/dark/add.png"),
            Self::Close => egui::include_image!("../../assets/dark/close.png"),
            Self::Copy => egui::include_image!("../../assets/dark/copy.png"),
            Self::Paste => egui::include_image!("../../assets/dark/paste.png"),
            Self::Plot => egui::include_image!("../../assets/dark/plot.png"),
        }
    }

    pub fn selected(self, selected: bool) -> Selected {
        Selected {
            icon_button: self,
            selected,
        }
    }
}

impl Widget for IconButton {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        if ui.style().visuals.dark_mode {
            ui.add(egui::Button::image(self.dark_image()))
        } else {
            ui.add(egui::Button::image(self.light_image()))
        }
    }
}

pub struct Selected {
    icon_button: IconButton,
    selected: bool,
}

impl Widget for Selected {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        if ui.style().visuals.dark_mode {
            ui.add(egui::Button::image(self.icon_button.dark_image()).selected(self.selected))
        } else {
            ui.add(egui::Button::image(self.icon_button.light_image()).selected(self.selected))
        }
    }
}
