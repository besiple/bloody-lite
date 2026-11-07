use std::sync::atomic::AtomicU16;
use std::sync::Arc;

pub struct HookState {
    pub button4_vk: AtomicU16, // 0 = Default, >0 = Virtual Key
    pub button5_vk: AtomicU16, // 0 = Default, >0 = Virtual Key
}

#[cfg(windows)]
pub fn start_mouse_hook(state: Arc<HookState>) {
    use std::sync::atomic::Ordering;
    use std::thread;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    static mut GLOBAL_STATE: Option<Arc<HookState>> = None;
    static mut HOOK_HANDLE: HHOOK = std::ptr::null_mut();

    unsafe fn send_key_event(vk: u16, key_up: bool) {
        let mut flags = 0;
        if key_up {
            flags |= KEYEVENTF_KEYUP;
        }

        let mut input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        unsafe {
            SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
        }
    }

    unsafe extern "system" fn low_level_mouse_proc(
        n_code: i32,
        w_param: usize,
        l_param: isize,
    ) -> isize {
        if n_code >= 0 {
            let msg = w_param as u32;
            if msg == WM_XBUTTONDOWN || msg == WM_XBUTTONUP {
                let hook_struct = unsafe { *(l_param as *const MSLLHOOKSTRUCT) };
                let x_button = (hook_struct.mouseData >> 16) as u16;
                let is_up = msg == WM_XBUTTONUP;

                if let Some(state) = unsafe { &*std::ptr::addr_of!(GLOBAL_STATE) } {
                    if x_button == 1 {
                        let vk = state.button4_vk.load(Ordering::SeqCst);
                        if vk > 0 {
                            unsafe {
                                send_key_event(vk, is_up);
                            }
                            return 1; // Suppress default click
                        }
                    } else if x_button == 2 {
                        let vk = state.button5_vk.load(Ordering::SeqCst);
                        if vk > 0 {
                            unsafe {
                                send_key_event(vk, is_up);
                            }
                            return 1; // Suppress default click
                        }
                    }
                }
            }
        }

        unsafe { CallNextHookEx(HOOK_HANDLE, n_code, w_param, l_param) }
    }

    unsafe {
        GLOBAL_STATE = Some(state);
    }

    thread::spawn(|| unsafe {
        HOOK_HANDLE = SetWindowsHookExW(
            WH_MOUSE_LL,
            Some(low_level_mouse_proc),
            std::ptr::null_mut(),
            0,
        );

        if HOOK_HANDLE.is_null() {
            eprintln!("Windows: Failed to install mouse hook!");
            return;
        }

        // Message pump for the hook
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        UnhookWindowsHookEx(HOOK_HANDLE);
    });
}

#[cfg(target_os = "linux")]
pub fn start_mouse_hook(state: Arc<HookState>) {
    use crate::keys::vk_to_evdev_keycode;
    use evdev::uinput::VirtualDevice;
    use evdev::{AttributeSet, Device, EventType, InputEvent, KeyCode};
    use std::sync::atomic::Ordering;
    use std::thread;

    thread::spawn(move || {
        // Create virtual keyboard for injecting key events
        let mut keys_set = AttributeSet::<KeyCode>::new();
        // Add common keys to attribute set
        for vk in 0x08..=0x7B {
            if let Some(k) = vk_to_evdev_keycode(vk) {
                keys_set.insert(k);
            }
        }

        let mut vdevice = match VirtualDevice::builder() {
            Ok(builder) => match builder.name("Bloody Lite Virtual Keyboard").with_keys(&keys_set) {
                Ok(b) => b.build().ok(),
                Err(_) => None,
            },
            Err(_) => None,
        };

        if vdevice.is_none() {
            eprintln!("Linux notice: /dev/uinput is not accessible without root or 'input' group.");
            eprintln!("To enable button remapping on Linux: sudo usermod -a -G input $USER");
        }

        // Find Bloody mouse or default mice in /dev/input
        let devices: Vec<Device> = evdev::enumerate()
            .map(|(_, dev)| dev)
            .filter(|dev| {
                dev.supported_keys().map_or(false, |keys| {
                    keys.contains(KeyCode::BTN_SIDE) || keys.contains(KeyCode::BTN_EXTRA)
                })
            })
            .collect();

        if devices.is_empty() {
            eprintln!("Linux: No mice with side buttons found in /dev/input/event*");
            return;
        }

        // Monitor first matching mouse device
        if let Some(mut mouse) = devices.into_iter().next() {
            println!("Linux: Hooking side buttons for device: {:?}", mouse.name());
            loop {
                if let Ok(events) = mouse.fetch_events() {
                    for ev in events {
                        if ev.event_type() == EventType::KEY {
                            let key = KeyCode::new(ev.code());
                            let is_down = ev.value() == 1;
                            let is_up = ev.value() == 0;

                            let target_vk = if key == KeyCode::BTN_SIDE {
                                state.button4_vk.load(Ordering::SeqCst)
                            } else if key == KeyCode::BTN_EXTRA {
                                state.button5_vk.load(Ordering::SeqCst)
                            } else {
                                0
                            };

                            if target_vk > 0 {
                                if let Some(vdev) = vdevice.as_mut() {
                                    if let Some(ev_key) = vk_to_evdev_keycode(target_vk) {
                                        let val = if is_down { 1 } else if is_up { 0 } else { continue };
                                        let out_ev = InputEvent::new(EventType::KEY.0, ev_key.code(), val);
                                        let _ = vdev.emit(&[out_ev]);
                                    }
                                }
                            }
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
    });
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn start_mouse_hook(_state: Arc<HookState>) {
    // Stub for unsupported OS
}
