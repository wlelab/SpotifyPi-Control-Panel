#![windows_subsystem = "windows"]

pub mod main_window;

use main_window::MainWindow;
use gtk::prelude::*;


#[tokio::main]
async fn main() {
    let app = gtk::Application::builder()
        .application_id("site.riddleling.app.spotifypi-control-panel")
        .build();

    app.connect_activate(move |app| {
        build_ui(&app);
    });

    app.run();
}

fn build_ui(app: &gtk::Application) {
    let win = MainWindow::new(app);
    win.set_title("SpotifyPi Control Panel");
    win.set_border_width(0);
    win.set_window_position(gtk::WindowPosition::Center);
    win.show_all();
}