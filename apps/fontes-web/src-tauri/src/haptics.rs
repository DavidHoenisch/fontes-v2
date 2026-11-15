// A fixed, best-effort presentation cue; Android owns its playback and settings.
pub fn arrival(webview: &tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(not(target_os = "android"))]
    {
        let _ = webview;
        Ok(())
    }
    #[cfg(target_os = "android")]
    {
        use tauri::wry::prelude::jni::objects::JValue;

        webview
            .with_webview(|platform| {
                platform.jni_handle().exec(|env, activity, view| {
                    // The activity schedules/cancels the sequence on this UI thread.
                    if env
                        .call_method(
                            activity,
                            "performVerseArrivalHaptics",
                            "(Landroid/view/View;)V",
                            &[JValue::Object(view)],
                        )
                        .is_err()
                    {
                        // An unavailable cue must not leave a JNI exception behind.
                        let _ = env.exception_clear();
                    }
                });
            })
            .map_err(|error| error.to_string())
    }
}
