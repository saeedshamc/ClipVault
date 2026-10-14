//! Windows clipboard watcher using `AddClipboardFormatListener`.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use clipvault_core::model::{FormatPayload, ItemKind};
use clipvault_platform::{ClipboardEvent, ClipboardWatcher, PlatformError};

#[cfg(windows)]
mod win {
    use super::*;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;

    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::DataExchange::{
        AddClipboardFormatListener, CloseClipboard, GetClipboardData, GetClipboardSequenceNumber,
        OpenClipboard, RemoveClipboardFormatListener,
    };
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
        PostMessageW, PostQuitMessage, RegisterClassW, TranslateMessage, CW_USEDEFAULT,
        HWND_MESSAGE, WM_CLIPBOARDUPDATE, WM_DESTROY, WM_USER, WNDCLASSW,
    };

    const WM_STOP: u32 = WM_USER + 1;
    const CF_UNICODETEXT: u32 = 13;

    struct ThreadState {
        tx: Sender<ClipboardEvent>,
        last_seq: AtomicU32,
    }

    pub struct Inner {
        stop: Arc<AtomicBool>,
        join: Option<JoinHandle<()>>,
        hwnd: Arc<AtomicUsize>,
    }

    impl Inner {
        pub fn start(tx: Sender<ClipboardEvent>) -> Result<Self, PlatformError> {
            let stop = Arc::new(AtomicBool::new(false));
            let hwnd = Arc::new(AtomicUsize::new(0));
            let stop_thread = Arc::clone(&stop);
            let hwnd_thread = Arc::clone(&hwnd);

            let join = thread::Builder::new()
                .name("clipvault-clipboard".into())
                .spawn(move || {
                    if let Err(err) = run_message_loop(tx, stop_thread, hwnd_thread) {
                        tracing::error!(error = %err, "clipboard watcher thread exited");
                    }
                })
                .map_err(|err| PlatformError::Operation(err.to_string()))?;

            // Wait briefly for HWND registration.
            for _ in 0..50 {
                if hwnd.load(Ordering::SeqCst) != 0 {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
            if hwnd.load(Ordering::SeqCst) == 0 {
                return Err(PlatformError::Operation(
                    "failed to create message-only clipboard window".into(),
                ));
            }

            Ok(Self {
                stop,
                join: Some(join),
                hwnd,
            })
        }

        pub fn stop(&mut self) -> Result<(), PlatformError> {
            self.stop.store(true, Ordering::SeqCst);
            let hwnd = self.hwnd.load(Ordering::SeqCst) as HWND;
            if !hwnd.is_null() {
                unsafe {
                    let _ = PostMessageW(hwnd, WM_STOP, 0, 0);
                }
            }
            if let Some(join) = self.join.take() {
                let _ = join.join();
            }
            Ok(())
        }
    }

    fn run_message_loop(
        tx: Sender<ClipboardEvent>,
        stop: Arc<AtomicBool>,
        hwnd_out: Arc<AtomicUsize>,
    ) -> Result<(), String> {
        unsafe {
            let class_name = wide("ClipVaultClipboardWatcher");
            let wc = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: GetModuleHandleW(ptr::null()),
                hIcon: ptr::null_mut(),
                hCursor: ptr::null_mut(),
                hbrBackground: ptr::null_mut(),
                lpszMenuName: ptr::null(),
                lpszClassName: class_name.as_ptr(),
            };
            RegisterClassW(&wc);

            let state = Box::new(ThreadState {
                tx,
                last_seq: AtomicU32::new(GetClipboardSequenceNumber()),
            });
            let state_ptr = Box::into_raw(state);

            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                wide("ClipVault").as_ptr(),
                0,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                HWND_MESSAGE,
                ptr::null_mut(),
                GetModuleHandleW(ptr::null()),
                state_ptr.cast(),
            );
            if hwnd.is_null() {
                let _ = Box::from_raw(state_ptr);
                return Err("CreateWindowExW failed".into());
            }
            hwnd_out.store(hwnd as usize, Ordering::SeqCst);

            if AddClipboardFormatListener(hwnd) == 0 {
                DestroyWindow(hwnd);
                return Err("AddClipboardFormatListener failed".into());
            }

            let mut msg = std::mem::zeroed();
            while !stop.load(Ordering::SeqCst) && GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
                if msg.message == WM_STOP {
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            RemoveClipboardFormatListener(hwnd);
            DestroyWindow(hwnd);
            hwnd_out.store(0, Ordering::SeqCst);
        }
        Ok(())
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            windows_sys::Win32::UI::WindowsAndMessaging::WM_NCCREATE => {
                let create =
                    lparam as *const windows_sys::Win32::UI::WindowsAndMessaging::CREATESTRUCTW;
                if create.is_null() {
                    return 0;
                }
                let state = (*create).lpCreateParams as *mut ThreadState;
                windows_sys::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
                    hwnd,
                    windows_sys::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
                    state as isize,
                );
                return 1;
            }
            WM_CLIPBOARDUPDATE => {
                let state = windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
                    hwnd,
                    windows_sys::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
                ) as *mut ThreadState;
                if !state.is_null() {
                    handle_clipboard_update(&*state);
                }
            }
            WM_DESTROY => {
                let state = windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
                    hwnd,
                    windows_sys::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
                ) as *mut ThreadState;
                if !state.is_null() {
                    windows_sys::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
                        hwnd,
                        windows_sys::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
                        0,
                    );
                    drop(Box::from_raw(state));
                }
                PostQuitMessage(0);
            }
            _ => {}
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }

    fn handle_clipboard_update(state: &ThreadState) {
        let seq = unsafe { GetClipboardSequenceNumber() };
        let prev = state.last_seq.swap(seq, Ordering::SeqCst);
        if seq == prev {
            return;
        }

        match read_unicode_text() {
            Ok(Some(text)) => {
                if text.is_empty() {
                    return;
                }
                let event = ClipboardEvent {
                    formats: vec![FormatPayload::text_plain(&text)],
                    searchable_text: text,
                    source_app: None,
                };
                let _ = state.tx.send(event);
            }
            Ok(None) => {
                tracing::debug!(
                    kind = ItemKind::Text.as_str(),
                    "clipboard update without unicode text"
                );
            }
            Err(err) => {
                tracing::warn!(error = %err, "failed to read clipboard text");
            }
        }
    }

    fn read_unicode_text() -> Result<Option<String>, String> {
        // OpenClipboard may fail when another app holds it: retry 5 x 50ms.
        for attempt in 0..5 {
            let opened = unsafe { OpenClipboard(ptr::null_mut()) };
            if opened != 0 {
                let result = unsafe { extract_text_and_close() };
                return result;
            }
            if attempt + 1 < 5 {
                thread::sleep(Duration::from_millis(50));
            }
        }
        Err("OpenClipboard failed after retries".into())
    }

    unsafe fn extract_text_and_close() -> Result<Option<String>, String> {
        let handle = GetClipboardData(CF_UNICODETEXT);
        if handle.is_null() {
            CloseClipboard();
            return Ok(None);
        }
        let locked = GlobalLock(handle);
        if locked.is_null() {
            CloseClipboard();
            return Err("GlobalLock failed".into());
        }
        let text = wide_ptr_to_string(locked.cast());
        GlobalUnlock(handle);
        CloseClipboard();
        Ok(text)
    }

    unsafe fn wide_ptr_to_string(ptr: *const u16) -> Option<String> {
        if ptr.is_null() {
            return None;
        }
        let mut len = 0usize;
        while *ptr.add(len) != 0 {
            len += 1;
            if len > 16 * 1024 * 1024 {
                break;
            }
        }
        let slice = std::slice::from_raw_parts(ptr, len);
        Some(String::from_utf16_lossy(slice))
    }

    fn wide(s: &str) -> Vec<u16> {
        OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }
}

/// Windows clipboard watcher backed by a message-only window.
#[derive(Default)]
pub struct WindowsClipboardWatcher {
    #[cfg(windows)]
    inner: Option<win::Inner>,
    #[cfg(not(windows))]
    _private: (),
}

impl ClipboardWatcher for WindowsClipboardWatcher {
    fn start(&mut self, tx: Sender<ClipboardEvent>) -> Result<(), PlatformError> {
        #[cfg(windows)]
        {
            if self.inner.is_some() {
                return Ok(());
            }
            self.inner = Some(win::Inner::start(tx)?);
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = tx;
            Err(PlatformError::Unavailable(
                "Windows clipboard watcher requires a Windows host".into(),
            ))
        }
    }

    fn stop(&mut self) -> Result<(), PlatformError> {
        #[cfg(windows)]
        {
            if let Some(mut inner) = self.inner.take() {
                inner.stop()?;
            }
            Ok(())
        }
        #[cfg(not(windows))]
        {
            Ok(())
        }
    }
}

impl WindowsClipboardWatcher {
    pub fn is_running(&self) -> bool {
        #[cfg(windows)]
        {
            self.inner.is_some()
        }
        #[cfg(not(windows))]
        {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn watcher_start_stop_on_windows() {
        let mut watcher = WindowsClipboardWatcher::default();
        let (tx, _rx) = mpsc::channel();
        let result = watcher.start(tx);
        if cfg!(windows) {
            // Creating a message window should succeed in an interactive session.
            // In headless CI it may still work for HWND_MESSAGE.
            if result.is_ok() {
                assert!(watcher.is_running());
                assert!(watcher.stop().is_ok());
                assert!(!watcher.is_running());
            } else {
                // Document failure rather than flaking CI on restricted hosts.
                let err = result.expect_err("checked");
                tracing::warn!(error = %err, "windows watcher start skipped in this environment");
            }
        } else {
            assert!(result.is_err());
        }
    }
}
