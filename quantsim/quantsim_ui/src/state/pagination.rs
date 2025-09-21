use egui::{Id, TextEdit, Ui};

use crate::messages::Message;

/// Represents the state of a paginated view.
#[derive(Clone, Debug, PartialEq)]
pub struct PaginationState {
    pub current_page: usize,
    pub entries_per_page: usize,
    pub total_entries: usize,
    pub total_pages: usize,
}

impl PaginationState {
    /// Creates a new `PaginationState`.
    pub fn new(total_entries: usize, entries_per_page: usize) -> Self {
        let total_pages = (total_entries as f32 / entries_per_page as f32).ceil() as usize;
        Self {
            current_page: 1, // Start at the first page
            entries_per_page,
            total_entries,
            total_pages,
        }
    }

    /// Moves to the next page.
    pub fn next_page(&mut self) {
        if self.current_page < self.total_pages {
            self.current_page += 1;
        }
    }

    /// Moves to the previous page.
    pub fn previous_page(&mut self) {
        if self.current_page > 1 {
            self.current_page -= 1;
        }
    }

    /// Sets the current page.
    pub fn set_page(&mut self, page: usize) {
        self.current_page = page.max(1).min(self.total_pages);
    }

    /// Gets the starting index of the current page.
    pub fn get_start_index(&self) -> usize {
        (self.current_page - 1) * self.entries_per_page
    }

    /// Gets the ending index of the current page.
    pub fn get_end_index(&self) -> usize {
        let end = self.current_page * self.entries_per_page;
        end.min(self.total_entries)
    }

    /// Renders the pagination controls.
    pub fn render_controls(
        &mut self,
        ui: &mut Ui,
        page_input_text: &mut String,
    ) -> Option<Message> {
        let mut message = None;

        let prev_button = ui.add_enabled_ui(self.current_page > 1, |ui| ui.button("Previous"));
        if prev_button.inner.clicked() {
            message = Some(Message::InitialStateEditorPageChanged(
                self.current_page - 1,
            ));
        }

        // Direct page input
        let text_edit_result = ui.add(
            TextEdit::singleline(page_input_text)
                .desired_width(30.0)
                .id(Id::new("page_input"))
                .hint_text("Page")
                .interactive(true),
        );
        text_edit_result.clone().on_hover_text("Enter page number");

        ui.label(format!("of {}", self.total_pages));

        // Handle Enter key press
        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Ok(new_page) = page_input_text.parse::<usize>() {
                message = Some(Message::InitialStateEditorPageChanged(new_page));
            } else {
                // If parsing fails, revert the input text to the current valid page
                *page_input_text = self.current_page.to_string();
            }
        } else if text_edit_result.lost_focus() && text_edit_result.changed() {
            // Handle focus lost and text changed
            if let Ok(new_page) = page_input_text.parse::<usize>() {
                message = Some(Message::InitialStateEditorPageChanged(new_page));
            } else {
                // If parsing fails, revert the input text to the current valid page
                *page_input_text = self.current_page.to_string();
            }
        }

        let next_button =
            ui.add_enabled_ui(self.current_page < self.total_pages, |ui| ui.button("Next"));
        if next_button.inner.clicked() {
            message = Some(Message::InitialStateEditorPageChanged(
                self.current_page + 1,
            ));
        }

        message
    }
}
