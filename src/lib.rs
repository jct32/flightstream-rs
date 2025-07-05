extern crate xplm;

use xplm::menu::{ActionItem, Menu, MenuClickHandler};
use xplm::plugin::{Plugin, PluginInfo};
use xplm::{debugln, xplane_plugin};
use xplane_sdk_sys::XPLMLoadFMSFlightPlan;
use std::ffi::{CString};

struct FlightStreamPlugin {
    _plugins_submenu: Menu,
}

impl Plugin for FlightStreamPlugin {
    type Error = std::convert::Infallible;

    fn start() -> Result<Self, Self::Error> {
        let plugin_submenu = Menu::new("flightstream-rs").unwrap();
        plugin_submenu.add_child(ActionItem::new("Download and Load Flight Plan", DownloadAndLoadHandler).unwrap());
        plugin_submenu.add_child(ActionItem::new("Set username", SetUserNameHandler).unwrap());
        plugin_submenu.add_to_plugins_menu();
        Ok(FlightStreamPlugin {
            _plugins_submenu: plugin_submenu
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
        const FLIGHTPLAN: &str = "I
        1100 Version
        CYCLE 2112
        ADEP EDDS
        DEPRWY RW25
        SID ETAS4B
        ADES EDDF
        DESRWY RW25L
        STAR SPES3B
        APP I25L
        APPTRANS CHA
        NUMENR 6
        1 EDDS ADEP 1272.000000 48.689877 9.221964
        11 XINLA T163 0.000000 49.283646 9.141608
        11 SUKON T163 0.000000 49.659721 9.195556
        11 SUPIX T163 0.000000 49.727779 9.305278
        11 SPESA T163 0.000000 49.862240 9.348325
        1 EDDF ADES 354.000000 50.033306 8.570456";
        unsafe{call_load(FLIGHTPLAN)};
        debugln!("Download Selected");
    }
}

struct SetUserNameHandler;

impl MenuClickHandler for SetUserNameHandler {
    fn item_clicked(&mut self, _item: &ActionItem) {
        debugln!("Set Username Selected");
    }
}


unsafe fn call_load(data: &str)
{
    let plan = CString::new(data).unwrap();
    unsafe {XPLMLoadFMSFlightPlan(0, plan.as_ptr(), u32::try_from(plan.count_bytes()).unwrap());}
}