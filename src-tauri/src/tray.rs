use tauri::{
    menu::{CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    App, Manager, Runtime,
};

use crate::{commands, state::AppState};

pub fn create_tray<R: Runtime>(app: &App<R>) -> tauri::Result<()> {
    let open = MenuItemBuilder::with_id("open-settings", "打开设置").build(app)?;
    let toggle = MenuItemBuilder::with_id("toggle-crosshair", "显示／隐藏准星").build(app)?;
    let launch_at_login = app
        .state::<AppState>()
        .settings
        .lock()
        .map(|settings| settings.launch_at_login)
        .unwrap_or(false);
    let autostart = CheckMenuItemBuilder::with_id("autostart", "开机启动")
        .checked(launch_at_login)
        .build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "退出 Dead Center").build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[&open, &toggle, &autostart])
        .separator()
        .item(&quit)
        .build()?;
    let autostart_item = autostart.clone();

    let mut builder = TrayIconBuilder::with_id("dead-center-tray")
        .menu(&menu)
        .tooltip("Dead Center")
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "open-settings" => {
                let _ = commands::show_settings(app);
            }
            "toggle-crosshair" => {
                let _ = commands::toggle_visibility(app);
            }
            "autostart" => {
                if let Ok(enabled) = commands::toggle_autostart(app) {
                    let _ = autostart_item.set_checked(enabled);
                }
            }
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}
