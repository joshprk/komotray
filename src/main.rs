#![windows_subsystem = "windows"]

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::BufReader;
use std::io::Read;
use std::time::Duration;

use komorebi_client::Notification;
use komorebi_client::SocketMessage;
use tokio::time;
use tray_icon::menu::Menu;
use tray_icon::Icon;
use tray_icon::TrayIcon;
use tray_icon::TrayIconBuilder;

struct IconCache {
    inner: HashMap<String, Icon>
}

impl IconCache {
    pub fn new() -> Self {
        let mut inner = HashMap::new();
        let mut asset_path = env::current_exe()
            .unwrap()
            .to_path_buf();

        asset_path.pop();
        asset_path.push("assets/icons/");

        fs::read_dir(asset_path)
            .unwrap()
            .filter_map(|res| res.ok())
            .map(|dir| dir.path())
            .filter_map(|path| {
                let name = path
                    .file_stem()?
                    .to_str()?
                    .to_owned();

                let img = image::open(path.clone())
                    .ok()?
                    .into_rgba8();

                let (w, h) = img.dimensions();
                let rgba = img.into_raw();

                let icon = Icon::from_rgba(rgba, w, h).ok()?;

                Some((name, icon))
            })
            .for_each(|(name, icon): (String, Icon)| {
                inner.insert(name, icon);
            });

        Self { inner }
    }

    pub fn get(&self, name: &str) -> Option<Icon> {
        self.inner.get(name).cloned()
    }
}

struct Tray {
    inner: TrayIcon
}

impl Tray {
    pub fn new(default_icon: Icon) -> Self {
        let inner = TrayIconBuilder::new()
            .with_tooltip(env!("CARGO_PKG_NAME"))
            .with_icon(default_icon)
            .with_menu(Self::create_menu())
            .build()
            .expect("failed to build tray icon");

        Self { inner }
    }

    fn create_menu() -> Box<Menu> {
        Box::new(Menu::new())
    }

    pub fn set_icon(&self, icon: Icon) {
        let _ = self.inner.set_icon(Some(icon));
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let cache = IconCache::new();
    let pause_icon = cache
        .get("pause")
        .expect("pause icon not found");

    let tray = Tray::new(pause_icon.clone());

    let tray_loop = async {
        // loop {}
    };

    let event_loop = async {
        let socket = komorebi_client::subscribe(env!("CARGO_PKG_NAME"))
            .unwrap();

        for data in socket.incoming() {
            let Ok(data) = data else {
                dbg!(data.unwrap_err());
                continue
            };

            let mut buffer = Vec::new();
            let mut reader = BufReader::new(data);

            if matches!(reader.read_to_end(&mut buffer), Ok(0)) {
                let timeout = Duration::from_secs(1);
                let mut interval = time::interval(timeout);
                let msg = SocketMessage::AddSubscriberSocket(env!("CARGO_PKG_NAME").to_string());
                
                tray.set_icon(pause_icon.clone());

                while komorebi_client::send_message(&msg).is_err() {
                    interval.tick().await;
                }
            }

            dbg!(&buffer);

            let Ok(json) = serde_json::from_str::<Notification>(&String::from_utf8(buffer).unwrap()) else {
                continue
            };

            let monitor_idx = json.state.monitors.focused_idx();
            let workspace_idx = {
                let Some(monitor) = json.state.monitors.focused() else {
                    tray.set_icon(pause_icon.clone());
                    continue
                };
                monitor.focused_workspace_idx()
            };

            if json.state.is_paused
                || !(0..2).contains(&monitor_idx) 
                || !(0..9).contains(&workspace_idx)
            {
                tray.set_icon(pause_icon.clone());
                continue
            }

            let icon_name = format!(
                "{}-{}",
                workspace_idx + 1,
                monitor_idx + 1,
            );

            if let Some(icon) = cache.get(&icon_name) {
                tray.set_icon(icon);
            } else {
                tray.set_icon(pause_icon.clone()); 
            }
        }
    };

    tokio::join!(tray_loop, event_loop);
}