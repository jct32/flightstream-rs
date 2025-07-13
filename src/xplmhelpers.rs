use std::ffi::{CStr, CString};
use xplane_sdk_sys::XPLMGetDirectorySeparator;
use xplane_sdk_sys::XPLMGetSystemPath;
use xplane_sdk_sys::XPLMLoadFMSFlightPlan;

pub fn xplm_load_fms_flight_plan(fp_data: &String) {
    let plan = CString::new(fp_data.as_str()).unwrap();
    unsafe {
        XPLMLoadFMSFlightPlan(0, plan.as_ptr(), u32::try_from(plan.count_bytes()).unwrap());
    }
}

pub fn xplm_get_system_path() -> String {
    let mut buffer = vec![0i8; 512];
    let path;
    unsafe {
        XPLMGetSystemPath(buffer.as_mut_ptr());
        let c_str = CStr::from_ptr(buffer.as_ptr());
        path = c_str.to_str().unwrap().to_string()
    }
    path
}

pub fn xplm_get_directory_separator() -> String {
    let div_char;
    unsafe {
        let sep_char = XPLMGetDirectorySeparator();
        let c_char = CStr::from_ptr(sep_char);
        div_char = c_char.to_str().unwrap();
    }
    div_char.to_string()
}
