//! Best-effort snapshot of the foreground window (Windows). Other platforms return [`crate::HarnessError::UnsupportedPlatform`].

#[cfg(not(windows))]
use crate::error::HarnessError;
use crate::error::Result;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ForegroundSnapshot {
    /// Executable file name or short app label (e.g. `Code.exe`, `chrome.exe`).
    pub app: String,
    pub title: Option<String>,
}

/// Returns the current foreground window's app name and title.
pub fn current_foreground() -> Result<ForegroundSnapshot> {
    #[cfg(windows)]
    {
        windows_impl::snapshot()
    }
    #[cfg(not(windows))]
    {
        Err(HarnessError::UnsupportedPlatform)
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::ForegroundSnapshot;
    use crate::error::{HarnessError, Result};
    use std::path::Path;
    use windows::core::PWSTR;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId};

    fn utf16_prefix_to_string(buf: &[u16]) -> String {
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..len])
    }

    pub fn snapshot() -> Result<ForegroundSnapshot> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.0.is_null() {
            return Ok(ForegroundSnapshot {
                app: String::from("(no foreground window)"),
                title: None,
            });
        }

        let mut title_buf = [0u16; 512];
        let title_len = unsafe { GetWindowTextW(hwnd, &mut title_buf) } as usize;
        let title = if title_len > 0 {
            Some(utf16_prefix_to_string(
                &title_buf[..title_len.min(title_buf.len())],
            ))
        } else {
            None
        };

        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };

        let app = if pid == 0 {
            String::from("(unknown)")
        } else {
            match unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) } {
                Ok(handle) => {
                    let mut path_buf = [0u16; 1024];
                    let mut size = path_buf.len() as u32;
                    let res = unsafe {
                        QueryFullProcessImageNameW(
                            handle,
                            PROCESS_NAME_WIN32,
                            PWSTR(path_buf.as_mut_ptr()),
                            &mut size,
                        )
                    };
                    let _ = unsafe { CloseHandle(handle) };
                    match res {
                        Ok(()) if size > 0 => {
                            let path = utf16_prefix_to_string(&path_buf[..size as usize]);
                            Path::new(&path)
                                .file_name()
                                .and_then(|s| s.to_str())
                                .map(String::from)
                                .unwrap_or(path)
                        }
                        _ => String::from("(unknown)"),
                    }
                }
                Err(e) => {
                    return Err(HarnessError::Windows(format!(
                        "OpenProcess(pid={pid}): {e}"
                    )));
                }
            }
        };

        Ok(ForegroundSnapshot { app, title })
    }
}
