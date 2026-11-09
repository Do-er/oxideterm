use std::{
    os::windows::ffi::OsStrExt as _,
    path::{Path, PathBuf},
    thread,
};

use anyhow::Context as _;
use windows::{
    Win32::{
        Foundation::HWND,
        System::Com::{
            CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
            CoTaskMemFree, CoUninitialize, IPersistFile, STGM_READWRITE,
        },
        UI::{
            Shell::{
                FOLDERID_CommonPrograms, FOLDERID_Desktop, FOLDERID_Programs,
                FOLDERID_PublicDesktop, IShellLinkW, ITaskbarList, KNOWN_FOLDER_FLAG,
                SHCNE_UPDATEITEM, SHCNF_FLUSH, SHCNF_PATH, SHChangeNotify,
                SHGetKnownFolderPath, ShellLink, TaskbarList,
            },
            WindowsAndMessaging::IsWindowVisible,
        },
    },
    core::{GUID, Interface as _, PCWSTR},
};

// Installer shortcuts are created as "<app name>.lnk" (optionally inside a
// subfolder named after the app); keep this aligned with package_native.py.
const SHORTCUT_FILE_NAME: &str = "OxideTerm.lnk";
const START_MENU_SHORTCUT_SUBFOLDER: &str = "OxideTerm";
const PINNED_TASKBAR_SHORTCUT_SUBPATH: &str =
    r"Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar";

// Windows 11 resolves the taskbar button icon from the shortcut associated with
// the installed executable (implicit AppUserModelID) instead of the live window
// icon, so WM_SETICON alone never updates the taskbar for installed builds.
// Rewriting the shortcut's IconLocation to the selected variant changes the
// icon source the shell resolves for that AUMID.
pub(super) fn schedule_shortcut_icon_update(icon_path: &Path, main_hwnd: Option<isize>) {
    // Shortcut rewriting needs an STA COM apartment plus Explorer cache
    // notifications; run on a short-lived worker so settings application on the
    // UI thread never waits on the shell.
    let icon_path = icon_path.to_path_buf();
    let spawned = thread::Builder::new()
        .name("oxideterm-shortcut-icons".to_string())
        .spawn(move || {
            if let Err(error) = rewrite_known_shortcuts(&icon_path) {
                eprintln!("failed to update OxideTerm shortcut icons: {error:#}");
                return;
            }

            // Explorer only re-reads the shortcut icon when the taskbar button is
            // recreated, so a visible window needs its tab rebuilt after the
            // rewrite; a re-shown hidden window recreates the button anyway.
            if let Some(handle) = main_hwnd {
                let hwnd = HWND(handle as _);
                if unsafe { IsWindowVisible(hwnd).as_bool() } {
                    if let Err(error) = refresh_taskbar_tab(hwnd) {
                        eprintln!("failed to refresh the OxideTerm taskbar tab: {error:#}");
                    }
                }
            }
        });
    if let Err(error) = spawned {
        eprintln!("failed to spawn shortcut icon updater: {error}");
    }
}

fn refresh_taskbar_tab(hwnd: HWND) -> anyhow::Result<()> {
    // Reuse this worker thread's COM apartment: ITaskbarList marshals to
    // Explorer, which re-queries the shortcut icon when the tab is recreated.
    let taskbar: ITaskbarList = unsafe { CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER) }
        .context("failed to create taskbar list instance")?;
    unsafe {
        taskbar.DeleteTab(hwnd)?;
        taskbar.AddTab(hwnd)?;
    }
    Ok(())
}

fn rewrite_known_shortcuts(icon_path: &Path) -> anyhow::Result<()> {
    let targets = shortcut_targets();
    if targets.is_empty() {
        return Ok(());
    }

    // Balance CoUninitialize only after a successful initialization: this worker
    // thread owns its apartment for the whole rewrite below.
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
        .ok()
        .context("failed to initialize COM for shortcut rewriting")?;

    let result = rewrite_shortcuts(&targets, icon_path);
    unsafe { CoUninitialize() };
    result
}

fn shortcut_targets() -> Vec<PathBuf> {
    let mut targets = Vec::new();
    for folder in known_folder_paths(&[
        FOLDERID_Programs,
        FOLDERID_CommonPrograms,
        FOLDERID_Desktop,
        FOLDERID_PublicDesktop,
    ]) {
        // The installer nests the Start Menu shortcut in an app subfolder; also
        // probe the bare location so layout changes stay covered.
        for candidate in [
            folder.join(START_MENU_SHORTCUT_SUBFOLDER).join(SHORTCUT_FILE_NAME),
            folder.join(SHORTCUT_FILE_NAME),
        ] {
            if candidate.exists() {
                targets.push(candidate);
            }
        }
    }

    // A pinned taskbar button resolves its icon from the pinned shortcut itself.
    if let Some(roaming) = std::env::var_os("APPDATA") {
        let candidate = PathBuf::from(roaming)
            .join(PINNED_TASKBAR_SHORTCUT_SUBPATH)
            .join(SHORTCUT_FILE_NAME);
        if candidate.exists() {
            targets.push(candidate);
        }
    }

    targets
}

fn known_folder_paths(folders: &[GUID]) -> Vec<PathBuf> {
    folders
        .iter()
        .filter_map(|folder| {
            unsafe { SHGetKnownFolderPath(folder, KNOWN_FOLDER_FLAG(0), None) }
                .ok()
                .map(|path| {
                    // SAFETY: PWSTR borrows the CoTaskMem allocation that is still alive here.
                    let text = unsafe { path.to_string() }.unwrap_or_default();
                    unsafe { CoTaskMemFree(Some(path.as_ptr().cast())) };
                    PathBuf::from(text)
                })
        })
        .collect()
}

fn rewrite_shortcuts(targets: &[PathBuf], icon_path: &Path) -> anyhow::Result<()> {
    // One shell-link instance serves every target; each file is still loaded and
    // saved independently so a single bad shortcut cannot affect the others.
    // SAFETY: CoCreateInstance with a valid coclass; the interface is released by RAII.
    let link: IShellLinkW = unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) }
        .context("failed to create shell link instance")?;
    let persist: IPersistFile = link.cast().context("shell link lacks IPersistFile")?;
    let icon_wide = wide_path(icon_path);

    for target in targets {
        let shortcut_wide = wide_path(target);
        let rewritten: anyhow::Result<()> = unsafe {
            persist
                .Load(PCWSTR(shortcut_wide.as_ptr()), STGM_READWRITE)
                .context("failed to read shortcut")?;
            link.SetIconLocation(PCWSTR(icon_wide.as_ptr()), 0)
                .context("failed to set shortcut icon location")?;
            persist
                .Save(PCWSTR(shortcut_wide.as_ptr()), true)
                .context("failed to save shortcut")?;

            // Ask Explorer to re-resolve the shortcut so the taskbar button icon
            // picks up the new IconLocation without waiting for a shell restart.
            SHChangeNotify(
                SHCNE_UPDATEITEM,
                SHCNF_PATH | SHCNF_FLUSH,
                Some(shortcut_wide.as_ptr().cast()),
                None,
            );
            Ok(())
        };
        // Machine-wide shortcuts may be read-only for a per-user process; skip
        // them rather than aborting the remaining rewrites.
        if let Err(error) = rewritten {
            eprintln!("skipping shortcut icon update for {}: {error:#}", target.display());
        }
    }
    Ok(())
}

fn wide_path(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
