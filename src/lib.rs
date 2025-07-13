extern crate xplm;
mod panic;
mod xplmhelpers;

use reqwest;
use serde_json::Value;
use std::fs;
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Duration;
use xplm::flight_loop::{FlightLoop, FlightLoopCallback, LoopState};
use xplm::menu::{ActionItem, Menu, MenuClickHandler};
use xplm::plugin::{Plugin, PluginInfo};
use xplm::{debugln, xplane_plugin};

use crate::panic::set_custom_panic;
use crate::xplmhelpers::{
    xplm_get_directory_separator, xplm_get_system_path, xplm_load_fms_flight_plan,
};

static HANDLE: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);
static DATA: Mutex<Option<String>> = Mutex::new(None);
static PATH: Mutex<Option<String>> = Mutex::new(None);
static USERNAME: Mutex<Option<String>> = Mutex::new(None);

struct FlightStreamPlugin {
    _plugins_submenu: Menu,
    _flight_loop: FlightLoop,
}

impl Plugin for FlightStreamPlugin {
    type Error = std::convert::Infallible;

    fn start() -> std::result::Result<Self, Self::Error> {
        set_custom_panic();
        let plugin_submenu = Menu::new("flightstream-rs").unwrap();
        plugin_submenu.add_child(
            ActionItem::new("Download and Load Flight Plan", DownloadAndLoadHandler).unwrap(),
        );
        plugin_submenu.add_child(ActionItem::new("Set username", SetUserNameHandler).unwrap());
        plugin_submenu.add_to_plugins_menu();

        set_path(get_plugin_path());

        match get_path() {
            Some(p) => debugln!("{p}"),
            None => debugln!("flightsteam-rs: Bad X-Plane directory path"),
        }

        set_username(get_username_from_file());
        match get_username() {
            Some(u) => debugln!("{u}"),
            None => debugln!("flightstream-rs: Unable to get username from file"),
        }

        let mut flight_loop = FlightLoop::new(LoopHandler);

        flight_loop.schedule_after(Duration::from_millis(100));

        Ok(FlightStreamPlugin {
            _plugins_submenu: plugin_submenu,
            _flight_loop: flight_loop,
        })
    }

    fn info(&self) -> PluginInfo {
        PluginInfo {
            name: String::from("flightstream-rs"),
            signature: String::from("jct32.flightstream"),
            description: String::from(
                "A plugin for downloading a Simbrief flight plan to the X1000",
            ),
        }
    }
}

xplane_plugin!(FlightStreamPlugin);

struct LoopHandler;
impl FlightLoopCallback for LoopHandler {
    fn flight_loop(&mut self, _state: &mut LoopState) {
        let mut lock_guard = HANDLE.lock().expect("Lock is panicked");
        if let Some(ref handle) = *lock_guard {
            if handle.is_finished() {
                *lock_guard = None;
                unsafe {
                    call_load();
                }
            }
        }
    }
}

struct DownloadAndLoadHandler;
impl MenuClickHandler for DownloadAndLoadHandler {
    fn item_clicked(&mut self, _item: &ActionItem) {
        let mut guard = HANDLE
            .lock()
            .expect("flightstream-rs: Download lock is panicked");
        if guard.is_some() {
            println!(
                "flightstream-rs: Error, cannot spawn another download thread, already working"
            );
        } else {
            *guard = Some(std::thread::spawn(|| {
                request_from_simbrief();
            }));
        }
    }
}

struct SetUserNameHandler;
impl MenuClickHandler for SetUserNameHandler {
    fn item_clicked(&mut self, _item: &ActionItem) {
        set_username(get_username_from_file());
        match get_username() {
            Some(u) => debugln!("{u}"),
            None => debugln!("flightstream-rs: No username"),
        }
    }
}

unsafe fn call_load() {
    let data_lock = DATA.lock().unwrap();
    if let Some(ref data) = *data_lock {
        xplm_load_fms_flight_plan(data);
    }
}

fn request_from_simbrief() {
    let mut data = DATA.lock().unwrap();
    *data = Some("".to_string());
    std::mem::drop(data);
    match get_username() {
        Some(u) => {
            let mut url = "https://www.simbrief.com/api/xml.fetcher.php?username=".to_string();
            url.push_str(u.as_str());
            url.push_str("&json=1");
            if let Ok(body) = reqwest::blocking::get(url)
                .expect("flightstream-rs: Unable to get request")
                .text()
            {
                let value: &str = body.as_str();
                let v: Value = serde_json::from_str(value).unwrap();
                if v["fetch"]["status"] == "Success" {
                    get_flight_plan(v);
                } else {
                    debugln!("flightstream-rs: Bad request: {}", v["fetch"]["status"]);
                }
            };
        }
        None => debugln!("flightstream-rs: No username"),
    }
}

fn get_flight_plan(v: Value) {
    let fp_link = v["fms_downloads"]["xpe"]["link"]
        .to_string()
        .replace("\"", "");
    let mut download_link = "https://www.simbrief.com/ofp/flightplans/".to_string();
    debugln!("{download_link}");
    download_link.push_str(&fp_link);
    if let Ok(body) = reqwest::blocking::get(download_link)
        .expect("flighstream-rs: Bad FP request link")
        .text()
    {
        let mut data = DATA.lock().unwrap();
        *data = Some(body);
    }
}

fn get_plugin_path() -> String {
    let mut path = xplm_get_system_path();
    let div_char = xplm_get_directory_separator();
    path = path
        + &div_char
        + "Resources"
        + &div_char
        + "plugins"
        + &div_char
        + "flightstream-rs"
        + &div_char.to_string();
    path
}

fn set_path(path: String) {
    let mut path_lock = PATH.lock().unwrap();
    *path_lock = Some(path);
}

fn get_path() -> Option<String> {
    let path_lock = PATH.lock().unwrap();
    path_lock.clone()
}

fn set_username(username: String) {
    let mut username_lock = USERNAME.lock().unwrap();
    *username_lock = Some(username);
}

fn get_username() -> Option<String> {
    let username_lock = USERNAME.lock().unwrap();
    username_lock.clone()
}

fn get_username_from_file() -> String {
    let file_path = get_path();
    let mut username = String::new();
    let mut _path = String::new();
    match file_path {
        Some(p) => {
            _path = p;
            _path.push_str("username.txt");
            let path = _path.clone();
            let contents = fs::read_to_string(_path);
            match contents {
                Ok(contents) => {
                    username = contents.trim().to_string();
                }
                Err(_) => {
                    debugln!("flightstream-rs: Unable to open file at {path}");
                }
            }
        }
        None => debugln!("flightstream-rs: File path not found"),
    }
    username
}
