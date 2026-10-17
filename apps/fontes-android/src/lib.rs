//! NativeActivity edge only. The same engine and UI run on desktop and Android.
#[cfg(any(target_os = "android", test))]
mod insets;
#[cfg(target_os = "android")]
mod native;
#[cfg(target_os = "android")]
mod system_bars;
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: winit::platform::android::activity::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    let Some(data_dir) = app.internal_data_path() else {
        log::error!("Android private app storage is unavailable");
        return;
    };
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        android_app: Some(app.clone()),
        ..Default::default()
    };
    if let Err(e) = eframe::run_native(
        "Fontes",
        options,
        Box::new(move |cc| Ok(Box::new(native::AndroidFontesApp::new(cc, app, data_dir)))),
    ) {
        log::error!("Fontes stopped: {e}");
    }
}
