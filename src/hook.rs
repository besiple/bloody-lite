use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::thread;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub struct HookState {
    pub button4_vk: AtomicU16, // 0 = Default, >0 = Virtual Key
    pub button5_vk: AtomicU16, // 0 = Default, >0 = Virtual Key
}

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

pub fn start_mouse_hook(state: Arc<HookState>) {
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
            eprintln!("Failed to install mouse hook!");
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
