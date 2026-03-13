use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    App,
};

pub fn setup_tray(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let quit_item = MenuItem::with_id(app, "quit", "Quit PrivacyHUD", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quit_item])?;

    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("PrivacyHUD — right-click to quit")
        .on_menu_event(|app, event| {
            if event.id().as_ref() == "quit" {
                app.exit(0);
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;

    Ok(())
}
