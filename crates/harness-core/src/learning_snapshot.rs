//! 快捷键「学习快照」：截取主显示器 PNG，与前台窗口信息一并写入时间线。

use crate::error::{HarnessError, Result};
use crate::event::DesktopEvent;
use crate::foreground::current_foreground;
use std::fs;

/// 采集主屏截图并生成 [`DesktopEvent::LearningSnapshot`]（不写库；由调用方 `insert_event`）。
pub fn build_learning_snapshot_event() -> Result<DesktopEvent> {
    #[cfg(windows)]
    {
        let fg = current_foreground()?;
        let (image_rel, w, h) = windows_impl::capture_primary_screen_png_file()?;
        Ok(DesktopEvent::learning_snapshot_now(
            image_rel, fg.app, fg.title, w, h,
        ))
    }
    #[cfg(not(windows))]
    {
        Err(HarnessError::UnsupportedPlatform)
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use crate::default_data_dir;
    use image::RgbaImage;
    use std::ffi::c_void;
    use std::path::Path;
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits,
        GetDC, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        HGDIOBJ, SRCCOPY,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

    pub fn capture_primary_screen_png_file() -> Result<(String, u32, u32)> {
        let data_dir = default_data_dir();
        let day = chrono::Local::now().format("%Y-%m-%d").to_string();
        let snap_dir = data_dir.join("snapshots").join(&day);
        fs::create_dir_all(&snap_dir).map_err(HarnessError::Io)?;

        let name = format!("ls-{}.png", chrono::Utc::now().timestamp_millis());
        let abs_path = snap_dir.join(&name);
        let (w, h, rgba) = unsafe {
            let screen_w = GetSystemMetrics(SM_CXSCREEN);
            let screen_h = GetSystemMetrics(SM_CYSCREEN);
            if screen_w <= 0 || screen_h <= 0 {
                return Err(HarnessError::Windows(
                    "GetSystemMetrics 返回无效尺寸".into(),
                ));
            }
            let w = screen_w as u32;
            let h = screen_h as u32;

            let hdc_screen = GetDC(None);
            if hdc_screen.is_invalid() {
                return Err(HarnessError::Windows("GetDC(屏幕) 失败".into()));
            }
            let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
            if hdc_mem.is_invalid() {
                let _ = ReleaseDC(None, hdc_screen);
                return Err(HarnessError::Windows("CreateCompatibleDC 失败".into()));
            }
            let hbmp = CreateCompatibleBitmap(hdc_screen, screen_w, screen_h);
            if hbmp.is_invalid() {
                let _ = DeleteDC(hdc_mem);
                let _ = ReleaseDC(None, hdc_screen);
                return Err(HarnessError::Windows("CreateCompatibleBitmap 失败".into()));
            }
            let old: HGDIOBJ = SelectObject(hdc_mem, hbmp.into());
            BitBlt(
                hdc_mem,
                0,
                0,
                screen_w,
                screen_h,
                Some(hdc_screen),
                0,
                0,
                SRCCOPY,
            )
            .map_err(|e| HarnessError::Windows(format!("BitBlt: {e}")))?;

            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: screen_w,
                    biHeight: -(screen_h),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0 as u32,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [Default::default()],
            };

            let px = (screen_w * screen_h * 4) as usize;
            let mut bgra = vec![0u8; px];
            let lines = GetDIBits(
                hdc_mem,
                hbmp,
                0,
                screen_h as u32,
                Some(bgra.as_mut_ptr() as *mut c_void),
                &mut bmi,
                DIB_RGB_COLORS,
            );
            let _ = SelectObject(hdc_mem, old);
            let _ = DeleteObject(hbmp.into());
            let _ = DeleteDC(hdc_mem);
            let _ = ReleaseDC(None, hdc_screen);

            if lines == 0 {
                return Err(HarnessError::Windows("GetDIBits 失败".into()));
            }

            let mut rgba = Vec::with_capacity(px);
            for chunk in bgra.chunks_exact(4) {
                rgba.push(chunk[2]);
                rgba.push(chunk[1]);
                rgba.push(chunk[0]);
                rgba.push(chunk[3]);
            }
            (w, h, rgba)
        };

        let img =
            RgbaImage::from_raw(w, h, rgba).ok_or_else(|| HarnessError::Windows("图像缓冲无效".into()))?;
        img.save(&abs_path).map_err(|e| HarnessError::Windows(format!("保存 PNG: {e}")))?;

        let rel = normalize_rel_path(
            abs_path
                .strip_prefix(&data_dir)
                .map_err(|_| HarnessError::Windows("截图路径不在数据目录内".into()))?,
        );
        Ok((rel, w, h))
    }

    fn normalize_rel_path(path: &Path) -> String {
        path.iter()
            .map(|p| p.to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }
}
