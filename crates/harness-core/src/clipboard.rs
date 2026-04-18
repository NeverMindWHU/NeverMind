//! Windows clipboard: sequence counter + optional Unicode text (truncated for events).

use crate::error::Result;
use crate::event::DesktopEvent;

const CF_UNICODETEXT: u32 = 13;
const MAX_CLIPBOARD_BYTES: usize = 512 * 1024;
const MAX_PREVIEW_CHARS: usize = 512;

/// Clipboard change counter (Windows). On other platforms returns `0` (no clipboard events).
pub fn clipboard_sequence_number() -> Result<u32> {
    #[cfg(windows)]
    {
        use windows::Win32::System::DataExchange::GetClipboardSequenceNumber;
        Ok(unsafe { GetClipboardSequenceNumber() })
    }
    #[cfg(not(windows))]
    {
        Ok(0)
    }
}

/// Read Unicode clipboard text and build a [`DesktopEvent::Clipboard`], or `None` if empty / not text / busy.
pub fn try_read_unicode_clipboard_event() -> Result<Option<DesktopEvent>> {
    #[cfg(windows)]
    {
        windows_impl::try_read_unicode_clipboard_event()
    }
    #[cfg(not(windows))]
    {
        Ok(None)
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::{CF_UNICODETEXT, MAX_CLIPBOARD_BYTES, MAX_PREVIEW_CHARS};
    use crate::error::Result;
    use crate::event::DesktopEvent;
    use windows::Win32::Foundation::{HANDLE, HGLOBAL};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    };
    use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};

    pub fn try_read_unicode_clipboard_event() -> Result<Option<DesktopEvent>> {
        unsafe {
            if IsClipboardFormatAvailable(CF_UNICODETEXT).is_err() {
                return Ok(None);
            }
            if OpenClipboard(None).is_err() {
                return Ok(None);
            }
        }

        let out = (|| -> Result<Option<DesktopEvent>> {
            let h: HANDLE = match unsafe { GetClipboardData(CF_UNICODETEXT) } {
                Ok(h) if !h.is_invalid() => h,
                _ => return Ok(None),
            };
            let hg = HGLOBAL(h.0);

            let sz = unsafe { GlobalSize(hg) };
            if sz == 0 {
                return Ok(None);
            }
            let sz = sz.min(MAX_CLIPBOARD_BYTES);
            let ptr = unsafe { GlobalLock(hg) };
            if ptr.is_null() {
                return Ok(None);
            }

            let wide_len = sz / std::mem::size_of::<u16>();
            let slice = unsafe { std::slice::from_raw_parts(ptr as *const u16, wide_len) };
            let len = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());
            let text = String::from_utf16_lossy(&slice[..len]);

            let _ = unsafe { GlobalUnlock(hg) };

            let char_len_u = text.chars().count();
            let char_len = char_len_u.min(u32::MAX as usize) as u32;
            let (preview, truncated) = truncate_preview(&text, MAX_PREVIEW_CHARS);

            Ok(Some(DesktopEvent::clipboard_now(preview, char_len, truncated)))
        })();

        unsafe {
            let _ = CloseClipboard();
        }

        out
    }

    fn truncate_preview(s: &str, max_chars: usize) -> (String, bool) {
        let n = s.chars().count();
        if n <= max_chars {
            (s.to_string(), false)
        } else {
            let t: String = s.chars().take(max_chars).collect();
            (t, true)
        }
    }
}
