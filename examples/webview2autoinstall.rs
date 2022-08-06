#![cfg(windows)]
#![windows_subsystem = "windows"]
use webview2autoinstall::{check_and_install_webview2, get_webview2_version, WString};
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_OK};

fn main() -> anyhow::Result<()> {
    match check_and_install_webview2(true) {
        Ok(_) => unsafe {
            MessageBoxW(
                None,
                WString::new(&format!(
                    "WebView2 Runtime Version: {}",
                    get_webview2_version().unwrap()
                ))
                .as_pcwstr(),
                WString::new("WebView2 Runtime Already Installed").as_pcwstr(),
                MB_OK,
            );
            Ok(())
        },
        Err(e) => {
            unsafe {
                MessageBoxW(
                    None,
                    WString::new(&format!("ERROR: {}", e)).as_pcwstr(),
                    WString::new("WebView2 Runtime Install Error").as_pcwstr(),
                    MB_OK,
                );
            }
            Err(e)
        }
    }
}
