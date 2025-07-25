use std::ffi::c_char;
use xplane_sdk_sys::{XPLMGetSystemPath, XPLMLoadFMSFlightPlan};

pub fn xplm_load_fms_flight_plan(fp_data: &str) {
    unsafe {
        XPLMLoadFMSFlightPlan(0, fp_data.as_ptr() as *const c_char, fp_data.len() as u32);
    }
}

pub fn xplm_get_system_path() -> std::path::PathBuf {
    let mut buffer = [0u8; 512];
    unsafe {
        XPLMGetSystemPath(buffer.as_mut_ptr() as *mut c_char);
    }
    let s = std::ffi::CStr::from_bytes_until_nul(&buffer)
        .unwrap()
        .to_str()
        .unwrap();
    std::path::PathBuf::from(s)
}
