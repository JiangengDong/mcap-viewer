use egui::TextEdit;

pub struct MemorizeTextEdit<'a> {
    pub hint: Option<&'a str>,
    pub id_source: Option<egui::Id>,
}

pub struct Response {
    pub inner: egui::Response,
    pub text: String,
    pub confirmed: bool,
}

impl<'a> MemorizeTextEdit<'a> {
    pub fn new() -> Self {
        Self {
            hint: None,
            id_source: None,
        }
    }

    pub fn id<S>(self, id_source: &S) -> Self
    where
        S: std::hash::Hash + ?Sized,
    {
        Self {
            hint: self.hint,
            id_source: Some(egui::Id::new(id_source)),
        }
    }

    #[allow(clippy::unused_self)]
    pub fn hint<'b, T>(self, hint: &'b T) -> MemorizeTextEdit<'b>
    where
        T: AsRef<str> + ?Sized,
        'a: 'b,
    {
        MemorizeTextEdit {
            hint: Some(hint.as_ref()),
            id_source: self.id_source,
        }
    }

    pub fn show(self, ui: &mut egui::Ui) -> Response {
        let id = self.id_source.unwrap_or_else(|| ui.next_auto_id());

        let mut new_text = ui
            .memory(|mem| mem.data.get_temp::<String>(id))
            .unwrap_or_default();

        let mut editor = TextEdit::singleline(&mut new_text);
        if let Some(hint_text) = self.hint {
            editor = editor.hint_text(hint_text);
        }
        let response = editor
            .show(ui)
            .response
            .on_hover_text("Press enter to confirm");
        let mut confirmed = false;
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            ui.memory_mut(|mem| mem.data.insert_temp(id, String::new()));
            confirmed = true;
        } else {
            ui.memory_mut(|mem| mem.data.insert_temp(id, new_text.clone()));
        }

        Response {
            inner: response,
            text: new_text,
            confirmed,
        }
    }
}
