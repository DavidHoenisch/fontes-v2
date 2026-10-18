use fontes_engine::{Command, Response, bootstrap, worker};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::{Manager, State};
struct Backend {
    host: Mutex<worker::Host>,
    data_dir: PathBuf,
    updating: Arc<AtomicBool>,
    cancel_update: Arc<AtomicBool>,
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
// Update work never holds the engine lock while fetching or validating content.
struct UpdateGuard(Arc<AtomicBool>);
impl Drop for UpdateGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
fn begin_update(backend: &Backend) -> Result<UpdateGuard, String> {
    backend
        .updating
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .map_err(|_| "A resource operation is already running.".to_string())?;
    backend.cancel_update.store(false, Ordering::SeqCst);
    Ok(UpdateGuard(backend.updating.clone()))
}
#[tauri::command]
async fn library_check_update(
    backend: State<'_, Backend>,
) -> Result<bootstrap::UpdateInfo, String> {
    let guard = begin_update(&backend)?;
    let path = backend.data_dir.clone();
    let cancel = backend.cancel_update.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        bootstrap::check_update(&path, &cancel).map_err(|e| format!("{e:#}"))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn library_update(
    backend: State<'_, Backend>,
    manifest_sha256: String,
    progress: tauri::ipc::Channel<bootstrap::Progress>,
) -> Result<(), String> {
    let guard = begin_update(&backend)?;
    let path = backend.data_dir.clone();
    let cancel = backend.cancel_update.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let last_progress = Mutex::new((std::time::Instant::now(), String::new()));
        bootstrap::install_update(&path, &manifest_sha256, &cancel, |p| {
            // Keep IPC/render work bounded during fast decompression.
            let mut last = last_progress.lock().unwrap();
            if last.1 != p.message
                || last.0.elapsed().as_millis() >= 100
                || p.total == Some(p.completed)
            {
                last.0 = std::time::Instant::now();
                last.1 = p.message.clone();
                let _ = progress.send(p);
            }
        })
        .map(|_| ())
        .map_err(|e| format!("{e:#}"))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn library_cancel_update(backend: State<'_, Backend>) {
    backend.cancel_update.store(true, Ordering::SeqCst);
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
                updating: Arc::new(AtomicBool::new(false)),
                cancel_update: Arc::new(AtomicBool::new(false)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            engine_dispatch,
            library_open,
            library_install,
            library_check_update,
            library_update,
            library_cancel_update
        ])
        .run(tauri::generate_context!())
        .expect("Fontes startup failed");
}
