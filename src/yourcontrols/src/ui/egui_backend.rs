// egui UI Backend

use super::{AppMessage, ConnectionMethod, UIBackend};
use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};
use eframe::egui;
use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

// egui-based UI backend
pub struct EguiBackend {
    exited: Arc<AtomicBool>,
    rx: Receiver<AppMessage>,
    event_tx: Sender<UiEvent>,
}

// Events sent from the application to the UI
#[derive(Debug, Clone)]
pub enum UiEvent {
    Error(String),
    Attempt,
    Connected,
    ServerFail(String),
    ClientFail(String),
    GainControl,
    LoseControl,
    ServerStarted,
    SessionCode(String),
    SetHost,
    NewConnection(String),
    LostConnection(String),
    Observing(bool),
    SetObserving {
        name: String,
        observing: bool,
    },
    SetInControl(String),
    AddAircraft(String),
    Version(String),
    UpdateFailed,
    SendConfig(String),
    SendMetrics {
        sent_packets: u64,
        received_packets: u64,
        sent_kbps: f32,
        receive_kbps: f32,
        packet_loss: f32,
        ping: f32,
    },
}

impl UIBackend for EguiBackend {
    fn setup(title: String) -> Self {
        let (action_tx, action_rx) = unbounded();
        let (event_tx, event_rx) = unbounded();

        let exited = Arc::new(AtomicBool::new(false));
        let exited_clone = exited.clone();

        // Spawn egui window in separate thread
        std::thread::spawn(move || {
            use eframe::egui::ViewportBuilder;

            #[cfg(target_os = "windows")]
            let event_loop_builder = {
                use winit::platform::windows::EventLoopBuilderExtWindows;
                Some(Box::new(
                    |builder: &mut winit::event_loop::EventLoopBuilder<eframe::UserEvent>| {
                        builder.with_any_thread(true);
                    },
                )
                    as Box<
                        dyn FnOnce(&mut winit::event_loop::EventLoopBuilder<eframe::UserEvent>),
                    >)
            };

            #[cfg(not(target_os = "windows"))]
            let event_loop_builder = None;

            let options = eframe::NativeOptions {
                viewport: ViewportBuilder::default()
                    .with_title(&title)
                    .with_inner_size([1000.0, 800.0])
                    .with_min_inner_size([800.0, 600.0]),
                event_loop_builder,
                ..Default::default()
            };

            let app = YourControlsApp::new(action_tx, event_rx);

            if let Err(e) = eframe::run_native(&title, options, Box::new(|_cc| Ok(Box::new(app)))) {
                eprintln!("egui error: {}", e);
            }

            exited_clone.store(true, Ordering::SeqCst);
        });

        Self {
            exited,
            rx: action_rx,
            event_tx,
        }
    }

    fn exited(&self) -> bool {
        self.exited.load(Ordering::SeqCst)
    }

    fn get_next_message(&self) -> Result<AppMessage, TryRecvError> {
        self.rx.try_recv()
    }

    fn invoke(&self, type_string: &str, data: Option<&str>) {
        let event = match type_string {
            "error" => UiEvent::Error(data.unwrap_or("Unknown error").to_string()),
            "attempt" => UiEvent::Attempt,
            "connected" => UiEvent::Connected,
            "server_fail" => UiEvent::ServerFail(data.unwrap_or("Unknown reason").to_string()),
            "client_fail" => UiEvent::ClientFail(data.unwrap_or("Unknown reason").to_string()),
            "control" => UiEvent::GainControl,
            "lostcontrol" => UiEvent::LoseControl,
            "server" => UiEvent::ServerStarted,
            "session" => UiEvent::SessionCode(data.unwrap_or("").to_string()),
            "host" => UiEvent::SetHost,
            "newconnection" => UiEvent::NewConnection(data.unwrap_or("").to_string()),
            "lostconnection" => UiEvent::LostConnection(data.unwrap_or("").to_string()),
            "observing" => UiEvent::Observing(true),
            "stop_observing" => UiEvent::Observing(false),
            "set_observing" => UiEvent::SetObserving {
                name: data.unwrap_or("").to_string(),
                observing: true,
            },
            "set_not_observing" => UiEvent::SetObserving {
                name: data.unwrap_or("").to_string(),
                observing: false,
            },
            "set_incontrol" => UiEvent::SetInControl(data.unwrap_or("").to_string()),
            "add_aircraft" => UiEvent::AddAircraft(data.unwrap_or("").to_string()),
            "version" => UiEvent::Version(data.unwrap_or("").to_string()),
            "update_failed" => UiEvent::UpdateFailed,
            "config_msg" => UiEvent::SendConfig(data.unwrap_or("{}").to_string()),
            "metrics" => {
                if let Some(data) = data {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                        self.event_tx
                            .send(UiEvent::SendMetrics {
                                sent_packets: json["sentPackets"].as_u64().unwrap_or(0),
                                received_packets: json["receivePackets"].as_u64().unwrap_or(0),
                                sent_kbps: json["sentBandwidth"].as_f64().unwrap_or(0.0) as f32,
                                receive_kbps: json["receiveBandwidth"].as_f64().unwrap_or(0.0)
                                    as f32,
                                packet_loss: json["packetLoss"].as_f64().unwrap_or(0.0) as f32,
                                ping: json["ping"].as_f64().unwrap_or(0.0) as f32,
                            })
                            .ok();
                        return;
                    }
                }
                return;
            }
            _ => return, // Unknown event type
        };

        self.event_tx.send(event).ok();
    }
}

// The egui application state
struct YourControlsApp {
    // Communication
    action_tx: Sender<AppMessage>,
    event_rx: Receiver<UiEvent>,
    event_queue: VecDeque<UiEvent>,

    // UI State
    username: String,
    session_code: String,
    port: String,
    ip_input: String,
    is_connected: bool,
    status_message: String,
    server_connection_method: ConnectionMethod,
    client_connection_method: ConnectionMethod,
    is_ipv6: bool,

    // Client list
    clients: Vec<ClientInfo>,

    // Aircraft selection
    selected_aircraft: usize,
    aircraft_list: Vec<String>,

    // Settings
    connection_timeout: String,
    instructor_mode: bool,
    streamer_mode: bool,
    sound_muted: bool,
    dark_theme: bool,

    // Network stats
    download_bandwidth: f32,
    upload_bandwidth: f32,
    packet_loss: f32,
    ping: f32,
}

#[derive(Clone, Debug)]
struct ClientInfo {
    name: String,
    has_control: bool,
    is_observer: bool,
}

impl YourControlsApp {
    fn new(action_tx: Sender<AppMessage>, event_rx: Receiver<UiEvent>) -> Self {
        // Send startup message
        action_tx.send(AppMessage::Startup).ok();

        Self {
            action_tx,
            event_rx,
            event_queue: VecDeque::new(),
            username: String::new(),
            session_code: String::new(),
            port: "7777".to_string(),
            ip_input: String::new(),
            is_connected: false,
            status_message: "Not connected".to_string(),
            server_connection_method: ConnectionMethod::CloudServer,
            client_connection_method: ConnectionMethod::CloudServer,
            is_ipv6: false,
            clients: Vec::new(),
            selected_aircraft: 0,
            aircraft_list: vec!["Select an aircraft...".to_string()],
            connection_timeout: "30".to_string(),
            instructor_mode: false,
            streamer_mode: false,
            sound_muted: false,
            dark_theme: false,
            download_bandwidth: 0.0,
            upload_bandwidth: 0.0,
            packet_loss: 0.0,
            ping: 0.0,
        }
    }

    fn process_events(&mut self) {
        // Process all pending events
        while let Ok(event) = self.event_rx.try_recv() {
            self.event_queue.push_back(event);
        }

        // Handle events
        while let Some(event) = self.event_queue.pop_front() {
            self.handle_event(event);
        }
    }

    fn handle_event(&mut self, event: UiEvent) {
        match event {
            UiEvent::Error(msg) => {
                self.status_message = format!("Error: {}", msg);
                self.is_connected = false;
            }
            UiEvent::Attempt => {
                self.status_message = "Attempting connection...".to_string();
            }
            UiEvent::Connected => {
                self.status_message = "Connected to server".to_string();
                self.is_connected = true;
            }
            UiEvent::ServerFail(reason) => {
                self.status_message = format!("Server failed: {}", reason);
                self.is_connected = false;
            }
            UiEvent::ClientFail(reason) => {
                self.status_message = format!("Client failed: {}", reason);
                self.is_connected = false;
                self.clients.clear();
            }
            UiEvent::GainControl => {
                self.status_message = "You have control".to_string();
            }
            UiEvent::LoseControl => {
                self.status_message = "You lost control".to_string();
            }
            UiEvent::ServerStarted => {
                self.status_message = "Server started".to_string();
                self.is_connected = true;
            }
            UiEvent::SessionCode(code) => {
                self.status_message = format!("Session Code: {}", code);
            }
            UiEvent::SetHost => {
                self.status_message = "You are now hosting".to_string();
            }
            UiEvent::NewConnection(name) => {
                self.clients.push(ClientInfo {
                    name,
                    has_control: false,
                    is_observer: false,
                });
            }
            UiEvent::LostConnection(name) => {
                self.clients.retain(|c| c.name != name);
            }
            UiEvent::Observing(_observing) => {
                // Update own observer state if needed
            }
            UiEvent::SetObserving { name, observing } => {
                if let Some(client) = self.clients.iter_mut().find(|c| c.name == name) {
                    client.is_observer = observing;
                }
            }
            UiEvent::SetInControl(name) => {
                // Clear all control flags
                for client in &mut self.clients {
                    client.has_control = false;
                }
                // Set the new controller
                if let Some(client) = self.clients.iter_mut().find(|c| c.name == name) {
                    client.has_control = true;
                }
            }
            UiEvent::AddAircraft(name) => {
                if self.aircraft_list.len() == 1 && self.aircraft_list[0] == "Select an aircraft..."
                {
                    self.aircraft_list.clear();
                }
                self.aircraft_list.push(name);
            }
            UiEvent::Version(version) => {
                self.status_message = format!("Update available: {}", version);
            }
            UiEvent::UpdateFailed => {
                self.status_message = "Update download failed".to_string();
            }
            UiEvent::SendConfig(config_json) => {
                // Parse and load config
                if let Ok(config) = serde_json::from_str::<serde_json::Value>(&config_json) {
                    if let Some(name) = config["name"].as_str() {
                        self.username = name.to_string();
                    }
                    if let Some(port) = config["port"].as_u64() {
                        self.port = port.to_string();
                    }
                    if let Some(timeout) = config["conn_timeout"].as_u64() {
                        self.connection_timeout = timeout.to_string();
                    }
                    if let Some(dark) = config["ui_dark_theme"].as_bool() {
                        self.dark_theme = dark;
                    }
                }
            }
            UiEvent::SendMetrics {
                sent_packets: _,
                received_packets: _,
                sent_kbps,
                receive_kbps,
                packet_loss,
                ping,
            } => {
                self.download_bandwidth = receive_kbps;
                self.upload_bandwidth = sent_kbps;
                self.packet_loss = packet_loss;
                self.ping = ping;
            }
        }
    }
}

impl eframe::App for YourControlsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process events from backend
        self.process_events();

        // Request repaint for real-time updates
        ctx.request_repaint();

        // Apply theme
        if self.dark_theme {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // Status bar
            ui.horizontal(|ui| {
                let (color, icon) = if self.is_connected {
                    (egui::Color32::GREEN, "●")
                } else {
                    (egui::Color32::RED, "●")
                };
                ui.colored_label(color, icon);
                ui.label(&self.status_message);
            });

            ui.separator();

            // Main content - two columns
            ui.columns(2, |columns| {
                // LEFT COLUMN: Server
                columns[0].group(|ui| {
                    ui.heading("🖥 Host");
                    ui.add_space(5.0);

                    ui.horizontal(|ui| {
                        ui.label("Port:");
                        ui.text_edit_singleline(&mut self.port);
                    });

                    ui.horizontal(|ui| {
                        ui.radio_value(
                            &mut self.server_connection_method,
                            ConnectionMethod::CloudServer,
                            "Cloud P2P",
                        );
                        ui.radio_value(
                            &mut self.server_connection_method,
                            ConnectionMethod::Relay,
                            "Cloud Host",
                        );
                        ui.radio_value(
                            &mut self.server_connection_method,
                            ConnectionMethod::Direct,
                            "Direct",
                        );
                    });

                    ui.checkbox(&mut self.is_ipv6, "Use IPv6");

                    if ui
                        .button(if self.is_connected {
                            "Stop Server"
                        } else {
                            "Start Server"
                        })
                        .clicked()
                    {
                        if self.is_connected {
                            self.action_tx.send(AppMessage::Disconnect).ok();
                        } else {
                            self.action_tx
                                .send(AppMessage::StartServer {
                                    username: self.username.clone(),
                                    port: self.port.parse().unwrap_or(7777),
                                    is_ipv6: self.is_ipv6,
                                    use_upnp: true,
                                    method: self.server_connection_method,
                                })
                                .ok();
                        }
                    }
                });

                // RIGHT COLUMN: Client
                columns[1].group(|ui| {
                    ui.heading("🔌 Join");
                    ui.add_space(5.0);

                    // Connection method radio buttons
                    ui.horizontal(|ui| {
                        ui.radio_value(
                            &mut self.client_connection_method,
                            ConnectionMethod::CloudServer,
                            "Cloud Server",
                        );
                        ui.radio_value(
                            &mut self.client_connection_method,
                            ConnectionMethod::Direct,
                            "Direct",
                        );
                    });

                    ui.add_space(5.0);

                    // Show different fields based on connection method
                    if self.client_connection_method == ConnectionMethod::Direct {
                        // Direct connection: IP + Port
                        ui.horizontal(|ui| {
                            ui.label("IP Address:");
                            ui.text_edit_singleline(&mut self.ip_input);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Port:");
                            ui.text_edit_singleline(&mut self.port);
                        });
                    } else {
                        // Cloud connection: Session Code
                        ui.horizontal(|ui| {
                            ui.label("Session Code:");
                            ui.text_edit_singleline(&mut self.session_code);
                        });
                    }

                    ui.checkbox(&mut self.is_ipv6, "Use IPv6");

                    if ui
                        .button(if self.is_connected {
                            "Disconnect"
                        } else {
                            "Connect"
                        })
                        .clicked()
                    {
                        if self.is_connected {
                            self.action_tx.send(AppMessage::Disconnect).ok();
                        } else {
                            let (session_id, ip, port) =
                                if self.client_connection_method == ConnectionMethod::Direct {
                                    // Direct: use IP and port
                                    let parsed_ip = self.ip_input.parse().ok();
                                    let parsed_port = self.port.parse().ok();
                                    (None, parsed_ip, parsed_port)
                                } else {
                                    // Cloud: use session code
                                    (Some(self.session_code.clone()), None, None)
                                };

                            self.action_tx
                                .send(AppMessage::Connect {
                                    username: self.username.clone(),
                                    session_id,
                                    isipv6: self.is_ipv6,
                                    ip,
                                    hostname: None,
                                    port,
                                    method: self.client_connection_method,
                                })
                                .ok();
                        }
                    }
                });
            });

            ui.separator();

            // Bottom section - two columns
            ui.columns(2, |columns| {
                // LEFT: Client list
                columns[0].group(|ui| {
                    ui.heading("👥 Connected Clients");
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            for client in &self.clients {
                                ui.horizontal(|ui| {
                                    let icon = if client.has_control {
                                        "✓"
                                    } else if client.is_observer {
                                        "👁"
                                    } else {
                                        "○"
                                    };
                                    ui.label(format!("{} {}", icon, client.name));

                                    if !client.has_control
                                        && ui.small_button("Give Control").clicked()
                                    {
                                        self.action_tx
                                            .send(AppMessage::TransferControl {
                                                target: client.name.clone(),
                                            })
                                            .ok();
                                    }
                                });
                            }
                        });
                });

                // RIGHT: Settings
                columns[1].group(|ui| {
                    ui.heading("⚙ Settings");

                    ui.horizontal(|ui| {
                        ui.label("Username:");
                        ui.text_edit_singleline(&mut self.username);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Aircraft:");
                        egui::ComboBox::new("aircraft", "")
                            .selected_text(&self.aircraft_list[self.selected_aircraft])
                            .show_ui(ui, |ui| {
                                for (i, aircraft) in self.aircraft_list.iter().enumerate() {
                                    if ui
                                        .selectable_value(&mut self.selected_aircraft, i, aircraft)
                                        .clicked()
                                    {
                                        self.action_tx
                                            .send(AppMessage::LoadAircraft {
                                                config_file_name: aircraft.clone(),
                                            })
                                            .ok();
                                    }
                                }
                            });
                    });

                    ui.checkbox(&mut self.instructor_mode, "Instructor Mode");
                    ui.checkbox(&mut self.streamer_mode, "Streamer Mode");
                    ui.checkbox(&mut self.sound_muted, "Mute Sound");
                    ui.checkbox(&mut self.dark_theme, "Dark Theme");

                    if ui.button("💾 Save Settings").clicked() {
                        // Save settings logic here
                    }
                });
            });

            // Network stats (if connected)
            if self.is_connected {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(format!("↓ {:.2} KB/s", self.download_bandwidth));
                    ui.separator();
                    ui.label(format!("↑ {:.2} KB/s", self.upload_bandwidth));
                    ui.separator();
                    ui.label(format!("Loss: {:.1}%", self.packet_loss * 100.0));
                    ui.separator();
                    ui.label(format!("Ping: {:.0}ms", self.ping));
                });
            }
        });
    }
}
