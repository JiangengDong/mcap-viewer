use egui::Widget;

pub enum IconButton {
    Add,
    Close,
    Copy,
    Paste,
}

impl IconButton {
    pub fn light_image(&self) -> egui::ImageSource<'_> {
        match self {
            Self::Add => egui::include_image!("../../assets/light/add.png"),
            Self::Close => egui::include_image!("../../assets/light/close.png"),
            Self::Copy => egui::include_image!("../../assets/light/copy.png"),
            Self::Paste => egui::include_image!("../../assets/light/paste.png"),
        }
    }

    pub fn dark_image(&self) -> egui::ImageSource<'_> {
        match self {
            Self::Add => egui::include_image!("../../assets/dark/add.png"),
            Self::Close => egui::include_image!("../../assets/dark/close.png"),
            Self::Copy => egui::include_image!("../../assets/dark/copy.png"),
            Self::Paste => egui::include_image!("../../assets/dark/paste.png"),
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
