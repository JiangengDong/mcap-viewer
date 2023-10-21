pub struct MemorizeTextEdit;

pub struct Response {
    pub response: egui::Response,
    pub text: String,
    pub confirmed: bool,
}

impl MemorizeTextEdit {
    pub fn show(ui: &mut egui::Ui) -> Response {
        let id = ui.next_auto_id();

        let mut new_text = ui
            .memory(|mem| mem.data.get_temp::<String>(id))
            .unwrap_or_default();

        let response = ui
            .text_edit_singleline(&mut new_text)
            .on_hover_text("Press enter to confirm");
        let mut confirmed = false;
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            ui.memory_mut(|mem| mem.data.insert_temp(id, String::new()));
            confirmed = true;
        } else {
            ui.memory_mut(|mem| mem.data.insert_temp(id, new_text.clone()));
        }

        Response {
            response,
            text: new_text,
            confirmed,
        }
    }
}
