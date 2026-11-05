use cli_superpower_core::{
    apply::{HistoryEntry, PreviewFile},
    desktop::{Desktop, TeamPreview},
};
use serde_json::Value;
use tauri::{Manager, State};

async fn desktop_task<T: Send + 'static>(
    task: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn load_config(desktop: State<'_, Desktop>) -> Result<Value, String> {
    let desktop = desktop.inner().clone();
    desktop_task(move || desktop.load()).await
}

#[tauri::command]
async fn save_config(desktop: State<'_, Desktop>, config: Value) -> Result<(), String> {
    let desktop = desktop.inner().clone();
    desktop_task(move || desktop.save(&config)).await
}

#[tauri::command]
async fn preview_config(
    desktop: State<'_, Desktop>,
    config: Value,
) -> Result<Vec<PreviewFile>, String> {
    let desktop = desktop.inner().clone();
    desktop_task(move || desktop.preview(&config)).await
}

#[tauri::command]
async fn apply_config(
    desktop: State<'_, Desktop>,
    config: Value,
) -> Result<Vec<PreviewFile>, String> {
    let desktop = desktop.inner().clone();
    desktop_task(move || desktop.apply(&config)).await
}

#[tauri::command]
async fn remove_integration(desktop: State<'_, Desktop>) -> Result<(), String> {
    let desktop = desktop.inner().clone();
    desktop_task(move || desktop.remove()).await
}

#[tauri::command]
async fn launch_workspace(desktop: State<'_, Desktop>, id: String) -> Result<(), String> {
    let desktop = desktop.inner().clone();
    desktop_task(move || desktop.launch(&id)).await
}

#[tauri::command]
async fn preview_team_package(
    desktop: State<'_, Desktop>,
    config: Value,
    content: String,
) -> Result<TeamPreview, String> {
    let desktop = desktop.inner().clone();
    desktop_task(move || desktop.preview_team(&config, &content)).await
}

#[tauri::command]
async fn install_team_package(
    desktop: State<'_, Desktop>,
    config: Value,
    content: String,
) -> Result<Value, String> {
    let desktop = desktop.inner().clone();
    desktop_task(move || desktop.install_team(&config, &content)).await
}

#[tauri::command]
async fn list_history(desktop: State<'_, Desktop>) -> Result<Vec<HistoryEntry>, String> {
    let desktop = desktop.inner().clone();
    desktop_task(move || desktop.history()).await
}

#[tauri::command]
async fn restore_latest(desktop: State<'_, Desktop>) -> Result<(), String> {
    let desktop = desktop.inner().clone();
    desktop_task(move || desktop.restore_latest()).await
}

#[tauri::command]
async fn list_tools(
    desktop: State<'_, Desktop>,
) -> Result<Vec<cli_superpower_core::tools::ToolStatus>, String> {
    let desktop = desktop.inner().clone();
    desktop_task(move || Ok(desktop.tool_statuses())).await
}

#[tauri::command]
async fn install_tool(desktop: State<'_, Desktop>, id: String) -> Result<(), String> {
    let desktop = desktop.inner().clone();
    desktop_task(move || desktop.install_tool(&id)).await
}

#[tauri::command]
async fn uninstall_tool(desktop: State<'_, Desktop>, id: String) -> Result<(), String> {
    let desktop = desktop.inner().clone();
    desktop_task(move || desktop.uninstall_tool(&id)).await
}

#[tauri::command]
async fn install_recommended_tools(desktop: State<'_, Desktop>) -> Result<Vec<String>, String> {
    let desktop = desktop.inner().clone();
    desktop_task(move || {
        desktop
            .install_recommended_tools()
            .map(|names| names.into_iter().map(str::to_owned).collect())
    })
    .await
}

#[tauri::command]
async fn upload_background(
    desktop: State<'_, Desktop>,
    data: String,
) -> Result<std::path::PathBuf, String> {
    let desktop = desktop.inner().clone();
    desktop_task(move || desktop.upload_background(&data)).await
}

#[tauri::command]
async fn background_preview(
    desktop: State<'_, Desktop>,
    path: std::path::PathBuf,
) -> Result<Option<String>, String> {
    let desktop = desktop.inner().clone();
    desktop_task(move || desktop.background_preview(&path)).await
}

#[tauri::command]
async fn terminal_profiles(desktop: State<'_, Desktop>) -> Result<Vec<String>, String> {
    let desktop = desktop.inner().clone();
    desktop_task(move || desktop.terminal_profiles()).await
}

#[tauri::command]
fn open_link(target: String) -> Result<(), String> {
    let url = match target.as_str() {
        "github" => "https://github.com/anto88ct/CLI-superpower",
        "website" => "https://anto88ct.github.io/CLI-superpower",
        _ => return Err("collegamento non consentito".into()),
    };
    std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data = app.path().data_dir()?;
            let local = app.path().local_data_dir()?;
            let clink = app.path().resource_dir()?.join("clink");
            app.manage(
                Desktop::new(
                    data.join("CliSuperpower"),
                    local.join("CliSuperpower"),
                    local,
                )
                .with_clink_bundle(clink),
            );
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            preview_config,
            apply_config,
            remove_integration,
            launch_workspace,
            preview_team_package,
            install_team_package,
            list_history,
            restore_latest,
            list_tools,
            install_tool,
            uninstall_tool,
            install_recommended_tools,
            upload_background,
            background_preview,
            terminal_profiles,
            open_link
        ])
        .run(tauri::generate_context!())
        .expect("errore nell'avvio di CLI Superpower");
}
