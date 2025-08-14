use std::{sync::{mpsc::{self, Receiver, Sender}, Arc, Mutex}, thread::{self, JoinHandle}, time::Duration};
use iced::{application, widget::{button, pick_list, radio, slider, text, text_input, toggler, vertical_space, Column, Row}, Alignment, Element, Length, Size, Subscription, Task, Theme};
use vol_limiter::styles::{get_rgb_color};
use cpvc::command::{get_sound_devices_command, get_system_volume_command, set_system_volume_command};
use cpvc::{get_sound_devices, get_system_volume, set_system_volume};
use vol_limiter::{components::hov_container_row::{self, HovContainer}};


#[derive(Debug, Clone)]
enum Message {
    EnableLimit,
    DisableLimit,
    ChangePercent(String, bool),
    ConfirmPercent(bool),
    ChangeDevice(String),
    UpdateDeviceList,
    AutoLimiter,
    AutoCheck(bool),
    ChangeByOne(bool, bool),
    SystemVolChange,
    SliderVolChange(u8, bool),
    None,
    ChangeVolInput(InputType),
    ChangeLimitSel(BuiltIn),
    ClearError,
    ChangeAutoLimiter(bool),
    ChangeAutoAutoLimiter(bool),
    OnToggle(bool),
    OnPick(String),
}

#[derive(Debug, Clone, PartialEq)]
enum Error {
    UpdateError,
    ParseError,
    ParseVolError,
    AdjustWhileOn,


}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputType {
    Slider,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuiltIn {
    Twenty,
    Fifty,
    Eighty,
    Custom
}

#[derive(Debug)]
struct VolControl {
    limiter: bool,
    percent: u8,
    percent_str: String,
    all_devices: Vec<String>,
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
    error_length: u8,
    autolimiter: bool,
    auto_autolimiter: bool,
    input_vol: Option<InputType>,
    sel_lim: Option<BuiltIn>,
    volume: u8, 
    vol_str: String,
}

impl Default for VolControl {
    fn default() -> Self {
        let mut device_list = Vec::from(["".to_owned()]);
        
        device_list.append(&mut get_sound_devices());
        let copy = device_list.clone();
        Self { 
            limiter: false, 
            percent: 20, 
            percent_str: 20.to_string(),
            all_devices: device_list.clone(),
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
            error_length: 0,
            autolimiter: true,
            auto_autolimiter: true,
            input_vol: Some(InputType::Slider),
            sel_lim: Some(BuiltIn::Twenty),
            volume: get_system_volume(),
            vol_str: get_system_volume().to_string(),
        }
    }
}

impl VolControl {
    pub fn update(&mut self, message:Message) -> Task<Message> {
        match message {
            Message::EnableLimit => {
                    println!("limiter {:?} runner{:?}", self.tx_limiter, self.runner);
                    if self.tx_limiter.is_none() && self.runner.is_none(){
                
                        // if self.tx.is_none() && self.rx.is_none() {
                        //     let (tx, rx) = mpsc::channel();
                        //     self.tx = Some(tx.clone());
                        //     self.rx.replace(rx);
                        // } 
                        if self.percent.to_string() == self.percent_str {
                            let percent = self.percent;
                            let (tx, rx) = mpsc::channel();
                            self.tx_limiter = Some(tx.clone());
                            println!("Enabling");
                            self.limiter = true;
                            tx.send(true).unwrap();
                            self.runner.replace(enable_limiter(percent, rx));
                            self.volume = if get_system_volume() < self.percent {
                                    get_system_volume()
                                } else {
                                    self.percent
                                };
                            self.vol_str = self.volume.to_string();
                            Task::none()
                        } else {
                            self.volume = cpvc::get_system_volume();
                            self.vol_str = self.volume.to_string();
                            Task::batch(vec![
                                Task::perform(async {}, |_| Message::ConfirmPercent(true)),
                                Task::perform(async {}, |_| Message::EnableLimit),
                            ])
                        }
                        
                    }
                    else {
                        Task::none()
                    }
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
                Task::none()             
            },
            Message::ChangePercent(input, limit)=> {
                if limit {
                    self.percent_str = input;
                    if self.percent_str.parse::<u8>().is_err()|| self.percent_str.parse::<u8>().unwrap_or(0) > 100 {
                        self.error = Some(Error::ParseError);
                        self.error_length = 0;
                    } else {
                        self.error = None;
                    }
                } else {
                    self.vol_str = input;
                    if self.vol_str.parse::<u8>().is_err()|| self.vol_str.parse::<u8>().unwrap_or(0) > 100 {
                        self.error = Some(Error::ParseVolError);
                        self.error_length = 0;
                    } else {
                        self.error = None;
                    }
                }
                Task::none()
            },
            Message::ConfirmPercent(limit) => {
                if limit && !self.limiter{
                    self.percent = if let Ok(new) = self.percent_str.parse::<u8>() {if new <= 100 {new} else {100}} else {self.error = Some(Error::ParseError); self.error_length = 0; self.percent};
                    self.percent_str = self.percent.to_string();
                    match self.percent {
                        20 => {
                            self.sel_lim.replace(BuiltIn::Twenty);
                        },
                        50 => {
                            self.sel_lim.replace(BuiltIn::Fifty);
                        },
                        80 => {
                            self.sel_lim.replace(BuiltIn::Eighty);
                        },
                        _ => {
                            self.sel_lim.replace(BuiltIn::Custom);
                        },
                    }
                    Task::none()
                } else if limit && self.limiter {
                    Task::batch(vec![
                        Task::perform(async {}, move |_| Message::DisableLimit),
                        Task::perform(async {}, move |_| Message::ConfirmPercent(limit)),
                        Task::perform(async {}, move |_| Message::EnableLimit),
                    ])
                } else {
                    if !self.limiter {
                        self.volume = if let Ok(new) = self.vol_str.parse::<u8>() {if new <= 100 {new} else {100}} else {self.error = Some(Error::ParseError); self.error_length = 0; self.volume};
                        self.vol_str = self.volume.to_string();
                    } else {
                        self.volume = if let Ok(new) = self.vol_str.parse::<u8>() {if new <= self.percent {new} else {self.percent}} else {self.error = Some(Error::ParseError); self.error_length = 0; self.volume};
                        self.vol_str = self.volume.to_string();
                    }
                    set_system_volume(self.volume);
                    Task::none()
                }
            }
            Message::ChangeDevice(device) => {
                if device == String::from("") {
                    self.device = None;
                } else {
                    self.device = Some(device);
                }
                Task::none()
            },
            Message::UpdateDeviceList => {
                if Arc::clone(&self.mutex).lock().unwrap().len() != self.devices.len() {
                    println!("Length 1 = {:?} Length2 = {}", Arc::clone(&self.mutex).lock().unwrap(), self.devices.len());
                    println!("DEVICE LIsT cHANGED!");
                    for device in Arc::clone(&self.mutex).lock().unwrap().iter() {
                        if !self.all_devices.contains(device) {
                            self.all_devices.push(String::from(device));
                        }
                    } 
                    self.devices = get_sound_devices();
                }     
                Task::none()   
            }
            Message::AutoLimiter => {
                if self.autolimiter && self.auto_autolimiter {
                    if self.devices.contains(self.device.as_ref().unwrap_or(&String::from(""))) && self.device != Some(String::from("")) && !self.limiter {
                        let device = self.device.clone().unwrap_or(String::from(""));
                        println!("{}", device);
                        println!("Turning on!");
                        Task::perform(async {}, |_| Message::EnableLimit)
                    } else if !self.devices.contains(self.device.as_ref().unwrap_or(&String::from(""))) || self.device == None && self.limiter {
                        self.device = Some(String::from(""));
                        let device = self.device.clone().unwrap_or(String::from(""));
                        println!("{}", device);
                        println!("Turning OFF!");
                        Task::perform(async {}, |_| Message::DisableLimit)
                    } else {
                        Task::none()
                    }
                } else {
                    Task::none()
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
                            Err(_) => {println!("UpdateError"); self.error = Some(Error::UpdateError); self.error_length = 0;}
                        };
                    }
                }
                Task::none()
            }
            Message::ChangeByOne(increase, limit) => {
                if limit == true {
                    if self.percent.to_string() == self.percent_str && !self.limiter{
                        if increase && self.percent < 100{
                            self.percent += 1;
                        } else if self.percent > 0 {
                            self.percent -= 1;
                        }
                        match self.percent {
                            20 => {
                                self.sel_lim.replace(BuiltIn::Twenty);
                            },
                            50 => {
                                self.sel_lim.replace(BuiltIn::Fifty);
                            },
                            80 => {
                                self.sel_lim.replace(BuiltIn::Eighty);
                            },
                            _ => {
                                self.sel_lim.replace(BuiltIn::Custom);
                            },
                        }
                        self.percent_str = self.percent.to_string();
                        Task::none()
                    } else if self.limiter {
                         Task::batch(vec![
                            Task::perform(async {}, move |_| Message::DisableLimit),
                            Task::perform(async {}, move |_| Message::ConfirmPercent(limit)),
                            Task::perform(async {}, move |_| Message::ChangeByOne(increase, limit)),
                            Task::perform(async {}, move |_| Message::EnableLimit),
                        ])
                    } else {
                        Task::batch(vec![
                            Task::perform(async {}, move |_| Message::ConfirmPercent(limit)),
                            Task::perform(async {}, move |_| Message::ChangeByOne(increase, limit)),
                        ])
                    }
                } else {
                    if self.volume.to_string() == self.vol_str {
                        if increase && self.volume < 100{
                            if (self.volume < self.percent && self.limiter) || !self.limiter {
                                self.volume += 1;
                            }
                        } else if self.volume > 0 {
                            self.volume -= 1;
                        }
                        cpvc::set_system_volume(self.volume);
                        self.vol_str = self.volume.to_string();
                        Task::none()
                    } else {
                        Task::batch(vec![
                            Task::perform(async {}, move |_| Message::ConfirmPercent(limit)),
                            Task::perform(async {}, move |_| Message::ChangeByOne(increase, limit)),
                        ])
                    }
                }
            },
            Message::SystemVolChange => {
                if cpvc::get_system_volume() != self.volume {
                    self.volume = cpvc::get_system_volume();
                    self.vol_str = self.volume.to_string();
                }
                Task::none()
            },
            Message::SliderVolChange(volume, limit) => {
                if limit {
                    if volume != self.percent {
                        self.percent = volume;
                    }
                } else {
                    if volume != self.volume {
                        set_system_volume(volume);
                        self.volume = volume;
                        self.vol_str = self.volume.to_string();
                    }
                }
                Task::none()
            },
            Message::None => {
                Task::none()
            },
            Message::ChangeLimitSel(new) => {
                let mut task = Task::none();
                if !self.limiter {   
                    let mut change_str = true;
                    match new {
                        BuiltIn::Twenty => {
                            self.percent = 20;
                        },
                        BuiltIn::Fifty => {
                            self.percent = 50;
                        },
                        BuiltIn::Eighty => {
                            self.percent = 80;
                        },
                        BuiltIn::Custom => {
                            task = Task::perform(async {}, |_| Message::ConfirmPercent(true));
                            change_str = false
                        },
                    }
                    if change_str {
                        self.percent_str = self.percent.to_string();
                    }
                    self.sel_lim.replace(new);
                    task
                } else {
                    Task::batch(vec![
                        Task::perform(async {}, move |_| Message::DisableLimit),
                        Task::perform(async {}, move |_| Message::ChangeLimitSel(new)),
                        Task::perform(async {}, move |_| Message::EnableLimit),
                    ])
                }
                
            },
            Message::ChangeVolInput(new) => {
                self.input_vol.replace(new);
                Task::none()
            }
            Message::ClearError => {
                self.error_length += 1;
                if self.error_length == 3 {
                    self.error = None;
                }
                if self.error_length > 200 {
                    self.error_length = 4;
                }
                Task::none()
            },
            Message::ChangeAutoAutoLimiter(status) => {
                self.auto_autolimiter = status;
                Task::none()
            },
            Message::ChangeAutoLimiter(status) => {
                self.autolimiter = status;
                Task::none()
            },
            Message::OnToggle(toggle) => {
                self.auto_autolimiter = false;
                if toggle {
                    Task::perform(async {}, |_| Message::EnableLimit)
                } else {
                    Task::perform(async {}, |_| Message::DisableLimit)
                }
            },
            Message::OnPick(device) => {
                Task::batch(vec! [
                    Task::perform(async {}, |_| Message::ChangeAutoAutoLimiter(true)),
                    Task::perform(async {}, move |_| Message::ChangeDevice(device.clone())),
                ]
                )
            },
        }
    }
    // NextUI
    pub fn view(&self) -> Element<Message> {
        Column::new().push(text("Volume Limiter").center().size(20).width(Length::Fill)).push(
            HovContainer::new()
            .push(Column::new()
                .push(text("Volume Controls").width(Length::Fill).size(18).height(30).center())
                .push(
                    Row::new()
                    .push(
                        radio("Slider", InputType::Slider, self.input_vol, |selection| Message::ChangeVolInput(selection)))    
                    .push(
                        radio("Text", InputType::Text, self.input_vol, |selection| Message::ChangeVolInput(selection)))
                    .spacing(40)
                ).width(Length::Fill).align_x(Alignment::Center)
            )
                .on_hover(Message::None).on_exit(Message::None)
                .style(hov_container_row::auto_style(get_rgb_color(150, 150, 150), get_rgb_color(100, 100, 255), 3, 15))
                .padding(20).width(Length::Fill))

            .push(HovContainer::new()
                .push(Column::new()
                    .push(
                        if self.input_vol == Some(InputType::Slider) {
                            Column::new()
                                .push(Row::new()
                                    .push(slider(if self.limiter {0..=self.percent} else {0..=100}, self.volume,|vol| Message::SliderVolChange(vol, false)))
                                    .push(text(self.volume)).padding(20).spacing(20).height(70).align_y(Alignment::Center)
                                )
                        } else {
                            Column::new()
                                .push(Row::new().push(button(" + ").on_press(Message::ChangeByOne(true, false)))
                                .push(text_input(&self.vol_str, &self.vol_str).on_input(|input| Message::ChangePercent(input, false)).on_submit(Message::ConfirmPercent(false)).style(
                                    move |_: &Theme, status| {
                                        match status {
                                            _ => {
                                                if self.error == Some(Error::ParseVolError) {
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
                                                            placeholder: get_rgb_color(200, 200, 200),
                                                            selection: get_rgb_color(20, 20, 100),
                                                        }
                                                    }
                                                },
                                            }
                                        }
                                    ).align_x(Alignment::Center).width(Length::Fixed(100.0)))
                                .push(
                                    button(" - ").on_press(Message::ChangeByOne(false, false))
                                ).align_y(Alignment::Center).spacing(10).padding(20).height(70))
                                .push_maybe(if self.error == Some(Error::ParseVolError) {Some(text("Please enter a number between 0 and 100!").color(get_rgb_color(255, 0, 0)))} else {None})
                        }                        
                    ).width(Length::Fill).align_x(Alignment::Center)
                ).on_hover(Message::None).on_exit(Message::None).style(
                    hov_container_row::auto_style(get_rgb_color(150, 150, 150), get_rgb_color(100, 100, 255), 3, 15)
                )
            )
            .push(HovContainer::new().push(Column::new().push(text("Limiter Controls").size(18).height(30).center())
            .push(Column::new()
                .push(Row::new()
                    .push(radio("20%", BuiltIn::Twenty, self.sel_lim, |selection| Message::ChangeLimitSel(selection)))
                    .push(radio("50%", BuiltIn::Fifty, self.sel_lim, |selection| Message::ChangeLimitSel(selection)))
                    .push(radio("80%", BuiltIn::Eighty, self.sel_lim, |selection| Message::ChangeLimitSel(selection)))
                    .push(radio("Custom", BuiltIn::Custom, self.sel_lim, |selection| Message::ChangeLimitSel(selection)))
                    .push_maybe(
                        if self.sel_lim == Some(BuiltIn::Custom) {
                            Some(
                                text_input(&self.percent.to_string(), &self.percent_str)
                                    // .on_input_maybe(if !self.limiter {Some(|input| Message::ChangePercent(input, true))} else {None} )
                                    .on_input(|input| Message::ChangePercent(input, true))
                                    .on_submit(Message::ConfirmPercent(true))
                                    .size(14)
                                    .style(
                                        move |_: &Theme, status| {
                                            match status {
                                                _ => {
                                                    if self.error == Some(Error::ParseError) {
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
                                                            placeholder: get_rgb_color(200, 200, 200),
                                                            selection: get_rgb_color(20, 20, 100),
                                                        }
                                                    }   
                                                },
                                            }
                                        }
                                    ).width(Length::Fixed(40.0)).align_x(Alignment::Center)
                            )
                        } else {
                            None
                        }
                    ).align_y(Alignment::Center).height(30).spacing(20)
                ).spacing(40)
                .push_maybe(if self.error.is_some() && self.error == Some(Error::AdjustWhileOn) {Some(text("Please Turn off the Volume Limiter to Adjust!").color(get_rgb_color(255, 0, 0)))} else {None})
            )
            .push(Row::new()
                .push(
                    Column::new()
                        .push(toggler(self.limiter).label("Enable Volume Limiter").on_toggle(|toggle| Message::OnToggle(toggle)))
                        .push(toggler(self.autolimiter).label("Enable Auto Limiter").on_toggle(|toggle| Message::ChangeAutoLimiter(toggle)))
                        .push(toggler(self.autocheck).label("Enable Auto Check Device Update").on_toggle(|toggle| Message::AutoCheck(toggle)))
                        .push(pick_list(self.all_devices.clone(), self.device.clone(), Message::OnPick))
                    .align_x(Alignment::Center).padding(10).width(Length::FillPortion(1)))
                    .push(Column::new()
                        .push(Row::new()
                            .push(button(" + ").on_press(Message::ChangeByOne(true, true)))
                            .push(text_input(&self.percent.to_string(), &self.percent_str)
                                // .on_input_maybe(if !self.limiter {Some(|input| Message::ChangePercent(input, true))} else {None} )
                                .on_input(|input| Message::ChangePercent(input, true))
                                .on_submit(Message::ConfirmPercent(true))
                                .style(
                                    move |_: &Theme, status| {
                                        match status {
                                            _ => {
                                                if self.error == Some(Error::ParseError) {
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
                                                        placeholder: get_rgb_color(200, 200, 200),
                                                        selection: get_rgb_color(20, 20, 100),
                                                    }
                                                }
                                            },
                                        }
                                    }
                                ).align_x(Alignment::Center).width(Length::Fixed(100.0))
                            )
                            .push(button(" - ").on_press(Message::ChangeByOne(false, true)))
                        )
                        .push_maybe(if self.error == Some(Error::ParseError) {Some(text("Please enter a number between 0 and 100!").color(get_rgb_color(255, 0, 0)))} else {None})
                        .push(text(format!("Current Volume Limit: {}", self.percent)))
                        .push(text(format!{"Current Volume: {}", self.volume}))
                        .push(text("Hello World")).align_x(Alignment::Center).spacing(10).padding(20).width(Length::FillPortion(1))
                    ).padding(20)
                    .align_y(Alignment::Center)
                    .spacing(10)
                ).align_x(Alignment::Center))
                .on_hover(Message::None)
                .on_exit(Message::None)
                .style(
                    hov_container_row::auto_style(get_rgb_color(150, 150, 150), get_rgb_color(100, 100, 255), 3, 15)
                ))
            .push(vertical_space())
            .push(Row::new().push(text("(C) Xephyris 2025").align_x(Alignment::Center).width(Length::Fill).center()).padding(10))
        .spacing(20)
        .padding(10)
        .into()
        
    }

    pub fn theme(_: &VolControl) -> Theme{
        Theme::Dark
    }

    pub fn subscription(&self) -> Subscription<Message>{
        Subscription::batch(vec![
            // iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::UpdateDeviceList),
            // iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::AutoLimiter),
            iced::time::every(std::time::Duration::from_millis(500)).map(|_| Message::SystemVolChange),
            iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::ClearError),
        ])
        
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
            // println!("Current System volume is: {}", get_system_volume_command());
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
    application("Volume Limiter", VolControl::update, VolControl::view).subscription(VolControl::subscription).theme(VolControl::theme).window_size(Size{width:550.0, height:900.0}).run()
    // println!("{}", max_area(Vec::from([1,12,6,2,10,4,8,3])));
    // Ok(())
}