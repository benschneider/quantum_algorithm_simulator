use crate::handlers;
use crate::messages::Message;
use crate::state::AppState;
use crate::ui::theme;

#[derive(Default)]
pub struct QCSimApp {
    pub state: AppState,
    pub messages: Vec<Message>,
}

impl QCSimApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        log::debug!("QCSimApp::new() called");
        theme::apply_theme(&cc.egui_ctx);

        Self {
            state: AppState::new(),
            messages: Vec::new(),
        }
    }

    pub fn dispatch(&mut self, message: Message) {
        self.messages.push(message);
    }
    pub fn handle_message(&mut self, message: Message) {
        handlers::handle_message(&mut self.state, message, &mut self.messages);
    }

    /// Drains the message queue, returning all pending messages.
    /// This is primarily a helper for testing environments where the main
    /// `eframe::App::update` loop is not running.
    pub fn drain_messages(&mut self) -> Vec<Message> {
        std::mem::take(&mut self.messages)
    }
}

impl eframe::App for QCSimApp {
    /// The `update` function is the main entry point for the application's
    /// update loop. It is called by `eframe` on every frame.
    ///
    /// The application follows a message-based architecture. The `update`
    /// function first processes any messages that have been dispatched since
    /// the last frame. These messages can trigger changes in the application
    /// state.
    ///
    /// After processing messages, the `update` function renders the UI. The
    /// UI is rendered by calling the `render` functions of the various UI
    /// components. These `render` functions can dispatch new messages, which
    /// will be processed in the next frame.
    ///
    /// This architecture allows for a clear separation of concerns between
    /// the application logic and the UI. The application logic is implemented
    /// in the message handlers, and the UI is implemented in the `render`
    /// functions.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let messages: Vec<Message> = self.messages.drain(..).collect();
        for message in messages {
            handlers::handle_message(&mut self.state, message, &mut self.messages);
        }

        let mut state = std::mem::take(&mut self.state);
        let mut ui_messages = Vec::new();

        let top_panel_messages = crate::ui::top_panel::render(&mut state, ctx);
        ui_messages.extend(top_panel_messages);

        if state.ui_state.show_gate_editor_window {
            let mut open = state.ui_state.show_gate_editor_window;
            egui::Window::new("Gate Editor")
                .vscroll(true)
                .open(&mut open)
                .show(ctx, |ui| {
                    crate::components::gate_editor::gate_editor_panel(
                        &mut state,
                        ui,
                        &mut ui_messages,
                    );
                });
            if !open {
                state.ui_state.palette_gate_for_editing = None;
            }
            state.ui_state.show_gate_editor_window = open;
        }

        if state.ui_state.show_results_window {
            let mut open = state.ui_state.show_results_window;
            egui::Window::new("Results")
                .vscroll(true)
                .open(&mut open)
                .show(ctx, |ui| {
                    if let Some(msg) =
                        crate::components::results_panel::results_panel(ui, &mut state)
                    {
                        ui_messages.push(msg);
                    }
                });
            state.ui_state.show_results_window = open;
        }

        crate::ui::central_panel::render(&mut state, ctx, &mut ui_messages);
        crate::ui::welcome_screen::render(&mut state, ctx);
        crate::ui::info_panel::render(&mut state, ctx);
        crate::ui::tutorial::render(&mut state, ctx);

        if let Some(message) =
            crate::components::initial_state_editor::initial_state_editor(&mut state, ctx)
        {
            ui_messages.push(message);
        }

        crate::components::custom_gate_definition_editor::custom_gate_definition_editor(
            &mut state,
            ctx,
            &mut ui_messages,
        );

        self.state = state;
        self.messages.extend(ui_messages);

        self.state.ui_state.bloch_sphere_animation_time += ctx.input(|i| i.unstable_dt) as f64;
        if self.state.ui_state.bloch_sphere_animation_time > 1.0 {
            self.state.ui_state.bloch_sphere_animation_time = 0.0;
        }
        ctx.request_repaint();
    }
}
