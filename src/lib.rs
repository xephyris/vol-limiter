use std::{sync::mpsc::{Receiver, Sender}, thread::{self, JoinHandle}, time::Duration};

pub mod components;
pub mod styles;
pub mod vol_ctl;

pub enum VolumeCommand {
    GetVol(Option<f32>),
    SetVol(Option<f32>),
    GetDevices(Option<Vec<String>>),
    GetMute(Option<bool>),
    SetMute(Option<bool>),
    Failed,
}

pub fn command_handler(tx: Sender<VolumeCommand>, rx: Receiver<VolumeCommand>) -> JoinHandle<()> {
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(100));
            if let Ok(command) = rx.try_recv() {
                match command {
                    VolumeCommand::GetVol(_ignore) => {
                        tx.send(VolumeCommand::GetVol(Some(cpvc::get_system_volume() as f32 / 100.0))).unwrap();
                    },
                    VolumeCommand::SetVol(vol) => {
                        cpvc::set_system_volume((vol.unwrap() * 100.0) as u8);
                        tx.send(VolumeCommand::SetVol(vol)).unwrap();
                    },
                    VolumeCommand::GetDevices(_ignore) => {
                        tx.send(VolumeCommand::GetDevices(Some(cpvc::get_sound_devices()))).unwrap();
                    },
                    // For cpvc v0.5.0 update (transition in progress)
                    VolumeCommand::GetMute(_ignore) => {
                        if cpvc::get_system_volume() == 0 {
                            tx.send(VolumeCommand::GetMute(Some(true))).unwrap();
                        } else {
                            tx.send(VolumeCommand::GetMute(Some(false))).unwrap();
                        }
                    },
                    // For cpvc v0.5.0 update (transition in progress)
                    VolumeCommand::SetMute(mute) => {
                        if mute.unwrap() {
                            cpvc::set_system_volume(0);
                            tx.send(VolumeCommand::SetMute(Some(true))).unwrap();
                        } else {
                            tx.send(VolumeCommand::SetMute(Some(false))).unwrap();
                        }
                    },
                    VolumeCommand::Failed => {

                    }
                }
            }
        }
    })
}