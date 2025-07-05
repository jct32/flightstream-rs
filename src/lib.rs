extern crate xplm;

use xplm::menu::{ActionItem, Menu, MenuClickHandler};
use xplm::plugin::{Plugin, PluginInfo};
use xplm::{debugln, xplane_plugin};

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
        debugln!("Download Selected");
    }
}

struct SetUserNameHandler;

impl MenuClickHandler for SetUserNameHandler {
    fn item_clicked(&mut self, _item: &ActionItem) {
        debugln!("Set Username Selected");
    }
}