#![cfg(windows)]
#![windows_subsystem = "windows"]
use webview2autoinstall::{check_and_install_webview2, get_webview2_version, WString};
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_OK};

fn main() -> anyhow::Result<()> {
    match check_and_install_webview2(true) {
        Ok(_) => {
            let version = WString::new(&format!(
                "WebView2 Runtime Version: {}",
                get_webview2_version().unwrap()
            ));
            let title = WString::new("WebView2 Runtime Already Installed");
            unsafe {
                MessageBoxW(None, version.as_pcwstr(), title.as_pcwstr(), MB_OK);
            }
            Ok(())
        }
        Err(e) => {
            let error = WString::new(&format!("ERROR: {}", e));
            let title = WString::new("WebView2 Runtime Install Error");
            unsafe {
                MessageBoxW(None, error.as_pcwstr(), title.as_pcwstr(), MB_OK);
            }
            Err(e)
        }
    }
}
