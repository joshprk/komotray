#![windows_subsystem = "windows"]

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::BufRead;
use std::io::BufReader;
use std::time::Duration;

use komorebi_client::Notification;
use tokio::time;
use tray_icon::menu::Menu;
use tray_icon::menu::MenuEvent;
use tray_icon::menu::MenuItemBuilder;
use tray_icon::Icon;
use tray_icon::TrayIcon;
use tray_icon::TrayIconBuilder;
use uds_windows::UnixListener;
use uds_windows::UnixStream;

struct Tray {
    tray: TrayIcon,
}

impl Tray {
    pub fn new() -> Self {
        let menu = Self::create_menu();
        let tray = TrayIconBuilder::new()
            .with_tooltip("komotray")
            .with_menu(Box::new(menu))
            .build()
            .unwrap();
        
        Self { tray }
    }

    fn create_menu() -> Menu {
        let exit_item = MenuItemBuilder::new()
            .id("exit".into())
            .enabled(true)
            .text("Exit Tray")
            .build();

        let menu = Menu::new();
        let _ = menu.append_items(&[
            &exit_item
        ]);

        menu
    }

    pub fn set_icon(&self, icon: Icon) -> Result<(), tray_icon::Error> {
        self.tray.set_icon(Some(icon))
    }
}

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

struct Connection {
    inner: UnixListener,
    connected: bool,
}

impl Connection {
    pub async fn new() -> Self {
        Self {
            inner: Self::connect().await,
            connected: true,
        }
    }

    async fn connect() -> UnixListener {
        let timeout = Duration::from_secs(1);
        let mut interval = time::interval(timeout);
        loop {
            let Some(socket) = Self::try_connect() else {
                interval.tick().await;
                continue
            };

            break socket
        }
    }

    fn try_connect() -> Option<UnixListener> {
        komorebi_client::subscribe("komotray").ok()
    }
}

impl Iterator for Connection {
    type Item = Option<UnixStream>;

    fn next(&mut self) -> Option<Self::Item> {
        let stream = self.inner
            .incoming()
            .filter_map(|res| res.ok())
            .next();

        if stream.is_none() {
            if let Some(socket) = Self::try_connect() {
                self.inner = socket;
                self.connected = true;
            }

            None
        } else if !self.connected {
            self.connected = true;

            Some(None)
        } else {
            Some(stream)
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let tray = Tray::new();
    let cache = IconCache::new();
    let connection = Connection::new().await;

    let tray_handler = async {
        let event = MenuEvent::receiver();
        let mut interval = time::interval(Duration::from_micros(100));
        loop {
            if let Ok(menu_event) = event.try_recv() {
                match menu_event.id.as_ref() {
                    "exit"  => std::process::exit(0),
                    _       => unreachable!(),
                }
            }

            interval.tick().await;
        }
    };

    let icon_handler = async {
        for data in connection {
            let set_paused = || {
                if let Some(icon) = cache.get("pause") {
                    let _ = tray.set_icon(icon);
                }
            };

            let Some(data) = data else {
                set_paused();
                continue
            };

            let reader = BufReader::new(data);

            for line in reader.lines().flatten() {
                let Ok(json) = serde_json::from_str::<Notification>(&line) else {
                    continue
                };

                let monitor_idx = json.state.monitors.focused_idx();
                let workspace_idx = {
                    let Some(focused_monitor) = json.state.monitors.focused() else {
                        set_paused();
                        continue
                    };

                    focused_monitor.focused_workspace_idx()
                };

                if json.state.is_paused 
                    || !(0..2).contains(&monitor_idx) 
                    || !(0..9).contains(&workspace_idx) 
                {
                    set_paused();
                    continue
                }

                let img_name = format!(
                    "{}-{}",
                    workspace_idx + 1,
                    monitor_idx + 1
                );

                if let Some(icon) = cache.get(img_name.as_str()) {
                    let _ = tray.set_icon(icon);
                }
            }
        }
    };

    tokio::join!(tray_handler, icon_handler);
}