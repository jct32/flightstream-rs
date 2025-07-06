extern crate xplm;

use xplm::menu::{ActionItem, Menu, MenuClickHandler};
use xplm::plugin::{Plugin, PluginInfo};
use xplm::{debugln, xplane_plugin};
use xplm::flight_loop::{FlightLoop, LoopState, FlightLoopCallback};
use xplane_sdk_sys::XPLMLoadFMSFlightPlan;
use std::ffi::{CString};
use std::thread::JoinHandle;
use std::sync::Mutex;
use std::time::Duration;
use reqwest;
use serde_json::{Value};

static HANDLE: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);
static DATA: Mutex<Option<String>> = Mutex::new(None);

struct FlightStreamPlugin {
    _plugins_submenu: Menu,
    _flight_loop: FlightLoop,
}

impl Plugin for FlightStreamPlugin {
    type Error = std::convert::Infallible;

    fn start() -> std::result::Result<Self, Self::Error> {
        let plugin_submenu = Menu::new("flightstream-rs").unwrap();
        plugin_submenu.add_child(ActionItem::new("Download and Load Flight Plan", DownloadAndLoadHandler).unwrap());
        plugin_submenu.add_child(ActionItem::new("Set username", SetUserNameHandler).unwrap());
        plugin_submenu.add_to_plugins_menu();
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
            signature: String::from("jct32.flighstream"), 
            description: String::from("A plugin for downloading a Simbrief flight plan to the X1000") }
    }
}

xplane_plugin!(FlightStreamPlugin);

struct DownloadAndLoadHandler;


impl MenuClickHandler for DownloadAndLoadHandler {
    fn item_clicked(&mut self, _item: &ActionItem) {
        let mut guard = HANDLE.lock().expect("Lock is panciked");
        if guard.is_some() {
            println!("Error, cannot spawn another thread, already working");
        }
        else {
                *guard = Some(std::thread::spawn(|| {
                    request_from_simbrief();
                }));
        }
        debugln!("Download Selected");
    }
}

struct SetUserNameHandler;

impl MenuClickHandler for SetUserNameHandler {
    fn item_clicked(&mut self, _item: &ActionItem) {
        debugln!("Set Username Selected");
    }
}


unsafe fn call_load()
{
    let data_lock = DATA.lock().unwrap();
    if let Some(ref data) = *data_lock {
        let plan = CString::new(data.as_str()).unwrap();
        unsafe {XPLMLoadFMSFlightPlan(0, plan.as_ptr(), u32::try_from(plan.count_bytes()).unwrap());}
    }
}

fn request_from_simbrief()
{
    if let Ok(body) = reqwest::blocking::get("https://www.simbrief.com/api/xml.fetcher.php?username=jct323&json=1").expect("Bad request").text() {
        let value: &str = body.as_str();
        let v: Value = serde_json::from_str(value).unwrap();
        if v["fetch"]["status"] == "Success" {
            get_flight_plan(v);
        }
        else {
            debugln!("Failed to get request: {}", v["fetch"]["status"]);
        }
    };
}

fn get_flight_plan(v: Value)
{
    let fp_link = v["fms_downloads"]["xpe"]["link"].to_string().replace("\"", "");
    let mut download_link= "https://www.simbrief.com/ofp/flightplans/".to_string();
    download_link.push_str(&fp_link);
    if let Ok(body) = reqwest::blocking::get(download_link).expect("Bad FP request").text() {
        let mut data = DATA.lock().unwrap();
        *data = Some(body);
    }
}

struct LoopHandler;

impl FlightLoopCallback for LoopHandler {
    fn flight_loop(&mut self, _state: &mut LoopState) {
        let mut lock_guard = HANDLE.lock().expect("Lock is panicked");
        if let Some(ref handle) = *lock_guard
        {
            if handle.is_finished() {
                *lock_guard = None;
                unsafe{call_load();}
            }
        }
    }
}