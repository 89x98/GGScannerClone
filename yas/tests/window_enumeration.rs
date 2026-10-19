#![cfg(target_os = "windows")]

use std::collections::HashSet;
use std::ptr::null_mut;
use std::sync::{Arc, Barrier};

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow};

struct HiddenWindow(HWND);

impl HiddenWindow {
    fn new() -> Self {
        let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        // A hidden top-level window gives each worker a stable enumeration
        // sentinel without depending on whichever apps happen to be open.
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                null_mut(),
                0,
                0,
                0,
                1,
                1,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
            )
        };
        assert!(!hwnd.is_null(), "{}", std::io::Error::last_os_error());
        Self(hwnd)
    }
}

impl Drop for HiddenWindow {
    fn drop(&mut self) {
        unsafe { DestroyWindow(self.0) };
    }
}

#[test]
fn concurrent_enumerations_keep_each_call_complete_and_unique() {
    const THREADS: usize = 8;
    const ITERATIONS: usize = 256;
    let start = Arc::new(Barrier::new(THREADS));
    let workers: Vec<_> = (0..THREADS)
        .map(|_| {
            let start = start.clone();
            std::thread::spawn(move || {
                start.wait();
                let window = HiddenWindow::new();
                for _ in 0..ITERATIONS {
                    let handles = yas_core::utils::iterate_window();
                    assert!(handles.contains(&window.0), "own window was omitted");
                    assert!(handles.iter().all(|handle| !handle.is_null()));
                    let unique: HashSet<_> = handles.iter().copied().collect();
                    assert_eq!(handles.len(), unique.len(), "duplicate window handles");
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().expect("window enumeration worker panicked");
    }
}
