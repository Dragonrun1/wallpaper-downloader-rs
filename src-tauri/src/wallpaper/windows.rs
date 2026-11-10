use std::path::Path;
use anyhow::Result;

pub fn set(path: &Path) -> Result<()> {
    use windows::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW, SPIF_SENDWININICHANGE, SPIF_UPDATEINIFILE, SPI_SETDESKWALLPAPER,
    };
    use windows::Win32::Foundation::BOOL;

    let wide_path: Vec<u16> = path
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let result = unsafe {
        SystemParametersInfoW(
            SPI_SETDESKWALLPAPER,
            0,
            wide_path.as_ptr() as *const core::ffi::c_void,
            SPIF_SENDWININICHANGE,
        )
    };

    if result == BOOL(0) {
        anyhow::bail!(
            "SystemParametersInfoW failed (error: {})",
            std::io::Error::last_os_error()
        );
    }

    Ok(())
}
