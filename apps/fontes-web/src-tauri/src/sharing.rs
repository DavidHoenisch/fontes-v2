pub fn share(
    webview: &tauri::WebviewWindow,
    data_url: String,
    caption: String,
) -> Result<(), String> {
    if !data_url.starts_with("data:image/png;base64,")
        || data_url.len() > 8_000_000
        || caption.len() > 20_000
    {
        return Err("Invalid or oversized share image/caption.".into());
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = webview;
        Err("Native image sharing is currently available on Android. Browser preview supports PNG download.".into())
    }
    #[cfg(target_os = "android")]
    {
        use tauri::wry::prelude::jni::objects::{JString, JValue};
        let (sender, receiver) = std::sync::mpsc::channel();
        webview
            .with_webview(move |platform| {
                platform.jni_handle().exec(move |env, activity, _| {
                    let result = (|| {
                        let png = env.new_string(&data_url).map_err(|e| e.to_string())?;
                        let text = env.new_string(&caption).map_err(|e| e.to_string())?;
                        let value = env
                            .call_method(
                                activity,
                                "shareVerseImage",
                                "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
                                &[JValue::Object(&png), JValue::Object(&text)],
                            )
                            .map_err(|e| e.to_string())?;
                        let value = JString::from(value.l().map_err(|e| e.to_string())?);
                        let error: String =
                            env.get_string(&value).map_err(|e| e.to_string())?.into();
                        if error.is_empty() { Ok(()) } else { Err(error) }
                    })();
                    if result.is_err() {
                        let _ = env.exception_clear();
                    }
                    let _ = sender.send(result);
                });
            })
            .map_err(|e| e.to_string())?;
        receiver
            .recv_timeout(std::time::Duration::from_secs(15))
            .map_err(|_| "Android share options did not respond.".to_string())?
    }
}
