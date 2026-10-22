//! App icons. Extraction goes through the Windows shell and exists only on Windows; the
//! on-disk cache and the data-URL encoding are plain Rust so they can be tested anywhere.
use std::path::Path;
use std::time::UNIX_EPOCH;

/// PNG bytes of the shell icon for `target`: a file path, or `shell:AppsFolder\<AUMID>` for
/// Store/MSIX apps. `size` is the requested edge in pixels. `None` when the shell has no icon.
#[cfg(windows)]
pub fn shell_icon_png(target: &str, size: i32) -> Option<Vec<u8>> {
    use std::ffi::c_void;
    use windows::{
        core::HSTRING,
        Win32::{
            Foundation::SIZE,
            Graphics::Gdi::{
                DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO,
                BITMAPINFOHEADER, DIB_RGB_COLORS,
            },
            System::Com::{CoInitializeEx, CoUninitialize, IBindCtx, COINIT_APARTMENTTHREADED},
            UI::Shell::{IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY},
        },
    };
    // The shell wants COM on the calling thread; spawn_blocking threads have none.
    struct Com(bool);
    impl Drop for Com {
        fn drop(&mut self) {
            if self.0 {
                unsafe { CoUninitialize() }
            }
        }
    }
    unsafe {
        let _com = Com(CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok());
        let factory: IShellItemImageFactory =
            SHCreateItemFromParsingName(&HSTRING::from(target), None::<&IBindCtx>).ok()?;
        // SIIGBF_ICONONLY: never ask the thumbnail cache, which can touch the disk.
        let bitmap = factory
            .GetImage(SIZE { cx: size, cy: size }, SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK)
            .ok()?;
        let mut info = BITMAP::default();
        let described = GetObjectW(
            bitmap.into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut info as *mut BITMAP as *mut c_void),
        );
        let (width, height) = (info.bmWidth, info.bmHeight);
        let png = if described == 0 || width <= 0 || height <= 0 {
            None
        } else {
            // Ask for a top-down 32-bit copy so the pixel layout does not depend on how the
            // shell built the bitmap.
            let mut header = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: 0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];
            let screen = GetDC(None);
            let copied = GetDIBits(
                screen,
                bitmap,
                0,
                height as u32,
                Some(pixels.as_mut_ptr() as *mut c_void),
                &mut header,
                DIB_RGB_COLORS,
            );
            ReleaseDC(None, screen);
            if copied == 0 {
                None
            } else {
                for pixel in pixels.chunks_exact_mut(4) {
                    pixel.swap(0, 2); // BGRA -> RGBA
                }
                encode_png(&pixels, width as u32, height as u32)
            }
        };
        let _ = DeleteObject(bitmap.into());
        png
    }
}

#[cfg(not(windows))]
pub fn shell_icon_png(_target: &str, _size: i32) -> Option<Vec<u8>> {
    None
}

#[cfg(windows)]
fn encode_png(rgba: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().ok()?.write_image_data(rgba).ok()?;
    Some(out)
}

fn fnv1a(text: &str) -> u64 {
    text.bytes()
        .fold(0xcbf2_9ce4_8422_2325, |hash, byte| (hash ^ byte as u64).wrapping_mul(0x0100_0000_01b3))
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let group = (chunk[0] as u32) << 16
            | (*chunk.get(1).unwrap_or(&0) as u32) << 8
            | *chunk.get(2).unwrap_or(&0) as u32;
        for position in 0..4 {
            if position <= chunk.len() {
                out.push(ALPHABET[(group >> (18 - 6 * position) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn data_url(png: &[u8]) -> String {
    format!("data:image/png;base64,{}", base64(png))
}

/// The icon for `target` as a PNG data URL. Extracted icons are cached under `cache_dir`, keyed
/// by the path, the requested size and the file's modification time, so an updated app gets a
/// fresh icon and an unchanged one is never extracted twice. Store apps have no file to stat,
/// so their key carries no timestamp.
pub fn cached_icon(
    target: &Path,
    cache_dir: Option<&Path>,
    size: i32,
    extract: impl Fn(&str, i32) -> Option<Vec<u8>>,
) -> Option<String> {
    let text = target.to_string_lossy();
    let modified = std::fs::metadata(target)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |elapsed| elapsed.as_secs());
    let file = cache_dir.map(|dir| dir.join(format!("{:016x}-{size}-{modified}.png", fnv1a(&text.to_lowercase()))));
    if let Some(png) = file.as_ref().and_then(|file| std::fs::read(file).ok()).filter(|png| !png.is_empty()) {
        return Some(data_url(&png));
    }
    let png = extract(&text, size).filter(|png| !png.is_empty())?;
    if let Some(file) = &file {
        if let Some(dir) = file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(file, &png);
    }
    Some(data_url(&png))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn base64_matches_the_rfc_4648_vectors() {
        for (input, expected) in [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foob", "Zm9vYg=="), ("fooba", "Zm9vYmE="), ("foobar", "Zm9vYmFy")] {
            assert_eq!(base64(input.as_bytes()), expected);
        }
        assert_eq!(base64(&[0xff, 0xfe, 0xfd, 0x00]), "//79AA==");
    }

    #[test]
    fn icons_are_extracted_once_and_refreshed_when_the_file_changes() {
        let temp = std::env::temp_dir().join(format!("lumen-icons-{}", std::process::id()));
        let cache = temp.join("cache");
        std::fs::create_dir_all(&temp).unwrap();
        let app = temp.join("app.exe");
        std::fs::write(&app, "v1").unwrap();
        let calls = Cell::new(0);
        let extract = |_: &str, size: i32| {
            calls.set(calls.get() + 1);
            Some(vec![size as u8, 1, 2, 3])
        };
        let first = cached_icon(&app, Some(&cache), 64, &extract).unwrap();
        assert!(first.starts_with("data:image/png;base64,"));
        assert_eq!(cached_icon(&app, Some(&cache), 64, &extract).unwrap(), first);
        assert_eq!(calls.get(), 1, "second lookup must come from the cache");
        // Another size is a different icon.
        cached_icon(&app, Some(&cache), 32, &extract).unwrap();
        assert_eq!(calls.get(), 2);
        // A newer file (different modification time) invalidates the entry.
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(120);
        std::fs::File::options().write(true).open(&app).unwrap().set_modified(later).unwrap();
        cached_icon(&app, Some(&cache), 64, &extract).unwrap();
        assert_eq!(calls.get(), 3);
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn missing_icons_are_not_cached_and_store_apps_need_no_file() {
        let temp = std::env::temp_dir().join(format!("lumen-icons-none-{}", std::process::id()));
        let none = |_: &str, _: i32| None;
        let store = Path::new("shell:AppsFolder\\Pkg_x!App");
        assert!(cached_icon(store, Some(&temp), 64, none).is_none());
        assert!(!temp.exists(), "a failed extraction must not leave cache files");
        let calls = Cell::new(0);
        let some = |text: &str, _: i32| {
            calls.set(calls.get() + 1);
            assert_eq!(text, "shell:AppsFolder\\Pkg_x!App");
            Some(vec![9, 9])
        };
        assert!(cached_icon(store, Some(&temp), 64, &some).is_some());
        assert!(cached_icon(store, Some(&temp), 64, &some).is_some());
        assert_eq!(calls.get(), 1);
        // Without a cache directory it still works, it just extracts every time.
        assert!(cached_icon(store, None, 64, &some).is_some());
        assert_eq!(calls.get(), 2);
        std::fs::remove_dir_all(temp).unwrap();
    }
}
