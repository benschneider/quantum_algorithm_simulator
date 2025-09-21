use crate::messages::Message;
use crate::state::AppState;

use crate::components::menu_bar;

pub fn render(state: &mut AppState, ctx: &egui::Context) -> Vec<Message> {
    let mut messages = Vec::new();
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            messages.extend(menu_bar::menu_bar(state, ui));
        });
    });
    messages
}
