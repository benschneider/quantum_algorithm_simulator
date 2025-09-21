use quantsim_ui::{QCSimApp, messages::Message};

// Helper function to set up the application
fn setup_app() -> QCSimApp {
    QCSimApp::default()
}

#[test]
fn test_toggle_info_window() {
    let mut app = setup_app();
    assert!(!app.state.ui_state.show_info_window);

    // Toggle info window on
    app.handle_message(Message::ToggleInfoWindow);
    assert!(app.state.ui_state.show_info_window);

    // Toggle info window off
    app.handle_message(Message::ToggleInfoWindow);
    assert!(!app.state.ui_state.show_info_window);
}

#[test]
fn test_toggle_about_window() {
    let mut app = setup_app();
    assert!(!app.state.ui_state.show_about_window);

    // Toggle about window on
    app.handle_message(Message::ToggleAboutWindow);
    assert!(app.state.ui_state.show_about_window);

    // Toggle about window off
    app.handle_message(Message::ToggleAboutWindow);
    assert!(!app.state.ui_state.show_about_window);
}
