// WebView UI Backend Implementation
//
// This module wraps the existing WebView-based UI to implement the UIBackend trait.
// It maintains backward compatibility with the original implementation.

use super::{AppMessage, UIBackend};
use base64::Engine;
use crossbeam_channel::{unbounded, Receiver, TryRecvError};
use std::fs::File;
use std::io::Read;
use std::sync::{
    atomic::{AtomicBool, Ordering::SeqCst},
    Arc, Mutex,
};
use std::thread;

/// WebView-based UI backend
pub struct WebViewBackend {
    app_handle: Arc<Mutex<Option<web_view::Handle<i32>>>>,
    exited: Arc<AtomicBool>,
    rx: Receiver<AppMessage>,
}

impl UIBackend for WebViewBackend {
    fn setup(title: String) -> Self {
        let (tx, rx) = unbounded();

        let mut logo = vec![];
        File::open("assets/logo.png")
            .unwrap()
            .read_to_end(&mut logo)
            .ok();

        let handle = Arc::new(Mutex::new(None));
        let handle_clone = handle.clone();
        let exited = Arc::new(AtomicBool::new(false));
        let exited_clone = exited.clone();

        thread::spawn(move || {
            let webview = web_view::builder()
                .title(&title)
                .content(web_view::Content::Html(format!(
                    r##"<!DOCTYPE html>
                <html>
                <head>
                    <style>
                        {bootstrapcss}
                        {css}
                    </style>
                </head>
                    <body class="themed">
                    <img src="data:image/png;base64,{logo}" class="logo-image"/>
                    {body}
                </body>
                <script>
                    {jquery}
                    {bootstrapjs}
                    {js1}
                    {js}
                </script>
                </html>
            "##,
                    css = include_str!("../../web/stylesheet.css"),
                    js = include_str!("../../web/main.js"),
                    js1 = include_str!("../../web/list.js"),
                    body = include_str!("../../web/index.html"),
                    jquery = include_str!("../../web/jquery.min.js"),
                    bootstrapjs = include_str!("../../web/bootstrap.bundle.min.js"),
                    bootstrapcss = include_str!("../../web/bootstrap.min.css"),
                    logo = base64::engine::general_purpose::STANDARD_NO_PAD.encode(logo.as_slice())
                )))
                .invoke_handler(move |_, arg| {
                    tx.try_send(serde_json::from_str(arg).unwrap()).ok();
                    Ok(())
                })
                .user_data(0)
                .resizable(true)
                .size(1000, 800)
                .build()
                .unwrap();

            let mut handle = handle_clone.lock().unwrap();
            *handle = Some(webview.handle());
            std::mem::drop(handle);

            webview.run().ok();
            exited_clone.store(true, SeqCst);
        });

        Self {
            app_handle: handle,
            exited,
            rx,
        }
    }

    fn exited(&self) -> bool {
        self.exited.load(SeqCst)
    }

    fn get_next_message(&self) -> Result<AppMessage, TryRecvError> {
        self.rx.try_recv()
    }

    fn invoke(&self, type_string: &str, data: Option<&str>) {
        let handle = self.app_handle.lock().unwrap();
        if handle.is_none() {
            return;
        }
        // Send data to javascript
        let data = data.unwrap_or_default().to_string();
        let type_string = type_string.to_owned();
        handle
            .as_ref()
            .unwrap()
            .dispatch(move |webview| {
                webview.eval(&get_message_str(&type_string, &data)).ok();
                Ok(())
            })
            .ok();
    }
}

/// Helper function to construct JavaScript message
fn get_message_str(type_string: &str, data: &str) -> String {
    format!(
        r#"MessageReceived({})"#,
        serde_json::json!({"type": type_string, "data": data})
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_message_str() {
        let result = get_message_str("error", "test message");
        assert!(result.contains("MessageReceived"));
        assert!(result.contains("error"));
        assert!(result.contains("test message"));
    }

    #[test]
    fn test_get_message_str_empty_data() {
        let result = get_message_str("connected", "");
        assert!(result.contains("MessageReceived"));
        assert!(result.contains("connected"));
    }
}
