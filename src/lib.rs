use std::process::Command;

pub fn get_sound_devices() -> Vec<String> {
    let mut devices:Vec<String> = Vec::new();
    #[cfg(target_os="macos")] {
        let output = Command::new("system_profiler").arg("SPAudioDataType").output().expect("Are you running on MacOS?");
        let lines:Vec<String> = String::from_utf8_lossy(&output.stdout).to_string().lines().map(|str| str.to_owned()).collect();
        for (num, line) in lines.iter().enumerate() {
            if !line.contains("          ") && num > 3 && line != ""{
                if !lines.get(num + 3).unwrap().contains("Input") {
                    devices.push(line.trim().replace(":", "").to_owned());
                }
            }
        }
        // dbg!("{}", devices);
    }
    #[cfg(target_os="windows")] {

    }
    devices
}

pub fn get_system_volume() -> u8 {
    #[allow(unused_assignments)]
    let mut vol = 0;
    #[cfg(target_os="macos")] {
        let output = Command::new("osascript").arg("-e").arg("return output volume of (get volume settings)").output().expect("Are you running on MacOS?");
        let out = String::from_utf8_lossy(&output.stdout).to_string().trim().to_owned();
        vol = out.parse::<u8>().unwrap_or(0);
    }
    vol
    
}

pub fn set_system_volume(percent: u8) -> bool {
    // println!("Setting vol to {}", format!("set Volume {}", (percent as f32 / 14.29 * 100.0).round() / 100.0));
    #[allow(unused_assignments)]
    let mut success = false;
    #[cfg(target_os="macos")]{
        let output = Command::new("osascript").arg("-e").arg(format!("set Volume {}",(percent as f32 / 14.29 * 100.0).round() / 100.0)).output().expect("Are you running on MacOS?");
        // dbg!(output);
        success = output.status.success();
    }
    success
}



