extern crate xplm;

use xplm::plugin::{Plugin, PluginInfo};
use xplm::{debugln, xplane_plugin};

struct FlightStreamPlugin;

impl Plugin for FlightStreamPlugin {
    type Error = std::convert::Infallible;

    fn start() -> Result<Self, Self::Error> {
        debugln!("Hello, World! From minimal plugin");
        Ok(FlightStreamPlugin)
    }

    fn info(&self) -> PluginInfo {
        PluginInfo { 
            name: String::from("Flightstream-rs"), 
            signature: String::from("jct32.flighstream"), 
            description: String::from("A plugin for downloading a Simbrief flight plan to the X1000") }
    }
}

xplane_plugin!(FlightStreamPlugin);