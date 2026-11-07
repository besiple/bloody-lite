use hidapi::HidApi;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    SystemParametersInfoW, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE, SPI_SETMOUSESPEED,
};

pub const A4TECH_VID: u16 = 0x09DA;

pub struct BloodyStatus {
    pub connected: bool,
    pub product_name: String,
    pub pid: u16,
}

pub fn check_bloody_device() -> BloodyStatus {
    if let Ok(api) = HidApi::new() {
        for device in api.device_list() {
            if device.vendor_id() == A4TECH_VID {
                let name = device
                    .product_string()
                    .unwrap_or("A4Tech Bloody Gaming Mouse")
                    .to_string();
                return BloodyStatus {
                    connected: true,
                    product_name: name,
                    pid: device.product_id(),
                };
            }
        }
    }

    BloodyStatus {
        connected: false,
        product_name: "Не найдена".to_string(),
        pid: 0,
    }
}

pub fn set_windows_sensitivity(speed: u32) -> bool {
    // Speed is clamped to 1..20 (standard Windows sensitivity)
    let speed = speed.clamp(1, 20);
    unsafe {
        let res = SystemParametersInfoW(
            SPI_SETMOUSESPEED,
            0,
            speed as *mut _,
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
        );
        res != 0
    }
}

// Convert DPI to equivalent Windows sensitivity level (1..20) for smooth scaling
pub fn dpi_to_sensitivity(dpi: u32) -> u32 {
    match dpi {
        0..=600 => 6,
        601..=900 => 10,  // 800 DPI = normal 1:1 speed
        901..=1100 => 11, // 1000 DPI
        1101..=1400 => 12, // 1200 DPI
        1401..=2000 => 14, // 1600 DPI
        _ => 18,          // 3200+ DPI
    }
}
