use glib::clone;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use std::cell::{Cell, RefCell};
use once_cell::unsync::OnceCell;

use futures_util::{future, pin_mut, StreamExt};
use futures::channel::mpsc::{unbounded, UnboundedSender, UnboundedReceiver};
use tokio::task;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use regex::Regex;


#[derive(Debug, Default)]
pub struct MainWindow {
    // connect
    ws_addr_entry: OnceCell<gtk::Entry>,
    connect_button: OnceCell<gtk::Button>,

    // prev track
    prev_track_button: OnceCell<gtk::Button>,
    prev_track_handler_id: RefCell<Option<glib::SignalHandlerId>>,

    // toggle play / pause
    play_pause_button: OnceCell<gtk::Button>,
    play_pause_handler_id: RefCell<Option<glib::SignalHandlerId>>,

    // next track
    next_track_button: OnceCell<gtk::Button>,
    next_track_handler_id: RefCell<Option<glib::SignalHandlerId>>,

    // toggle shuffle
    toggle_shuffle_button: OnceCell<gtk::Button>,
    toggle_shuffle_handler_id: RefCell<Option<glib::SignalHandlerId>>,

    // toggle repeat off / single song / whole playlist
    toggle_repeat_state_button: OnceCell<gtk::Button>,
    toggle_repeat_state_handler_id: RefCell<Option<glib::SignalHandlerId>>,

    // power
    power_button: OnceCell<gtk::MenuButton>,
    power_popover: OnceCell<gtk::Popover>,
    shutdown_button: OnceCell<gtk::Button>,
    shutdown_handler_id: RefCell<Option<glib::SignalHandlerId>>,
    reboot_button: OnceCell<gtk::Button>,
    reboot_handler_id: RefCell<Option<glib::SignalHandlerId>>,

    // volume
    volume_label: OnceCell<gtk::Label>,
    volume_button: OnceCell<gtk::SpinButton>,
    volume_handler_id: RefCell<Option<glib::SignalHandlerId>>,
    lock_volume_button_signal: Cell<bool>,
}

#[glib::object_subclass]
impl ObjectSubclass for MainWindow {
    const NAME: &'static str = "MainWindow";
    type Type = super::MainWindow;
    type ParentType = gtk::ApplicationWindow;
}

impl ObjectImpl for MainWindow {
    fn constructed(&self, obj: &Self::Type) {
        self.parent_constructed(obj);

        // main_box
        let main_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .homogeneous(false)
            .spacing(0)
            .build();

        // box1
        let box1 = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .homogeneous(false)
            .margin_start(15)
            .margin_end(15)
            .margin_top(10)
            .margin_bottom(10)
            .spacing(5)
            .build();
        let ws_label = gtk::Label::builder()
            .label("ws://")
            .margin_start(5)
            .margin_end(0)
            .build();

        let ws_addr_entry = gtk::Entry::builder()
            .text("spotifypi.local:9487")
            .build();

        let connect_button = gtk::Button::builder()
            .label("Connect")
            .build();

        box1.pack_start(&ws_label, false, false, 0);
        box1.pack_start(&ws_addr_entry, true, true, 0);
        box1.pack_start(&connect_button, false, false, 0);

        connect_button.connect_clicked(clone!(@weak obj => move |_| {
            let priv_ = MainWindow::from_instance(&obj);
            priv_.on_connect_button_clicked();
        }));


        // box2    
        let box2 = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .homogeneous(false)
            .margin_start(15)
            .margin_end(15)
            .margin_top(10)
            .margin_bottom(10)
            .spacing(5)
            .build();
        let prev_track_button = gtk::Button::builder()
            .label("Prev track")
            .build();
        let play_pause_button = gtk::Button::builder()
            .label("Play / Pause")
            .build();
        let next_track_button = gtk::Button::builder()
            .label("Next track")
            .build();

        box2.pack_start(&prev_track_button, false, false, 0);
        box2.pack_start(&play_pause_button, true, true, 0);
        box2.pack_start(&next_track_button, false, false, 0);


        // box3
        let box3 = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .homogeneous(false)
            .margin_start(15)
            .margin_end(15)
            .margin_top(10)
            .margin_bottom(10)
            .spacing(5)
            .build();
        let toggle_shuffle_button = gtk::Button::builder()
            .label("Toggle Shuffle")
            .build();
        let toggle_repeat_state_button = gtk::Button::builder()
            .label("Toggle Repeat off / Single song / Whole playlist")
            .build();

        box3.pack_start(&toggle_shuffle_button, true, true, 0);
        box3.pack_start(&toggle_repeat_state_button, true, true, 0);


        // blank_box
        let blank_box = gtk::Box::builder()
            .margin_start(15)
            .margin_end(15)
            .margin_top(5)
            .margin_bottom(5)
            .build();

        // action_bor
        let action_bor = gtk::ActionBar::new();

        // power
        let power_popover = gtk::Popover::builder()
            .build();
        let popover_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin(10)
            .spacing(10)
            .build();

        let shutdown_button = gtk::Button::builder()
            .label("Shutdown")
            .build();
        let reboot_button = gtk::Button::builder()
            .label("Reboot")
            .build();
        let power_button = gtk::MenuButton::builder()
            .label("Power")
            .margin_start(0)
            .margin_end(0)
            .popover(&power_popover)
            .build();

        // volume
        let volume_label = gtk::Label::builder()
            .label("Volume:")
            .build();
        let volume_button = gtk::SpinButton::builder()
            .margin_start(0)
            .margin_end(0)
            .build();

        popover_box.pack_start(&shutdown_button, false, false, 0);
        popover_box.pack_start(&reboot_button, false, false, 0);
        popover_box.show_all();
        power_popover.add(&popover_box);
        power_popover.set_position(gtk::PositionType::Top);

        volume_button.set_range(0., 100.);
        volume_button.set_increments(1., 0.);
        volume_button.set_value(0.);

        action_bor.pack_start(&power_button);
        action_bor.pack_end(&volume_button);
        action_bor.pack_end(&volume_label);
        

        // add components to main_box
        main_box.pack_start(&box1, false, false, 0);
        main_box.pack_start(&box2, false, false, 0);
        main_box.pack_start(&box3, false, false, 0);
        main_box.pack_start(&blank_box, true, true, 0);
        main_box.pack_start(&action_bor, false, false, 0);

        // set window
        obj.add(&main_box);
        obj.set_default_size(600, 0);

        // disable buttons 
        prev_track_button.set_sensitive(false);
        play_pause_button.set_sensitive(false);
        next_track_button.set_sensitive(false);
        toggle_shuffle_button.set_sensitive(false);
        toggle_repeat_state_button.set_sensitive(false);
        power_button.set_sensitive(false);
        volume_button.set_sensitive(false);
        volume_label.set_sensitive(false);
 
        self.ws_addr_entry.set(ws_addr_entry).expect("Failed to initialize window state: ws_addr_entry");
        self.connect_button.set(connect_button).expect("Failed to initialize window state: connect_button");
        
        self.prev_track_button.set(prev_track_button).expect("Failed to initialize window state: prev_track_button");
        self.play_pause_button.set(play_pause_button).expect("Failed to initialize window state: play_pause_button");
        self.next_track_button.set(next_track_button).expect("Failed to initialize window state: next_track_button");
        self.toggle_shuffle_button.set(toggle_shuffle_button).expect("Failed to initialize window state: toggle_shuffle_button");
        self.toggle_repeat_state_button.set(toggle_repeat_state_button).expect("Failed to initialize window state: toggle_repeat_state_button");

        self.power_button.set(power_button).expect("Failed to initialize window state: power_button");
        self.power_popover.set(power_popover).expect("Failed to initialize window state: power_popover");
        self.shutdown_button.set(shutdown_button).expect("Failed to initialize window state: shutdown_button");
        self.reboot_button.set(reboot_button).expect("Failed to initialize window state: reboot_button");

        self.volume_label.set(volume_label).expect("Failed to initialize window state: volume_label");
        self.volume_button.set(volume_button).expect("Failed to initialize window state: volume_button");

        self.lock_volume_button_signal.set(false);
    }
}

impl MainWindow {
    fn on_connect_button_clicked(&self) {
        let connect_button = self.connect_button.get().unwrap();
        connect_button.set_sensitive(false);

        let ws_addr_entry = self.ws_addr_entry.get().unwrap();
        let connect_addr = format!("ws://{}", &ws_addr_entry.text());

        let url = match url::Url::parse(connect_addr.as_str()) {
            Ok(url) => url,
            Err(e) => {
                eprintln!("Url::parse failed: {}", e);
                // display a dialog
                let obj = MainWindow::instance(self);
                glib::MainContext::default().spawn_local(show_dialog(obj, format!("{}", e)));
                return;
            }
        };
        eprintln!("ws_url: {}", url);
        ws_addr_entry.select_region(0,0);

        let (output_tx, output_rx) : (glib::Sender<String>, glib::Receiver<String>) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let (input_tx, input_rx) : (UnboundedSender<Message>, UnboundedReceiver<Message>) = unbounded();

        let prev_track_button = self.prev_track_button.get().unwrap();
        let play_pause_button = self.play_pause_button.get().unwrap();
        let next_track_button = self.next_track_button.get().unwrap();
        let toggle_shuffle_button = self.toggle_shuffle_button.get().unwrap();
        let toggle_repeat_state_button = self.toggle_repeat_state_button.get().unwrap();
        let power_popover = self.power_popover.get().unwrap();
        let shutdown_button = self.shutdown_button.get().unwrap();
        let reboot_button = self.reboot_button.get().unwrap();
        let volume_button = self.volume_button.get().unwrap();

        // prev track
        let prev_track_handler_id = prev_track_button.connect_clicked(clone!(@strong input_tx => move |_| {
            input_tx.unbounded_send(Message::text("prev_track")).expect("Could not send through channel");
        }));
        self.prev_track_handler_id.replace(Some(prev_track_handler_id));

        // toggle play / pause
        let play_pause_handler_id = play_pause_button.connect_clicked(clone!(@strong input_tx => move |_| {
            input_tx.unbounded_send(Message::text("toggle_play_pause")).expect("Could not send through channel");
        }));
        self.play_pause_handler_id.replace(Some(play_pause_handler_id));

        // next track
        let next_track_handler_id = next_track_button.connect_clicked(clone!(@strong input_tx => move |_| {
            input_tx.unbounded_send(Message::text("next_track")).expect("Could not send through channel");
        }));
        self.next_track_handler_id.replace(Some(next_track_handler_id));

        // toggle shuffle
        let toggle_shuffle_handler_id = toggle_shuffle_button.connect_clicked(clone!(@strong input_tx => move |_| {
            input_tx.unbounded_send(Message::text("toggle_shuffle")).expect("Could not send through channel");
        }));
        self.toggle_shuffle_handler_id.replace(Some(toggle_shuffle_handler_id));

        // toggle repeat off / single song / whole playlist
        let toggle_repeat_state_handler_id = toggle_repeat_state_button.connect_clicked(clone!(@strong input_tx => move |_| {
            input_tx.unbounded_send(Message::text("toggle_repeat_state")).expect("Could not send through channel");
        }));
        self.toggle_repeat_state_handler_id.replace(Some(toggle_repeat_state_handler_id));

        // shutdown
        let shutdown_handler_id = shutdown_button.connect_clicked(
            clone!(@strong input_tx, @weak power_popover => move |_| {
                power_popover.hide();
                input_tx.unbounded_send(Message::text("shutdown")).expect("Could not send through channel");
            })
        );
        self.shutdown_handler_id.replace(Some(shutdown_handler_id));

        // reboot
        let reboot_handler_id = reboot_button.connect_clicked(
            clone!(@strong input_tx, @weak power_popover => move |_| {
                power_popover.hide();
                input_tx.unbounded_send(Message::text("reboot")).expect("Could not send through channel");
            })
        );
        self.reboot_handler_id.replace(Some(reboot_handler_id));

        // get window instance
        let obj = MainWindow::instance(self);

        // volume
        let volume_handler_id = volume_button.connect_value_changed(clone!(@weak obj, @strong input_tx => move |_| {
            let priv_ = MainWindow::from_instance(&obj);
            priv_.send_volume_value(&input_tx);
        }));
        self.volume_handler_id.replace(Some(volume_handler_id));

        // receive message from ws
        output_rx.attach(
            None,
            clone!(@weak obj, @strong input_tx => @default-return Continue(false),
                move |msg| {
                    eprintln!(">> msg: {}", msg);
                    let priv_ = MainWindow::from_instance(&obj);
                    let (event, value) = get_event_and_value(msg);
                    if event == "connect" && value == "ok" {
                        priv_.control_widgets_enable(true);
                        input_tx.unbounded_send(Message::text("get_volume")).expect("Could not send through channel");
                    } else if event == "connect" && value == "failed" {
                        priv_.handle_disconnect("Connect failed.".to_string());
                    } else if event == "disconnect" {
                        priv_.handle_disconnect("WebSocket connection closed.".to_string());
                    } else if event == "volume" {
                        if let Ok(volume) = value.parse::<i32>() {
                            priv_.set_volume_value(volume);
                        }
                    }
                    glib::Continue(true)
                }
            )
        );

        // connect to ws
        connect_button.set_label("Connecting...");
        task::spawn(async move {
            connect_to_ws(url, input_rx, output_tx).await;
        });
    }

    fn handle_disconnect(&self, dialog_text: String) {
        if let Some(id) = self.prev_track_handler_id.borrow_mut().take() {
            self.prev_track_button.get().unwrap().disconnect(id)
        }
        if let Some(id) = self.play_pause_handler_id.borrow_mut().take() {
            self.play_pause_button.get().unwrap().disconnect(id)
        }
        if let Some(id) = self.next_track_handler_id.borrow_mut().take() {
            self.next_track_button.get().unwrap().disconnect(id)
        }
        if let Some(id) = self.toggle_shuffle_handler_id.borrow_mut().take() {
            self.toggle_shuffle_button.get().unwrap().disconnect(id)
        }
        if let Some(id) = self.toggle_repeat_state_handler_id.borrow_mut().take() {
            self.toggle_repeat_state_button.get().unwrap().disconnect(id)
        }
        if let Some(id) = self.shutdown_handler_id.borrow_mut().take() {
            self.shutdown_button.get().unwrap().disconnect(id)
        }
        if let Some(id) = self.reboot_handler_id.borrow_mut().take() {
            self.reboot_button.get().unwrap().disconnect(id)
        }
        if let Some(id) = self.volume_handler_id.borrow_mut().take() {
            self.volume_button.get().unwrap().disconnect(id)
        }

        self.control_widgets_enable(false);

        // display a dialog
        let obj = MainWindow::instance(self);
        glib::MainContext::default().spawn_local(show_dialog(obj, dialog_text));
    }

    fn control_widgets_enable(&self, enable: bool) {
        self.ws_addr_entry.get().unwrap().set_editable(!enable);

        let connect_button = self.connect_button.get().unwrap();
        connect_button.set_sensitive(!enable);
        if enable {
            connect_button.set_label("Connected");
        } else {
            connect_button.set_label("Connect");
        }
        
        self.prev_track_button.get().unwrap().set_sensitive(enable);
        self.play_pause_button.get().unwrap().set_sensitive(enable);
        self.next_track_button.get().unwrap().set_sensitive(enable);
        self.toggle_shuffle_button.get().unwrap().set_sensitive(enable);
        self.toggle_repeat_state_button.get().unwrap().set_sensitive(enable);
        self.power_button.get().unwrap().set_sensitive(enable);
        self.volume_button.get().unwrap().set_sensitive(enable);
        self.volume_label.get().unwrap().set_sensitive(enable);
    }

    fn send_volume_value(&self, input_tx: &UnboundedSender<Message>) {
        let volume_button = self.volume_button.get().unwrap();
        if self.lock_volume_button_signal.get() == true {
            eprintln!("!! lock volume button signal");
            self.lock_volume_button_signal.set(false);
            return;
        }
        volume_button.set_sensitive(false);
        let value = volume_button.value_as_int();
        eprintln!("< volume: {}", value);
        let cmd = format!("set_volume {}", value);
        input_tx.unbounded_send(Message::text(cmd)).expect("Could not send through channel");
    }

    fn set_volume_value(&self, volume: i32) {
        let volume_button = self.volume_button.get().unwrap();
        let current_volume = volume_button.value_as_int();
        if volume != current_volume {
            self.lock_volume_button_signal.set(true);
            let new_volume = volume as f64;
            volume_button.set_value(new_volume);
        }
        volume_button.set_sensitive(true);
    }
}

impl WidgetImpl for MainWindow {}
impl ContainerImpl for MainWindow {}
impl BinImpl for MainWindow {}
impl WindowImpl for MainWindow {}
impl ApplicationWindowImpl for MainWindow {}


async fn connect_to_ws(url: url::Url, input_rx: UnboundedReceiver<Message>, output_tx: glib::Sender<String>) {
    let (ws_stream, _) = match connect_async(url).await {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("Failed to connect: {}", e);
            output_tx.send("[connect](failed)".to_string()).expect("Could not send through channel");
            return
        }
    };

    eprintln!("WebSocket handshake has been successfully completed");
    output_tx.send("[connect](ok)".to_string()).expect("Could not send through channel");

    let (write, read) = ws_stream.split();

    let input_to_ws = input_rx.map(Ok).forward(write);
    let ws_to_output = {
        read.for_each(|message| async {
            match message {
                Ok(msg) => {
                    let data = msg.into_data();
                    if let Ok(text) = String::from_utf8(data) {
                        output_tx.send(text).expect("Could not send through channel");
                    }
                }
                Err(err) => eprintln!("Message unwrap failed: {}", err)
            }
        })
    };

    pin_mut!(input_to_ws, ws_to_output);
    future::select(input_to_ws, ws_to_output).await;

    eprintln!("WebSocket disconnected !!!");
    output_tx.send("[disconnect]()".to_string()).expect("Could not send through channel");
}

fn get_event_and_value(msg: String) -> (String, String) {
    let re = Regex::new(r"\[(?P<event>.+?)\]\((?P<value>.*?)\)").unwrap();
    match re.captures(&msg) {
        Some(caps) => {
            let event = &caps["event"];
            let value = &caps["value"];
            (event.to_string(), value.to_string())
        }
        None => ("".to_string(), "".to_string())
    }
}

async fn show_dialog<W: IsA<gtk::Window>>(window: W, message: String) {
    let dialog = gtk::MessageDialog::builder()
        .transient_for(&window)
        .modal(true)
        .buttons(gtk::ButtonsType::Ok)
        .title("Alert")
        .text("Message")
        .secondary_text(&message)
        .window_position(gtk::WindowPosition::Center)
        .build();
    dialog.run_future().await;
    dialog.close();
}

