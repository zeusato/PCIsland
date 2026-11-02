//! Resource counters, handle-based file/empty-directory removal, Recycle Bin and
//! the fixed elevated helper launch used by maintenance.
use std::{
    fs::{File, OpenOptions},
    io,
    os::windows::{
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::{Path, PathBuf},
};
use windows::{
    Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{
            FILE_DISPOSITION_INFO, FileDispositionInfo, GetDiskFreeSpaceExW,
            SetFileInformationByHandle,
        },
        System::{
            ProcessStatus::{
                GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
            },
            SystemInformation::{GetSystemDirectoryW, GlobalMemoryStatusEx, MEMORYSTATUSEX},
            Threading::GetCurrentProcess,
        },
    },
    core::HSTRING,
};

pub struct Resources {
    pub ram_total: u64,
    pub ram_available: u64,
    pub disk_total: u64,
    pub disk_free: u64,
    pub working_set: u64,
    pub private_bytes: u64,
    pub system_cache: u64,
}

/// Injectable boundary; tests never launch Windows tools or sample the host.
pub trait SystemResources {
    fn sample(&self) -> Result<Resources, String>;
    fn open(&self, tool: &str) -> Result<(), String>;
}
pub struct WindowsResources;

pub fn system_directory() -> Result<PathBuf, String> {
    let mut path = [0u16; 32768];
    // SAFETY: the API receives the full writable buffer and returns its length.
    let len = unsafe { GetSystemDirectoryW(Some(&mut path)) } as usize;
    if len == 0 || len >= path.len() {
        return Err("system directory unavailable".into());
    }
    Ok(PathBuf::from(String::from_utf16_lossy(&path[..len])))
}

impl SystemResources for WindowsResources {
    fn sample(&self) -> Result<Resources, String> {
        let system = system_directory()?;
        let mut memory = MEMORYSTATUSEX {
            dwLength: size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };
        let mut process = PROCESS_MEMORY_COUNTERS_EX::default();
        let mut performance =
            windows::Win32::System::ProcessStatus::PERFORMANCE_INFORMATION::default();
        let (mut free, mut total) = (0, 0);
        // SAFETY: all output structs have the documented size; the current-process
        // pseudo handle is valid for this call and must not be closed.
        unsafe {
            GlobalMemoryStatusEx(&raw mut memory).map_err(|e| e.to_string())?;
            GetDiskFreeSpaceExW(
                &HSTRING::from(system.as_os_str()),
                Some(&raw mut free),
                Some(&raw mut total),
                None,
            )
            .map_err(|e| e.to_string())?;
            GetProcessMemoryInfo(
                GetCurrentProcess(),
                (&raw mut process).cast::<PROCESS_MEMORY_COUNTERS>(),
                size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
            )
            .map_err(|e| e.to_string())?;
            windows::Win32::System::ProcessStatus::GetPerformanceInfo(
                &raw mut performance,
                size_of_val(&performance) as u32,
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(Resources {
            ram_total: memory.ullTotalPhys,
            ram_available: memory.ullAvailPhys,
            disk_total: total,
            disk_free: free,
            working_set: process.WorkingSetSize as u64,
            private_bytes: process.PrivateUsage as u64,
            system_cache: performance.SystemCache as u64 * performance.PageSize as u64,
        })
    }
    fn open(&self, tool: &str) -> Result<(), String> {
        let system = system_directory()?;
        match tool {
            "storage" => std::process::Command::new(system.join("..\\explorer.exe"))
                .arg("ms-settings:storagesense")
                .spawn(),
            "task_manager" => std::process::Command::new(system.join("Taskmgr.exe")).spawn(),
            _ => return Err("unsupported system tool".into()),
        }
        .map(|_| ())
        .map_err(|e| e.to_string())
    }
}

/// Holds every ancestor without delete sharing, preventing renames/junction swaps
/// between validation and deletion. Each traversed directory needs its own guard.
pub struct LogDirectory {
    path: PathBuf,
    _anchors: Vec<File>,
}
impl LogDirectory {
    pub fn open(path: &Path) -> io::Result<Self> {
        if !path.is_absolute() {
            return Err(io::Error::other("absolute log directory required"));
        }
        let mut anchors = Vec::new();
        for parent in path.ancestors().collect::<Vec<_>>().into_iter().rev() {
            let file = OpenOptions::new()
                // FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES. Attribute-only
                // handles do not enforce the sharing lock against renames.
                .access_mode(0x81)
                .share_mode(3)
                .custom_flags(0x0220_0000)
                .open(parent)?;
            let meta = file.metadata()?;
            if !meta.is_dir() || meta.file_attributes() & 0x400 != 0 {
                return Err(io::Error::other("linked log directory refused"));
            }
            anchors.push(file);
        }
        Ok(Self {
            path: path.canonicalize()?,
            _anchors: anchors,
        })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn file(&self, path: &Path, deleting: bool) -> io::Result<File> {
        if path.parent() != Some(self.path()) {
            return Err(io::Error::other("file outside log directory"));
        }
        // FILE_READ_ATTRIBUTES, optional DELETE; OPEN_REPARSE_POINT; share read only.
        let file = OpenOptions::new()
            .access_mode(0x80 | if deleting { 0x10000 } else { 0 })
            .share_mode(if deleting { 0 } else { 1 })
            .custom_flags(0x0020_0000)
            .open(path)?;
        let meta = file.metadata()?;
        if !meta.is_file() || meta.file_attributes() & 0x400 != 0 {
            return Err(io::Error::other("linked or non-file log refused"));
        }
        Ok(file)
    }
    /// Opens a direct child directory exclusively with DELETE access. The caller
    /// passes it to [`delete_log`]; Windows refuses the disposition while the
    /// directory still has entries, so a non-empty directory is never removed.
    pub fn directory(&self, path: &Path) -> io::Result<File> {
        if path.parent() != Some(self.path()) {
            return Err(io::Error::other("directory outside guarded parent"));
        }
        // DELETE | FILE_READ_ATTRIBUTES; no sharing; BACKUP_SEMANTICS | OPEN_REPARSE_POINT.
        let file = OpenOptions::new()
            .access_mode(0x1_0080)
            .share_mode(0)
            .custom_flags(0x0220_0000)
            .open(path)?;
        let meta = file.metadata()?;
        if !meta.is_dir() || meta.file_attributes() & 0x400 != 0 {
            return Err(io::Error::other("linked or non-directory entry refused"));
        }
        Ok(file)
    }
}

/// `%ProgramData%` from the shell known-folder database, not the environment.
pub fn program_data_directory() -> Result<PathBuf, String> {
    use windows::Win32::{
        System::Com::CoTaskMemFree,
        UI::Shell::{FOLDERID_ProgramData, KF_FLAG_DEFAULT, SHGetKnownFolderPath},
    };
    // SAFETY: the returned shell allocation is copied and then freed exactly once.
    unsafe {
        let raw = SHGetKnownFolderPath(&FOLDERID_ProgramData, KF_FLAG_DEFAULT, None)
            .map_err(|e| e.to_string())?;
        let path = raw.to_string();
        CoTaskMemFree(Some(raw.0.cast()));
        path.map(PathBuf::from).map_err(|e| e.to_string())
    }
}

/// Size and item count of the Recycle Bin on every drive.
pub fn recycle_bin() -> Result<(u64, u64), String> {
    use windows::Win32::UI::Shell::{SHQUERYRBINFO, SHQueryRecycleBinW};
    let mut info = SHQUERYRBINFO {
        cbSize: size_of::<SHQUERYRBINFO>() as u32,
        i64Size: 0,
        i64NumItems: 0,
    };
    // SAFETY: a null root means all drives; the struct carries its own size.
    unsafe { SHQueryRecycleBinW(windows::core::PCWSTR::null(), &raw mut info) }
        .map_err(|e| e.to_string())?;
    Ok((info.i64Size.max(0) as u64, info.i64NumItems.max(0) as u64))
}

/// Permanently empties every drive's Recycle Bin. The UI confirms first; the shell
/// confirmation, progress and sound are suppressed so the call never blocks on UI.
pub fn empty_recycle_bin() -> Result<(), String> {
    use windows::Win32::UI::Shell::{
        SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND, SHEmptyRecycleBinW,
    };
    // SAFETY: no window handle and a null root (all drives); flags are documented values.
    unsafe {
        SHEmptyRecycleBinW(
            None,
            windows::core::PCWSTR::null(),
            SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND,
        )
    }
    .map_err(|e| e.to_string())
}

/// Starts this executable elevated with one fixed argument and waits for its exit
/// code. `Ok(None)` means the user declined the UAC prompt.
pub fn run_elevated_self(argument: &'static str, timeout_ms: u32) -> Result<Option<u32>, String> {
    use windows::Win32::{
        Foundation::{CloseHandle, ERROR_CANCELLED, WAIT_OBJECT_0},
        System::Threading::{GetExitCodeProcess, WaitForSingleObject},
        UI::{
            Shell::{
                SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
                ShellExecuteExW,
            },
            WindowsAndMessaging::SW_HIDE,
        },
    };
    let exe = HSTRING::from(
        std::env::current_exe()
            .map_err(|e| e.to_string())?
            .as_os_str(),
    );
    let parameters = HSTRING::from(argument);
    let mut info = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI,
        lpVerb: windows::core::w!("runas"),
        lpFile: windows::core::PCWSTR(exe.as_ptr()),
        lpParameters: windows::core::PCWSTR(parameters.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    // SAFETY: the strings outlive the call; the returned process handle is owned
    // here and closed on every path after waiting.
    unsafe {
        if let Err(error) = ShellExecuteExW(&raw mut info) {
            if error.code() == windows::core::HRESULT::from_win32(ERROR_CANCELLED.0) {
                return Ok(None);
            }
            return Err(error.to_string());
        }
        if info.hProcess.is_invalid() {
            return Err("elevated helper handle unavailable".into());
        }
        let process = info.hProcess;
        let result = if WaitForSingleObject(process, timeout_ms) == WAIT_OBJECT_0 {
            let mut code = 0;
            GetExitCodeProcess(process, &raw mut code)
                .map(|_| Some(code))
                .map_err(|e| e.to_string())
        } else {
            Err("elevated helper did not finish in time".into())
        };
        let _ = CloseHandle(process);
        result
    }
}

/// Deletes the already validated, exclusively held file, never a replacement path.
pub fn delete_log(file: File) -> io::Result<()> {
    let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
    // SAFETY: file owns this valid DELETE-access handle until the call completes;
    // the disposition buffer has the matching class and size.
    unsafe {
        SetFileInformationByHandle(
            HANDLE(file.as_raw_handle()),
            FileDispositionInfo,
            (&raw const disposition).cast(),
            size_of::<FILE_DISPOSITION_INFO>() as u32,
        )
    }
    .map_err(io::Error::other)
}

/// All sessions, including processes whose memory we cannot query. Failure is
/// propagated so cache cleanup can skip applications instead of guessing idle.
pub fn running_process_names() -> Result<std::collections::HashSet<String>, String> {
    use windows::Win32::{
        Foundation::{CloseHandle, ERROR_NO_MORE_FILES},
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        },
    };
    let mut names = std::collections::HashSet::new();
    // SAFETY: snapshot is owned here and closed on every path; entry has its
    // required size and remains alive for each enumerator call.
    unsafe {
        let snapshot =
            CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).map_err(|e| e.to_string())?;
        let result = (|| {
            let mut entry = PROCESSENTRY32W {
                dwSize: size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };
            Process32FirstW(snapshot, &raw mut entry)?;
            loop {
                let end = entry
                    .szExeFile
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(entry.szExeFile.len());
                names.insert(String::from_utf16_lossy(&entry.szExeFile[..end]).to_lowercase());
                if let Err(error) = Process32NextW(snapshot, &raw mut entry) {
                    if error.code() != windows::core::HRESULT::from_win32(ERROR_NO_MORE_FILES.0) {
                        return Err(error);
                    }
                    break;
                }
            }
            Ok::<_, windows::core::Error>(())
        })();
        let _ = CloseHandle(snapshot);
        result.map_err(|e| e.to_string())?;
    }
    Ok(names)
}
