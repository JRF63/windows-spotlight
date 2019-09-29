extern crate winapi;

use winapi::shared::minwindef::FILETIME;
use winapi::um::minwinbase::SYSTEMTIME;
use winapi::um::timezoneapi::FileTimeToSystemTime;

pub fn datetime_str(time: u64) -> String {
    let ptr: *const u64 = &time;
    let mut systime: SYSTEMTIME = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
    if unsafe { FileTimeToSystemTime(ptr as *const FILETIME, &mut systime) } == 0 {
        panic!()
    }
    format!("{}{:02}{:02}", systime.wYear, systime.wMonth, systime.wDay)
}
