extern crate xplm;
mod panic;
mod xplmhelpers;

use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Duration;
use thiserror::Error;
use xplm::flight_loop::{FlightLoop, FlightLoopCallback, LoopState};
use xplm::menu::{ActionItem, Menu, MenuClickHandler};
use xplm::plugin::{Plugin, PluginInfo};
use xplm::{debugln, xplane_plugin};

use crate::panic::set_custom_panic;
use crate::xplmhelpers::{xplm_get_system_path, xplm_load_fms_flight_plan};

static HANDLE: Mutex<Option<JoinHandle<Result<String>>>> = Mutex::new(None);
static USERNAME: Mutex<Option<String>> = Mutex::new(None);

#[derive(Debug, Error)]
enum Error {
    #[error("No Simbrief username set")]
    NoSimBriefUsername,
    #[error("Simbrief JSON download failed: {0}")]
    SimbriefJsonDownloadFailed(reqwest::Error),
    #[error("Simbrief JSON is not a string: {0}")]
    SimbriefJsonNotAString(reqwest::Error),
    #[error("Simbrief JSON parsing failed: {0}")]
    SimbriefJsonParsingFailed(serde_json::Error),
    #[error("Simbrief API request failed with status: {0}")]
    SimbriefAPIRequestFailed(String),
    #[error("Simbrief response is malformed: {0}")]
    SimbriefJsonMalformed(&'static str),
    #[error("Simbrief flight plan download failed: {0}")]
    SimbriefFplnDownloadFailed(reqwest::Error),
    #[error("Simbrief flight plan is not a string: {0}")]
    SimbreifFplnIsNotAString(reqwest::Error),
    #[error("Cannot read config file: {0}")]
    CannotReadConfigFile(PathBuf, std::io::Error),
}

type Result<T> = core::result::Result<T, Error>;

struct FlightStreamPlugin {
    _plugins_submenu: Menu,
    _flight_loop: FlightLoop,
}

impl Plugin for FlightStreamPlugin {
    type Error = std::convert::Infallible;

    fn start() -> std::result::Result<Self, Self::Error> {
        set_custom_panic();
        let plugin_submenu = Menu::new("flightstream_rs").unwrap();
        plugin_submenu.add_child(
            ActionItem::new("Download and Load Flight Plan", DownloadAndLoadHandler).unwrap(),
        );
        plugin_submenu.add_child(ActionItem::new("Set username", SetUserNameHandler).unwrap());
        plugin_submenu.add_to_plugins_menu();

        match get_username_from_file() {
            Ok(username) => set_username(username),
            Err(e) => debugln!("flightstream_rs: Unable to get username from file: {e}"),
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
            name: String::from("flightstream_rs"),
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
        if let Some(handle) = HANDLE
            .lock()
            .expect("Lock is panicked")
            .take_if(|h| h.is_finished())
        {
            match handle.join().expect("Fatal error joining worker thread") {
                Ok(data) => xplm_load_fms_flight_plan(&data),
                Err(e) => debugln!("flightstream_rs: Error requesting data from simbrief: {e}"),
            }
        }
    }
}

struct DownloadAndLoadHandler;
impl MenuClickHandler for DownloadAndLoadHandler {
    fn item_clicked(&mut self, _item: &ActionItem) {
        let mut guard = HANDLE
            .lock()
            .expect("flightstream_rs: Download lock is panicked");
        if guard.is_some() {
            println!(
                "flightstream_rs: Error, cannot spawn another download thread, already working"
            );
        } else {
            *guard = Some(std::thread::spawn(request_from_simbrief));
        }
    }
}

struct SetUserNameHandler;
impl MenuClickHandler for SetUserNameHandler {
    fn item_clicked(&mut self, _item: &ActionItem) {
        match get_username_from_file() {
            Ok(username) => set_username(username),
            Err(e) => {
                debugln!("flightstream_rs: Error retrieving username from configuration: {e}");
            }
        }
    }
}

fn request_from_simbrief() -> Result<String> {
    let username: String = get_username().ok_or(Error::NoSimBriefUsername)?;
    let url = format!("https://www.simbrief.com/api/xml.fetcher.php?username={username}&json=1");
    let body = reqwest::blocking::get(url)
        .map_err(Error::SimbriefJsonDownloadFailed)?
        .text()
        .map_err(Error::SimbriefJsonNotAString)?;
    let v: Value = serde_json::from_str(&body).map_err(Error::SimbriefJsonParsingFailed)?;
    let status = v["fetch"]["status"]
        .as_str()
        .ok_or(Error::SimbriefJsonMalformed(
            "fetch/status key is not a string,",
        ))?;
    match status {
        "Success" => get_flight_plan(v),
        _ => Err(Error::SimbriefAPIRequestFailed(status.into())),
    }
}

fn get_flight_plan(v: Value) -> Result<String> {
    let fp_link = v["fms_downloads"]["xpe"]["link"]
        .as_str()
        .ok_or(Error::SimbriefJsonMalformed(
            "fms_downloads/xpe/link key is not a string",
        ))?
        .replace("\"", "");
    let download_link = format!("https://www.simbrief.com/ofp/flightplans/{fp_link}");
    reqwest::blocking::get(download_link)
        .map_err(Error::SimbriefFplnDownloadFailed)?
        .text()
        .map_err(Error::SimbreifFplnIsNotAString)
}

fn get_plugin_path() -> PathBuf {
    let mut path = xplm_get_system_path();
    path.push("Resources");
    path.push("plugins");
    path.push("flightstream_rs");
    path
}

fn set_username(username: String) {
    *USERNAME.lock().unwrap() = Some(username);
}

fn get_username() -> Option<String> {
    USERNAME.lock().unwrap().clone()
}

fn get_username_from_file() -> Result<String> {
    let mut file_path = get_plugin_path();
    file_path.push("username.txt");
    let contents =
        fs::read_to_string(&file_path).map_err(|e| Error::CannotReadConfigFile(file_path, e))?;
    Ok(contents.trim().to_string())
}
