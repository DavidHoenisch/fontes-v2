//! Serial background engine host, usable by any native frontend. No UI or GPU dependencies.
use crate::{Command, Engine, Response};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, SyncSender},
};
pub enum Request {
    Open { library: PathBuf, user_db: PathBuf },
    OpenInstalled { data_dir: PathBuf, user_db: PathBuf },
    Dispatch(Command),
}
pub struct Host {
    pub requests: SyncSender<Request>,
    pub events: Receiver<std::result::Result<Response, String>>,
}
pub fn spawn(wake: impl Fn() + Send + 'static) -> Host {
    let (tx, rx) = mpsc::sync_channel(32);
    let (events, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut engine: Option<Engine> = None;
        while let Ok(request) = rx.recv() {
            let result = match request {
                Request::Open { library, user_db } => match Engine::open(&library, &user_db) {
                    Ok(mut e) => {
                        let result = e.dispatch(Command::Workspace {});
                        if result.is_ok() {
                            engine = Some(e);
                        }
                        result.map_err(|e| e.to_string())
                    }
                    Err(e) => Err(e.to_string()),
                },
                Request::OpenInstalled { data_dir, user_db } => {
                    crate::bootstrap::verify_installed(&data_dir)
                        .and_then(|path| Ok(Engine::open(&path, &user_db)?))
                        .and_then(|mut e| {
                            let result = e.dispatch(Command::Workspace {});
                            if result.is_ok() {
                                engine = Some(e);
                            }
                            Ok(result?)
                        })
                        .map_err(|e| format!("{e:#}"))
                }
                Request::Dispatch(command) => engine
                    .as_mut()
                    .ok_or_else(|| "Open a local library first.".to_string())
                    .and_then(|e| e.dispatch(command).map_err(|e| e.to_string())),
            };
            if events.send(result).is_err() {
                break;
            }
            wake();
        }
    });
    Host {
        requests: tx,
        events: receiver,
    }
}
