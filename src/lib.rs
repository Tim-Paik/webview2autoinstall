#![cfg(windows)]
use anyhow::{anyhow, Context, Result};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{GetLastError, HANDLE, HINSTANCE, HWND, WAIT_FAILED, WAIT_TIMEOUT},
        Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
        System::{
            Com::{CoInitializeEx, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE},
            Registry::HKEY,
            Threading::{
                GetCurrentProcess, OpenProcessToken, WaitForSingleObject, WAIT_ABANDONED,
                WAIT_OBJECT_0,
            },
        },
        UI::{
            Shell::{
                ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, SHELLEXECUTEINFOW_0,
            },
            WindowsAndMessaging::{MessageBoxW, IDCANCEL, IDOK, MB_OKCANCEL, SW_NORMAL},
        },
    },
};

#[derive(Default)]
pub struct WString(Option<Vec<u16>>);

impl WString {
    pub fn from_opt_str(s: Option<&str>) -> Self {
        Self(s.map(|s| s.encode_utf16().chain(std::iter::once(0)).collect()))
    }
    pub fn new(s: &str) -> Self {
        Self::from_opt_str(Some(s))
    }
    pub fn as_ptr(&self) -> *const u16 {
        self.0.as_ref().map_or(std::ptr::null(), |s| s.as_ptr())
    }
    pub fn as_pcwstr(&self) -> PCWSTR {
        PCWSTR(self.as_ptr())
    }
}

impl std::fmt::Debug for WString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self.0.clone().map_or(Vec::new(), |s| s);
        let s = String::from_utf16(&s).map_err(|_| std::fmt::Error)?;
        f.debug_tuple("WString").field(&s).finish()
    }
}

pub fn is_elevated() -> bool {
    let mut tokenhandle = HANDLE::default();
    let result = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut tokenhandle) };
    if result != false {
        let mut tokeninformation = TOKEN_ELEVATION::default();
        let tokeninformation_ptr: *mut TOKEN_ELEVATION = &mut tokeninformation;
        let result = unsafe {
            GetTokenInformation(
                tokenhandle,
                TokenElevation,
                tokeninformation_ptr as *mut std::ffi::c_void,
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut 0u32,
            )
        };
        if result != false {
            return tokeninformation.TokenIsElevated != 0;
        }
    }
    false
}

pub fn get_webview2_version() -> Option<String> {
    let mut versioninfo = windows::core::PWSTR::default();
    match unsafe {
        webview2_com::Microsoft::Web::WebView2::Win32::GetAvailableCoreWebView2BrowserVersionString(
            windows::core::PCWSTR::default(),
            &mut versioninfo,
        )
    } {
        Ok(_) => Some(webview2_com::take_pwstr(versioninfo)),
        Err(_) => None,
    }
}

pub fn install_webview2(as_admin: bool) -> Result<()> {
    // see https://developer.microsoft.com/microsoft-edge/webview2#download-section
    let res = minreq::get("https://go.microsoft.com/fwlink/p/?LinkId=2124703")
        .send()
        .unwrap();
    let mut target =
        std::path::Path::new(&std::env::var("TEMP").unwrap_or_else(|_| "./".to_string()))
            .canonicalize()
            .with_context(|| "error parsing path, %TEMP% or ./ does not exist")?;
    target.push("MicrosoftEdgeWebview2Setup.exe");
    std::fs::write(&target, res.as_bytes()).with_context(|| {
        format!(
            "error writing to file: cannot write to {}",
            target.display()
        )
    })?;
    let is_elevated = is_elevated();
    if !as_admin || is_elevated {
        let output = std::process::Command::new(target)
            .arg("/install")
            .output()
            .with_context(|| "error run intaller")?;
        if output.status.success() {
            Ok(())
        } else {
            dbg!(get_webview2_version());
            Err(anyhow!(
                "error run process\nprocess exited with code: {}\nstdout: {}\nstderr: {}",
                output.status.code().unwrap_or(1),
                std::str::from_utf8(&output.stdout).unwrap_or_default(),
                std::str::from_utf8(&output.stderr).unwrap_or_default()
            ))
        }
    } else {
        unsafe {
            CoInitializeEx(
                std::ptr::null::<std::ffi::c_void>(),
                COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE,
            )
            .with_context(|| "error in initializing COM component")?;
        }
        const WAIT_TIMEOUT_VALUE: u32 = WAIT_TIMEOUT.0;
        const WAIT_FAILED_VALUE: u32 = WAIT_FAILED.0;
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            hwnd: HWND::default(),
            lpVerb: WString::new("runas").as_pcwstr(),
            lpFile: WString::from_opt_str(target.to_str()).as_pcwstr(),
            lpParameters: WString::new("/install").as_pcwstr(),
            lpDirectory: PCWSTR(std::ptr::null()),
            nShow: SW_NORMAL.0 as i32,
            hInstApp: HINSTANCE::default(),
            lpIDList: std::ptr::null_mut::<std::ffi::c_void>(),
            lpClass: PCWSTR(std::ptr::null()),
            hkeyClass: HKEY::default(),
            dwHotKey: 0,
            Anonymous: SHELLEXECUTEINFOW_0::default(),
            hProcess: HANDLE::default(),
        };
        match unsafe {
            ShellExecuteExW(&mut info);
            WaitForSingleObject(info.hProcess, 600000 /* 10mins */)
        } {
            WAIT_ABANDONED => Err(anyhow!(
                "error in ShellExecuteExW: the function has abandoned"
            )),
            WAIT_OBJECT_0 => Ok(()),
            WAIT_TIMEOUT_VALUE => Err(anyhow!(
                "error in ShellExecuteExW: timeout, 10mins Not responding",
            )),
            WAIT_FAILED_VALUE => Err(anyhow!(
                "error in ShellExecuteExW: the function has failed with {:?}",
                unsafe { GetLastError() }
            )),
            _ => Err(anyhow!("unknown error in ShellExecuteExW")),
        }
    }
}

pub fn check_and_install_webview2(try_as_admin: bool) -> Result<()> {
    if get_webview2_version().is_some() {
        return Ok(());
    }

    match unsafe {
        MessageBoxW(
            None,
            WString::new("WebView2 Runtime is not installed. Install now?\n(Click Cancel to exit)")
                .as_pcwstr(),
            PCWSTR(WString::new("Require WebView2 Runtime").as_ptr()),
            MB_OKCANCEL,
        )
    } {
        IDOK => install_webview2(try_as_admin),
        IDCANCEL => Err(anyhow!("user canceled webview2 installation")),
        _ => Err(anyhow!("unknown operation in MessageBoxW")),
    }
}
