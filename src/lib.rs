extern crate xplm;

use xplm::menu::{ActionItem, Menu, MenuClickHandler};
use xplm::plugin::{Plugin, PluginInfo};
use xplm::{debugln, xplane_plugin};
use xplm::flight_loop::{FlightLoop, LoopState, FlightLoopCallback};
use xplane_sdk_sys::XPLMLoadFMSFlightPlan;
use xplane_sdk_sys::XPLMGetSystemPath;
use xplane_sdk_sys::XPLMGetDirectorySeparator;
use std::ffi::{CStr, CString};
use std::thread::JoinHandle;
use std::sync::Mutex;
use std::time::Duration;
use std::fs;
use reqwest;
use serde_json::{Value};

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
        let plugin_submenu = Menu::new("flightstream-rs").unwrap();
        plugin_submenu.add_child(ActionItem::new("Download and Load Flight Plan", DownloadAndLoadHandler).unwrap());
        plugin_submenu.add_child(ActionItem::new("Set username", SetUserNameHandler).unwrap());
        plugin_submenu.add_to_plugins_menu();

        set_path(get_plugin_path());

        match get_path() {
            Some(p) => debugln!("{p}"),
            None => debugln!("Bad path"),
        }

        set_username(get_username_from_file());
        match get_username()
        {
            Some(u) => debugln!("{u}"),
            None => debugln!("Bad username"),
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
        set_username(get_username_from_file());
        match get_username()
        {
            Some(u) => debugln!("{u}"),
            None => debugln!("Bad username"),
        }
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
    let mut data = DATA.lock().unwrap();
    *data = Some("".to_string());
    std::mem::drop(data);
    match get_username()
    {
        Some(u) => 
        {
            let mut url = "https://www.simbrief.com/api/xml.fetcher.php?username=".to_string();
            url.push_str(u.as_str());
            url.push_str("&json=1");
            debugln!("flightstream-rs: {url}");
            if let Ok(body) = reqwest::blocking::get(url).expect("Bad request").text() {
            let value: &str = body.as_str();
            let v: Value = serde_json::from_str(value).unwrap();
            if v["fetch"]["status"] == "Success" {
                get_flight_plan(v);
            }
            else {
                debugln!("Failed to get request: {}", v["fetch"]["status"]);
            }
    };
        },
        None => debugln!("Bad username"),
    }
    
}

fn get_flight_plan(v: Value)
{
    let fp_link = v["fms_downloads"]["xpe"]["link"].to_string().replace("\"", "");
    let mut download_link= "https://www.simbrief.com/ofp/flightplans/".to_string();
    debugln!("{download_link}");
    download_link.push_str(&fp_link);
    if let Ok(body) = reqwest::blocking::get(download_link).expect("Bad FP request").text() {
        let mut data = DATA.lock().unwrap();
        *data = Some(body);
    }
}

fn get_plugin_path() -> String
{
    let mut buffer = vec![0i8; 512];
    let mut path: String;
    unsafe{
        XPLMGetSystemPath(buffer.as_mut_ptr());
        let c_str = CStr::from_ptr(buffer.as_ptr());
        path = c_str.to_str().unwrap().to_string();
    }
    let div_char;
    unsafe {
        let sep_char = XPLMGetDirectorySeparator();
        let c_char = CStr::from_ptr(sep_char);
        div_char = c_char.to_str().unwrap();
    }
    let div_char = div_char.to_string();
    path = path + &div_char + "Resources" + &div_char + "plugins" + &div_char + "flightstream-rs" + &div_char.to_string();
    path
}

fn set_path(path: String)
{
    let mut path_lock = PATH.lock().unwrap();
    *path_lock = Some(path);
}

fn get_path() -> Option<String>
{
    let path_lock = PATH.lock().unwrap();
    path_lock.clone()
}

fn set_username(username: String)
{
    let mut username_lock = USERNAME.lock().unwrap();
    *username_lock = Some(username);
}

fn get_username() -> Option<String>
{
    let username_lock = USERNAME.lock().unwrap();
    username_lock.clone()
}

fn get_username_from_file() -> String
{
    let file_path = get_path();
    let mut username = String::new();
    let mut _path = String::new();
    match file_path
    {
        Some(p) => {
            _path = p;
            _path.push_str("username.txt");
            let path = _path.clone();
            let contents = fs::read_to_string(_path);
            match contents
            {
                Ok(contents) => 
                {
                    username = contents.trim().to_string();
                },
                Err(_) => 
                {
                    debugln!("Unable to open file at {path}");
                }
            }
        },
        None => debugln!("File path not found"),
    }
    username
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