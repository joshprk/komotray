# komotray

Simple and lightweight tray icon which shows the current workspace for the Komorebi window manager.

![](assets/demo.png)

This is a Rust rewrite of the [original AutoHotKey komotray](https://github.com/urob/komotray) with improvements in automatically recovering from error states and using a single-threaded asynchronous runtime to optimize resource usage. Icons are courtesy of the original, provided by urob.

A major limitation is that only two monitors are supported due to the lack of icons. If you use more than two monitors, try using a status bar like yasb or Zebar.

Licensed under MIT.