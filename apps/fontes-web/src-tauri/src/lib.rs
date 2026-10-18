use fontes_engine::{Command, Response, bootstrap, worker};
use std::{
    path::PathBuf,
    sync::{Mutex, atomic::AtomicBool},
};
use tauri::{Manager, State};
struct Backend {
    host: Mutex<worker::Host>,
    data_dir: PathBuf,
}
fn exchange(host: &mut worker::Host, request: worker::Request) -> Result<Response, String> {
    host.requests
        .send(request)
        .map_err(|_| "Engine stopped".to_string())?;
    host.events
        .recv()
        .map_err(|_| "Engine stopped".to_string())?
}
#[tauri::command]
async fn engine_dispatch(
    backend: State<'_, Backend>,
    command: Command,
) -> Result<Response, String> {
    let mut host = backend.host.lock().map_err(|_| "Engine unavailable")?;
    exchange(&mut host, worker::Request::Dispatch(command))
}
#[derive(serde::Serialize)]
struct OpenError {
    kind: &'static str,
    message: String,
}
#[tauri::command]
async fn library_open(backend: State<'_, Backend>) -> Result<Response, OpenError> {
    open_library(&backend).map_err(|message| OpenError {
        kind: if message == "No library installed." {
            "missing"
        } else {
            "failed"
        },
        message,
    })
}
fn open_library(backend: &Backend) -> Result<Response, String> {
    let data = &backend.data_dir;
    let mut host = backend.host.lock().map_err(|_| "Engine unavailable")?;
    if bootstrap::installed_library(data)
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return exchange(
            &mut host,
            worker::Request::OpenInstalled {
                data_dir: data.clone(),
                user_db: data.join("user.sqlite"),
            },
        );
    }
    let path = std::env::var_os("FONTES_LIBRARY")
        .map(PathBuf::from)
        .or_else(|| {
            std::fs::read_to_string(data.join("library-path.txt"))
                .ok()
                .filter(|s| s.len() <= 4096)
                .map(|s| PathBuf::from(s.trim()))
        })
        .unwrap_or_else(|| data.join("library.sqlite"));
    if !path.is_file() {
        return Err("No library installed.".into());
    }
    exchange(
        &mut host,
        worker::Request::Open {
            library: path,
            user_db: data.join("user.sqlite"),
        },
    )
}
#[tauri::command]
async fn library_install(backend: State<'_, Backend>) -> Result<(), String> {
    let path = backend.data_dir.clone();
    tauri::async_runtime::spawn_blocking(move || {
        bootstrap::install(
            &path,
            bootstrap::Profile::default(),
            &AtomicBool::new(false),
            |_| {},
        )
        .map(|_| ())
        .map_err(|e| format!("{e:#}"))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(not(target_os = "android"))]
            let data_dir = std::env::var_os("FONTES_DATA_DIR")
                .map(PathBuf::from)
                .or_else(|| {
                    directories::ProjectDirs::from("org", "Fontes", "Fontes")
                        .map(|p| p.data_local_dir().to_path_buf())
                })
                .ok_or("No application data directory")?;
            #[cfg(target_os = "android")]
            let data_dir = app.path().app_data_dir()?.join("files");
            std::fs::create_dir_all(&data_dir)?;
            app.manage(Backend {
                host: Mutex::new(worker::spawn(|| {})),
                data_dir,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            engine_dispatch,
            library_open,
            library_install
        ])
        .run(tauri::generate_context!())
        .expect("Fontes startup failed");
}
