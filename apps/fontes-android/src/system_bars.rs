//! Read Android View insets on its UI thread without an app-authored Java layer.
use jni::{
    jni_sig, jni_str,
    objects::{JObject, JValue},
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
use winit::platform::android::activity::AndroidApp;

#[derive(Clone, Copy, Debug)]
pub(super) struct Sample {
    pub size: [i32; 2],
    pub content: [i32; 4],
}

#[derive(Default)]
pub(super) struct SystemBars {
    sample: Arc<Mutex<Option<Sample>>>,
    pending: Arc<AtomicBool>,
    styled: Arc<Mutex<Option<(bool, [u8; 4])>>>,
    last_poll: Option<Instant>,
}

impl SystemBars {
    pub fn poll(&mut self, app: &AndroidApp, ctx: &eframe::egui::Context) -> Option<Sample> {
        if self
            .last_poll
            .is_none_or(|t| t.elapsed() >= Duration::from_millis(250))
            && !self.pending.swap(true, Ordering::AcqRel)
        {
            self.last_poll = Some(Instant::now());
            let activity = app.clone();
            let sample = self.sample.clone();
            let pending = self.pending.clone();
            let styled = self.styled.clone();
            let appearance = fontes_ui_egui::system_bar_appearance(ctx);
            let wake = ctx.clone();
            app.run_on_java_main_thread(Box::new(move || {
                if *styled.lock().unwrap() != Some(appearance) {
                    match style_bars(&activity, appearance) {
                        Ok(()) => *styled.lock().unwrap() = Some(appearance),
                        Err(error) => log::debug!("System-bar appearance unavailable: {error}"),
                    }
                }
                match read(&activity) {
                    Ok(value) => *sample.lock().unwrap() = value,
                    Err(error) => log::debug!("WindowInsets unavailable: {error}"),
                }
                pending.store(false, Ordering::Release);
                wake.request_repaint();
            }));
        }
        *self.sample.lock().unwrap()
    }
}

/// Match OS-owned foregrounds to the selected theme. Preserve unrelated flags
/// on API 28–29; API 30+ uses the dedicated appearance mask.
fn style_bars(
    app: &AndroidApp,
    (light_background, color): (bool, [u8; 4]),
) -> jni::errors::Result<()> {
    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) };
    vm.attach_current_thread(|env| -> jni::errors::Result<_> {
        let raw = app.activity_as_ptr() as jni::sys::jobject;
        let activity = unsafe { env.as_cast_raw::<JObject>(&raw)? };
        let window = env
            .call_method(
                activity,
                jni_str!("getWindow"),
                jni_sig!(() -> android.view.Window),
                &[],
            )?
            .l()?;
        let decor = env
            .call_method(
                &window,
                jni_str!("getDecorView"),
                jni_sig!(() -> android.view.View),
                &[],
            )?
            .l()?;
        let sdk = env
            .get_static_field(
                jni_str!("android/os/Build$VERSION"),
                jni_str!("SDK_INT"),
                jni_sig!(jint),
            )?
            .i()?;
        if sdk >= 30 {
            let controller = env
                .call_method(
                    &decor,
                    jni_str!("getWindowInsetsController"),
                    jni_sig!(() -> android.view.WindowInsetsController),
                    &[],
                )?
                .l()?;
            if controller.is_null() {
                return Err(jni::errors::Error::NullPtr("WindowInsetsController"));
            }
            let mask = (1 << 3) | (1 << 4); // APPEARANCE_LIGHT_STATUS_BARS | LIGHT_NAVIGATION_BARS
            env.call_method(
                &controller,
                jni_str!("setSystemBarsAppearance"),
                jni_sig!((jint, jint)),
                &[
                    JValue::Int(if light_background { mask } else { 0 }),
                    JValue::Int(mask),
                ],
            )?;
        } else {
            let flags = env
                .call_method(
                    &decor,
                    jni_str!("getSystemUiVisibility"),
                    jni_sig!(() -> jint),
                    &[],
                )?
                .i()?;
            // SYSTEM_UI_FLAG_LIGHT_STATUS_BAR | SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR
            env.call_method(
                &decor,
                jni_str!("setSystemUiVisibility"),
                jni_sig!((jint)),
                &[JValue::Int(if light_background {
                    flags | 0x2000 | 0x10
                } else {
                    flags & !(0x2000 | 0x10)
                })],
            )?;
        }
        if sdk < 35 {
            // Target-35 edge-to-edge bars use the shared app clear color on newer Android.
            let [r, g, b, a] = color;
            let argb = u32::from_be_bytes([a, r, g, b]) as i32;
            for method in [
                jni_str!("setStatusBarColor"),
                jni_str!("setNavigationBarColor"),
            ] {
                env.call_method(&window, method, jni_sig!((jint)), &[JValue::Int(argb)])?;
            }
        }
        Ok(())
    })
}

pub(super) fn read(app: &AndroidApp) -> jni::errors::Result<Option<Sample>> {
    // AndroidApp owns both pointers throughout this callback. The cast borrows
    // its Activity reference and never takes ownership or deletes it.
    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) };
    vm.attach_current_thread(|env| -> jni::errors::Result<_> {
        let raw = app.activity_as_ptr() as jni::sys::jobject;
        let activity = unsafe { env.as_cast_raw::<JObject>(&raw)? };
        let window = env
            .call_method(
                activity,
                jni_str!("getWindow"),
                jni_sig!(() -> android.view.Window),
                &[],
            )?
            .l()?;
        let decor = env
            .call_method(
                &window,
                jni_str!("getDecorView"),
                jni_sig!(() -> android.view.View),
                &[],
            )?
            .l()?;
        let insets = env
            .call_method(
                &decor,
                jni_str!("getRootWindowInsets"),
                jni_sig!(() -> android.view.WindowInsets),
                &[],
            )?
            .l()?;
        if insets.is_null() {
            return Ok(None);
        }
        let width = env
            .call_method(&decor, jni_str!("getWidth"), jni_sig!(() -> jint), &[])?
            .i()?;
        let height = env
            .call_method(&decor, jni_str!("getHeight"), jni_sig!(() -> jint), &[])?
            .i()?;
        let sdk = env
            .get_static_field(
                jni_str!("android/os/Build$VERSION"),
                jni_str!("SDK_INT"),
                jni_sig!(jint),
            )?
            .i()?;
        let mut edges = [0; 4];
        if sdk >= 30 {
            let bars = env
                .call_static_method(
                    jni_str!("android/view/WindowInsets$Type"),
                    jni_str!("systemBars"),
                    jni_sig!(() -> jint),
                    &[],
                )?
                .i()?;
            let cutout = env
                .call_static_method(
                    jni_str!("android/view/WindowInsets$Type"),
                    jni_str!("displayCutout"),
                    jni_sig!(() -> jint),
                    &[],
                )?
                .i()?;
            let values = env
                .call_method(
                    &insets,
                    jni_str!("getInsets"),
                    jni_sig!((jint) -> android.graphics.Insets),
                    &[JValue::Int(bars | cutout)],
                )?
                .l()?;
            for (edge, name) in edges.iter_mut().zip([
                jni_str!("left"),
                jni_str!("top"),
                jni_str!("right"),
                jni_str!("bottom"),
            ]) {
                *edge = env.get_field(&values, name, jni_sig!(jint))?.i()?;
            }
        } else {
            // API 28–29: stable system-bar insets exclude the IME; native content
            // geometry/window resizing handles keyboard changes separately.
            for (edge, name) in edges.iter_mut().zip([
                jni_str!("getStableInsetLeft"),
                jni_str!("getStableInsetTop"),
                jni_str!("getStableInsetRight"),
                jni_str!("getStableInsetBottom"),
            ]) {
                *edge = env
                    .call_method(&insets, name, jni_sig!(() -> jint), &[])?
                    .i()?;
            }
            let cutout = env
                .call_method(
                    &insets,
                    jni_str!("getDisplayCutout"),
                    jni_sig!(() -> android.view.DisplayCutout),
                    &[],
                )?
                .l()?;
            if !cutout.is_null() {
                for (edge, name) in edges.iter_mut().zip([
                    jni_str!("getSafeInsetLeft"),
                    jni_str!("getSafeInsetTop"),
                    jni_str!("getSafeInsetRight"),
                    jni_str!("getSafeInsetBottom"),
                ]) {
                    *edge = (*edge).max(
                        env.call_method(&cutout, name, jni_sig!(() -> jint), &[])?
                            .i()?,
                    );
                }
            }
        }
        Ok(Some(Sample {
            size: [width, height],
            content: [edges[0], edges[1], width - edges[2], height - edges[3]],
        }))
    })
}
