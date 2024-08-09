#![windows_subsystem = "windows"]

use std::io::BufRead;
use std::io::BufReader;
use std::thread;
use std::time::Duration;

use komorebi_client::Notification;
use komorebi_client::UnixListener;
use tray_icon::menu::Menu;
use tray_icon::TrayIcon;
use tray_icon::TrayIconBuilder;
use uds_windows::UnixStream;

struct Connection {
    inner: UnixListener
}

impl Connection {
    pub fn new() -> Self {
        let inner = Self::connect();

        Self { inner }
    }

    fn connect() -> UnixListener {
        loop {
            if let Ok(socket) =  komorebi_client::subscribe("komotray") {
                break socket
            } else {
                thread::sleep(Duration::from_secs(1));
            }
        }
    }

    pub fn next(&mut self) -> UnixStream {
        // As written for the documentation on UnixListener::incoming(),
        // "the iterator will never return None."
        let tx = self.inner
            .incoming()
            .next()
            .unwrap();

        let Ok(stream) = tx else {
            self.inner = Self::connect();
            return self.next()
        };

        stream
    }
}

struct Tray {
    tray: TrayIcon,
    menu: Menu
}

impl Tray {
    pub fn new() -> Self {
        let menu = Menu::new();
        let tray = TrayIconBuilder::new()
            .with_tooltip("komotray")
            .build()
            .unwrap();

        Self { tray, menu }
    }
}

fn main() {
    let _tray = Tray::new();
    let mut connect = Connection::new();

    // TODO: handle initial state

    loop {
        let data = connect.next();
        let reader = BufReader::new(data);

        for line in reader.lines().flatten() {
            let msg: Notification = match serde_json::from_str(&line) {
                Ok(msg) => msg,
                Err(_)  => continue,
            };

            // TODO: handle events
            /*
            msg.state.monitors
                .focused()
                .unwrap()
                .focused_workspace_idx();
            */
        }
    }
}