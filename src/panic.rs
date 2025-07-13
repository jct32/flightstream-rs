use std::panic::PanicHookInfo;
use std::sync::Mutex;
use xplm::debugln;

static INITED: Mutex<Option<()>> = Mutex::new(None);

pub fn set_custom_panic() {
    let mut inited = INITED.lock().expect("Mutex is panicked");
    if !inited.is_some() {
        std::panic::set_hook(Box::new(|panic_info| panic_handler(panic_info)));
        *inited = Some(());
    }
}

fn panic_handler(panic_info: &PanicHookInfo) {
    if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
        debugln!(
            "{}\nBacktrace:\n{}",
            s,
            std::backtrace::Backtrace::force_capture()
        );
    } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
        debugln!(
            "{}\nBacktrace:\n{}",
            s,
            std::backtrace::Backtrace::force_capture()
        );
    } else {
        debugln!(
            "Unknown Panic\nBacktrace:\n{}",
            std::backtrace::Backtrace::force_capture()
        );
    }
}
