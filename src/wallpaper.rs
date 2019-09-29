extern crate winapi;

use winapi::um::*;

pub fn set_desktop_wallpaper(path: std::path::PathBuf) {
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;

    let mut wchar_str: Vec<u16> = path.into_os_string().encode_wide().chain(once(0)).collect();
    unsafe {
        winuser::SystemParametersInfoW(
            winuser::SPI_SETDESKWALLPAPER,
            0,
            wchar_str.as_mut_ptr() as *mut std::ffi::c_void,
            winuser::SPIF_UPDATEINIFILE | winuser::SPIF_SENDCHANGE,
        );
    }
}
