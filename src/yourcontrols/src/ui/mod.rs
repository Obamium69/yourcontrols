// UI Backend Abstraction Layer
//
// This module provides a trait-based abstraction for different UI backends
// (WebView, egui, etc.) to enable cross-platform compatibility and flexibility.

use crossbeam_channel::TryRecvError;
use laminar::Metrics;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

// Re-export backends based on feature flags
#[cfg(feature = "webview-ui")]
pub mod webview;

#[cfg(feature = "egui-ui")]
pub mod egui_backend;

// Re-export the active backend
#[cfg(feature = "webview-ui")]
pub use webview::WebViewBackend as ActiveBackend;

#[cfg(feature = "egui-ui")]
pub use egui_backend::EguiBackend as ActiveBackend;

/// Connection method for server/client
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionMethod {
    Direct,
    Relay,
    CloudServer,
}

/// Messages sent FROM the UI TO the application
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AppMessage {
    /// Start a server
    StartServer {
        username: String,
        is_ipv6: bool,
        use_upnp: bool,
        port: u16,
        method: ConnectionMethod,
    },
    /// Connect to a server
    Connect {
        username: String,
        session_id: Option<String>,
        isipv6: bool,
        ip: Option<IpAddr>,
        hostname: Option<String>,
        port: Option<u16>,
        method: ConnectionMethod,
    },
    /// Transfer control to another client
    TransferControl { target: String },
    /// Set observer mode for a client
    SetObserver { target: String, is_observer: bool },
    /// Load an aircraft configuration
    LoadAircraft { config_file_name: String },
    /// Disconnect from server/stop server
    Disconnect,
    /// Application startup event
    Startup,
    /// Run the updater
    RunUpdater,
    /// Force take control
    ForceTakeControl,
    /// Update configuration
    UpdateConfig {
        new_config: crate::simconfig::Config,
    },
    /// Go into observer mode
    GoObserver,
}

/// UI Backend trait - all UI implementations must implement this
///
/// This trait defines the interface between the application logic and the UI layer.
/// It provides methods for:
/// - Lifecycle management (setup, exit detection)
/// - Message passing (UI → App via get_next_message)
/// - UI updates (App → UI via invoke and convenience methods)
pub trait UIBackend: Send {
    /// Create and initialize the UI backend
    ///
    /// # Arguments
    /// * `title` - Window title (e.g., "YourControls v2.8.5")
    ///
    /// # Returns
    /// A new instance of the UI backend
    fn setup(title: String) -> Self
    where
        Self: Sized;

    /// Check if the UI has been closed/exited
    ///
    /// # Returns
    /// `true` if the user closed the window, `false` otherwise
    fn exited(&self) -> bool;

    /// Poll for the next message from the UI
    ///
    /// This is called in the main event loop to receive user actions.
    ///
    /// # Returns
    /// - `Ok(AppMessage)` if a message is available
    /// - `Err(TryRecvError::Empty)` if no messages are pending
    /// - `Err(TryRecvError::Disconnected)` if the UI thread terminated
    fn get_next_message(&self) -> Result<AppMessage, TryRecvError>;

    // ============================================================================
    // UI Update Methods
    // ============================================================================
    // These methods are called by the application to update the UI state.
    // All methods are non-blocking and return immediately.

    /// Generic invoke method for sending typed messages to the UI
    ///
    /// # Arguments
    /// * `type_string` - Message type identifier
    /// * `data` - Optional message payload (often JSON-stringified)
    fn invoke(&self, type_string: &str, data: Option<&str>);

    // --- Error and Status Messages ---

    /// Display an error message
    fn error(&self, msg: &str) {
        self.invoke("error", Some(msg));
    }

    /// Show "attempting connection" status
    fn attempt(&self) {
        self.invoke("attempt", None);
    }

    /// Show "connected to server" status (client side)
    fn connected(&self) {
        self.invoke("connected", None);
    }

    /// Show server start failure
    fn server_fail(&self, reason: &str) {
        self.invoke("server_fail", Some(reason));
    }

    /// Show client connection failure
    fn client_fail(&self, reason: &str) {
        self.invoke("client_fail", Some(reason));
    }

    // --- Control State ---

    /// Notify UI that we gained control of the aircraft
    fn gain_control(&self) {
        self.invoke("control", None);
    }

    /// Notify UI that we lost control of the aircraft
    fn lose_control(&self) {
        self.invoke("lostcontrol", None);
    }

    // --- Server State ---

    /// Notify UI that server started successfully
    fn server_started(&self) {
        self.invoke("server", None);
    }

    /// Set the session code for cloud connections
    fn set_session_code(&self, code: &str) {
        self.invoke("session", Some(code));
    }

    /// Notify UI that we became the host (relay mode)
    fn set_host(&self) {
        self.invoke("host", None);
    }

    // --- Connection Management ---

    /// Notify UI that a new client connected
    fn new_connection(&self, name: &str) {
        self.invoke("newconnection", Some(name));
    }

    /// Notify UI that a client disconnected
    fn lost_connection(&self, name: &str) {
        self.invoke("lostconnection", Some(name));
    }

    // --- Observer Mode ---

    /// Set our own observer mode state
    fn observing(&self, observing: bool) {
        if observing {
            self.invoke("observing", None);
        } else {
            self.invoke("stop_observing", None);
        }
    }

    /// Set another client's observer mode state
    fn set_observing(&self, name: &str, observing: bool) {
        if observing {
            self.invoke("set_observing", Some(name));
        } else {
            self.invoke("set_not_observing", Some(name));
        }
    }

    /// Set which client is in control
    fn set_incontrol(&self, name: &str) {
        self.invoke("set_incontrol", Some(name));
    }

    // --- Configuration ---

    /// Add an aircraft to the selection list
    fn add_aircraft(&self, name: &str) {
        self.invoke("add_aircraft", Some(name));
    }

    /// Notify UI of available update version
    fn version(&self, version: &str) {
        self.invoke("version", Some(version));
    }

    /// Notify UI that update download failed
    fn update_failed(&self) {
        self.invoke("update_failed", None);
    }

    /// Send configuration data to UI
    fn send_config(&self, value: &str) {
        self.invoke("config_msg", Some(value));
    }

    // --- Network Statistics ---

    /// Send network metrics to UI
    fn send_network(&self, metrics: &Metrics) {
        use serde_json::json;
        let data = json!({
            "sentPackets": metrics.sent_packets,
            "receivePackets": metrics.received_packets,
            "sentBandwidth": metrics.sent_kbps,
            "receiveBandwidth": metrics.receive_kbps,
            "packetLoss": metrics.packet_loss,
            "ping": metrics.rtt / 2.0
        });
        self.invoke("metrics", Some(&data.to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::{unbounded, Receiver, Sender};
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    };

    /// Mock UI backend for testing
    struct MockBackend {
        exited: Arc<AtomicBool>,
        rx: Receiver<AppMessage>,
        invocations: Arc<Mutex<Vec<(String, Option<String>)>>>,
    }

    impl UIBackend for MockBackend {
        fn setup(_title: String) -> Self {
            let (_tx, rx) = unbounded();
            Self {
                exited: Arc::new(AtomicBool::new(false)),
                rx,
                invocations: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn exited(&self) -> bool {
            self.exited.load(Ordering::SeqCst)
        }

        fn get_next_message(&self) -> Result<AppMessage, TryRecvError> {
            self.rx.try_recv()
        }

        fn invoke(&self, type_string: &str, data: Option<&str>) {
            let mut invocations = self.invocations.lock().unwrap();
            invocations.push((type_string.to_string(), data.map(|s| s.to_string())));
        }
    }

    #[test]
    fn test_mock_backend_creation() {
        let backend = MockBackend::setup("Test".to_string());
        assert!(!backend.exited());
    }

    #[test]
    fn test_invoke_recording() {
        let backend = MockBackend::setup("Test".to_string());
        backend.error("test error");
        backend.connected();

        let invocations = backend.invocations.lock().unwrap();
        assert_eq!(invocations.len(), 2);
        assert_eq!(invocations[0].0, "error");
        assert_eq!(invocations[0].1, Some("test error".to_string()));
        assert_eq!(invocations[1].0, "connected");
        assert_eq!(invocations[1].1, None);
    }

    #[test]
    fn test_connection_method_serialization() {
        let method = ConnectionMethod::Direct;
        let json = serde_json::to_string(&method).unwrap();
        assert_eq!(json, r#""direct"#);

        let method = ConnectionMethod::CloudServer;
        let json = serde_json::to_string(&method).unwrap();
        assert_eq!(json, r#""cloudServer"#);
    }
}
