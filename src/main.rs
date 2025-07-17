use std::{sync::{mpsc::{self, Receiver, Sender}, Arc, Mutex}, thread::{self, JoinHandle}, time::Duration};
use iced::{application, widget::{pick_list, text, text_input, toggler, Column, Row}, Alignment, Element, Length, Subscription, Theme};
use styles::get_rgb_color;
use vol_limiter::{get_sound_devices, get_system_volume, set_system_volume};

mod styles;

#[derive(Debug, Clone)]
enum Message {
    EnableLimit(u8),
    DisableLimit,
    ChangePercent(String),
    ConfirmPercent,
    ChangeDevice(String),
    UpdateDeviceList,
    AutoCheck(bool),
}

#[derive(Debug, Clone, PartialEq)]
enum Error {
    UpdateError,
    ParseError,

}

#[derive(Debug)]
struct VolControl {
    limiter: bool,
    percent: u8,
    percent_str: String,
    devices: Vec<String>,
    device: Option<String>,
    runner: Option<JoinHandle<()>>,
    scanner: Option<JoinHandle<()>>,
    autocheck: bool,
    tx_limiter: Option<Sender<bool>>,
    tx_scanner: Option<Sender<()>>,
    mutex: Arc<Mutex<Vec<String>>>,
    thread_count: Arc<Mutex<i32>>,
    error: Option<Error>,
}

impl Default for VolControl {
    fn default() -> Self {
        let mut device_list = Vec::from(["".to_owned()]);
        
        device_list.append(&mut get_sound_devices());
        let copy = device_list.clone();
        Self { 
            limiter: false, 
            percent: Default::default(), 
            percent_str: String::from("0"),
            devices: device_list,
            device: Some(String::new()),
            runner: None,
            scanner: None,
            autocheck: false,
            tx_limiter: None,
            tx_scanner: None,
            mutex: Arc::new(Mutex::new(copy)),
            thread_count: Arc::new(Mutex::new(0)),
            error: None,
        }
    }
}

impl VolControl {
    pub fn update(&mut self, message:Message) {
        match message {
            Message::EnableLimit(percent) => {
                println!("limiter {:?} runner{:?}", self.tx_limiter, self.runner);
                if self.tx_limiter.is_none() && self.runner.is_none(){
                    let (tx, rx) = mpsc::channel();
        
                    self.tx_limiter = Some(tx.clone());
        

                    println!("Enabling");
                    self.limiter = true;
                    tx.send(true).unwrap();
                    self.runner.replace(enable_limiter(percent, rx));
                }
        
                // if self.tx.is_none() && self.rx.is_none() {
                //     let (tx, rx) = mpsc::channel();
                //     self.tx = Some(tx.clone());
                //     self.rx.replace(rx);
                // } 

            },
            Message::DisableLimit => {
                println!("Diabling {:?}", self.tx_limiter);
                if let Some(tx) = self.tx_limiter.take() {
                    self.limiter = false;
                    disable_limiter(    tx.clone());
                    let run = self.runner.take();
                    match run {
                        Some(thread) => {
                            let _ = thread.join().map_err(|error| eprintln!("Error: {:?}", error));
                        }
                        None => {}
                    };
                }                
            },
            Message::ChangePercent(input)=> {
                self.percent_str = input;
                if self.percent_str.parse::<u8>().is_err()|| self.percent_str.parse::<u8>().unwrap_or(0) > 100 {
                    self.error = Some(Error::ParseError);
                } else {
                    self.error = None;
                }
            },
            Message::ConfirmPercent => {
                self.percent = if let Ok(new) = self.percent_str.parse::<u8>() {if new <= 100 {new} else {100}} else {self.error = Some(Error::ParseError); 0};
                self.percent_str = self.percent.to_string();
            }

            Message::ChangeDevice(device) => {
                self.device = Some(device);
            },
            Message::UpdateDeviceList => {
                if  Arc::clone(&self.mutex).lock().unwrap().len() != self.devices.len() {
                    println!("Length 1 = {:?} Length2 = {}", Arc::clone(&self.mutex).lock().unwrap(), self.devices.len());
                    println!("DEVICE LIST CHANGED!");
                    self.devices = get_sound_devices()
                }        
            }
            Message::AutoCheck(status) => {
                self.autocheck = status;
                println!("STATUS: {}", status);
                println!("SCANNER: {:?}", self.scanner);
                if status {
                    if self.scanner.is_none() {
                        let clone = Arc::clone(&self.mutex);
                        let count = Arc::clone(&self.thread_count);
                        let (tx, rx) = mpsc::channel();
                        self.tx_scanner.replace(tx);
                        self.scanner.replace( thread::spawn(move || {
                                *count.lock().unwrap() += 1;
                                while !rx.try_recv().is_ok() {
                                    thread::sleep(Duration::from_secs(1));
                                    let mut muter = clone.lock().unwrap();
                                    if get_sound_devices().len() != muter.len() {
                                        *muter = get_sound_devices();
                                    }
                                }
                            })
                        );
                        println!("threadcount {}", self.thread_count.lock().unwrap());
                        println!("{:?}", self.devices);
                    }
                } else {
                    if self.scanner.is_some() {
                        println!("Shutting Thread Down?");
                        let tx = self.tx_scanner.take().unwrap();
                        tx.send(()).unwrap();
                        match self.scanner.take().unwrap().join() {
                            Ok(()) => {println!("Reset Thread?"); self.scanner = None; *self.thread_count.lock().unwrap() -= 1;},
                            Err(_) => {println!("UpdateError"); self.error = Some(Error::UpdateError)}
                        };
                    }
                }

            }
        }
    }

    pub fn view(&self) -> Element<Message>{
        Row::new().push(
            Column::new().push(
                toggler(self.limiter).label("Enable Volume Limiter").on_toggle(|toggle| if toggle {Message::EnableLimit(self.percent)} else {Message::DisableLimit}))
                .push(toggler(self.autocheck).label("Enable Auto Check Device Update").on_toggle(|toggle| Message::AutoCheck(toggle)))
                .push(pick_list(self.devices.clone(), self.device.clone(), Message::ChangeDevice))
                .align_x(Alignment::Center).padding(10).width(Length::FillPortion(1))
                
                
            ).push(
                Column::new()
                .push(text_input(&self.percent.to_string(), &self.percent_str).on_input_maybe(if !self.limiter {Some(|input| Message::ChangePercent(input))} else {None} ).on_submit(Message::ConfirmPercent).style(
                    move |_: &Theme, status| {
                        match status {
                            _ => {
                                if self.error == Some(Error::ParseError) && !self.limiter {
                                    text_input::Style{
                                        border: iced::Border{color: get_rgb_color(255, 0, 0), width: 1.0, ..Default::default()},
                                        value: get_rgb_color(255, 0, 0),
                                        background: iced::Background::Color(get_rgb_color(100, 100, 100)),
                                        icon: iced::Color::default(),
                                        placeholder: get_rgb_color(50, 50, 50),
                                        selection: get_rgb_color(20, 20, 100),
                                    }
                                } else {
                                    text_input::Style{
                                        border: iced::Border{color:get_rgb_color(150, 150, 150), width: 1.0, ..Default::default()},
                                        value: get_rgb_color(255, 255, 255),
                                        background: iced::Background::Color(get_rgb_color(100, 100, 100)),
                                        icon: iced::Color::default(),
                                        placeholder: get_rgb_color(150, 150, 150),
                                        selection: get_rgb_color(20, 20, 100),
                                    }
                                }
                            },
                        }
                    }
                )).push_maybe(if self.error == Some(Error::ParseError) {Some(text("Please enter a number between 0 and 100!").color(get_rgb_color(255, 0, 0)))} else {None})
                .push(text(format!("Current Volume Limit: {}", self.percent)))
                .push(text("Hello World")).align_x(Alignment::Center).padding(20).width(Length::FillPortion(1))
            ).padding(20).align_y(Alignment::Center).spacing(10).into()
        
    }

    pub fn subscription(&self) -> Subscription<Message>{
        
        iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::UpdateDeviceList)
    }
}

fn enable_limiter(percent: u8, rx: Receiver<bool>) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut limit;
        if let Ok(status) = rx.recv() {
            println!("limit is :{}", status);
            limit = status;
        } else {
            limit = true;
        }
        while limit {
            // println!("Blocking!");
            // println!("Current System volume is: {}", get_system_volume());
            if get_system_volume() > percent {
                set_system_volume(percent);
            }
            thread::sleep(Duration::from_millis(100));

            if let Ok(status) = rx.recv_timeout(Duration::from_millis(10)) {
                println!("limit is :{}", status);
                limit = status;
            }
        }
        println!("thread ended");
    })
}

fn disable_limiter(tx:Sender<bool>) {
    tx.send(false).unwrap();
}

fn main() -> iced::Result{
    get_sound_devices();
    application("Volume Limiter", VolControl::update, VolControl::view).subscription(VolControl::subscription).run()
}
