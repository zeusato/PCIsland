//! Explicit, per-process working-set reclaim. Never terminates a process or purges standby.
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};
use ts_rs::TS;
use windows::{
    Win32::{
        Foundation::{CloseHandle, FILETIME, HANDLE, HWND, LPARAM},
        Media::Audio::{
            AudioSessionStateActive, DEVICE_STATE_ACTIVE, IAudioSessionControl2,
            IAudioSessionManager2, IMMDeviceEnumerator, MMDeviceEnumerator, eAll,
        },
        System::Com::{
            CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
        },
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
                TH32CS_SNAPPROCESS,
            },
            ProcessStatus::{
                EmptyWorkingSet, GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
                PROCESS_MEMORY_COUNTERS_EX,
            },
            RemoteDesktop::ProcessIdToSessionId,
            Threading::{
                GetCurrentProcessId, GetProcessIoCounters, GetProcessTimes, IO_COUNTERS,
                OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
                PROCESS_SET_QUOTA, QueryFullProcessImageNameW,
            },
        },
        UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId, IsWindowVisible},
    },
    core::{BOOL, Interface, PWSTR},
};

struct Owned(HANDLE);
impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: uniquely owned real handle.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

#[derive(Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct MemoryTarget {
    pub pid: u32,
    pub started: String,
}
#[derive(Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct MemoryProcess {
    pub target: MemoryTarget,
    pub name: String,
    #[ts(type = "number")]
    pub working_set: u64,
    #[ts(type = "number")]
    pub private_bytes: u64,
    pub reason: String,
}
#[derive(Default, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct MemoryReclaim {
    pub targets: Vec<MemoryTarget>,
    pub processed: u32,
    pub skipped: u32,
    #[ts(type = "number")]
    pub working_set_before: u64,
    #[ts(type = "number")]
    pub working_set_after: u64,
    #[ts(type = "number")]
    pub private_before: u64,
    #[ts(type = "number")]
    pub private_after: u64,
}
#[derive(Default, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct MemoryMeasurement {
    pub count: u32,
    #[ts(type = "number")]
    pub working_set: u64,
    #[ts(type = "number")]
    pub private_bytes: u64,
}
pub fn measure(targets: &[MemoryTarget]) -> Result<MemoryMeasurement, String> {
    if targets.len() > 128 {
        return Err("too many process targets".into());
    }
    let samples = samples()?;
    let mut measurement = MemoryMeasurement::default();
    let mut seen = HashSet::new();
    for target in targets {
        if !seen.insert(target.pid) {
            continue;
        }
        if let Some(s) = samples
            .get(&target.pid)
            .filter(|s| s.row.target.started == target.started)
        {
            measurement.count += 1;
            measurement.working_set += s.row.working_set;
            measurement.private_bytes += s.row.private_bytes;
        }
    }
    Ok(measurement)
}
struct Sample {
    row: MemoryProcess,
    cpu: u64,
    io: u64,
    parent: u32,
}
fn same_user(process: HANDLE) -> windows::core::Result<bool> {
    use windows::Win32::{
        Security::{EqualSid, GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser},
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };
    fn token_user(process: HANDLE) -> windows::core::Result<Vec<usize>> {
        let mut token = HANDLE::default();
        // SAFETY: owned query-only token; usize allocation aligns TOKEN_USER and
        // remains live while the SID pointers returned inside it are compared.
        unsafe {
            OpenProcessToken(process, TOKEN_QUERY, &raw mut token)?;
            let token = Owned(token);
            let mut size = 0;
            let _ = GetTokenInformation(token.0, TokenUser, None, 0, &raw mut size);
            let mut buffer = vec![0usize; (size as usize).div_ceil(size_of::<usize>())];
            GetTokenInformation(
                token.0,
                TokenUser,
                Some(buffer.as_mut_ptr().cast()),
                size,
                &raw mut size,
            )?;
            Ok(buffer)
        }
    }
    // SAFETY: the current-process pseudo handle is borrowed, not closed.
    let own = token_user(unsafe { GetCurrentProcess() })?;
    let other = token_user(process)?;
    // SAFETY: successful TokenUser queries initialize both aligned structures.
    unsafe {
        Ok(EqualSid(
            (*(own.as_ptr().cast::<TOKEN_USER>())).User.Sid,
            (*(other.as_ptr().cast::<TOKEN_USER>())).User.Sid,
        )
        .is_ok())
    }
}
fn ticks(t: FILETIME) -> u64 {
    (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime)
}

fn read(handle: HANDLE, pid: u32, parent: u32) -> windows::core::Result<Sample> {
    let (mut start, mut exit, mut kernel, mut user) = (
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
    );
    let mut path = [0u16; 32768];
    let mut len = path.len() as u32;
    let mut memory = PROCESS_MEMORY_COUNTERS_EX::default();
    let mut io = IO_COUNTERS::default();
    // SAFETY: handle stays open, writable buffers/structures match the documented sizes.
    unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(path.as_mut_ptr()),
            &raw mut len,
        )?;
        GetProcessTimes(
            handle,
            &raw mut start,
            &raw mut exit,
            &raw mut kernel,
            &raw mut user,
        )?;
        GetProcessIoCounters(handle, &raw mut io)?;
        GetProcessMemoryInfo(
            handle,
            (&raw mut memory).cast::<PROCESS_MEMORY_COUNTERS>(),
            size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
        )?;
    }
    let path = String::from_utf16_lossy(&path[..len as usize]);
    let name = path.rsplit('\\').next().unwrap_or(&path).to_owned();
    let system = std::env::var("SystemRoot")
        .unwrap_or_else(|_| "C:\\Windows".into())
        .to_lowercase()
        + "\\";
    Ok(Sample {
        row: MemoryProcess {
            target: MemoryTarget {
                pid,
                started: ticks(start).to_string(),
            },
            name,
            working_set: memory.WorkingSetSize as u64,
            private_bytes: memory.PrivateUsage as u64,
            reason: if path.to_lowercase().starts_with(&system) || !same_user(handle)? {
                "system"
            } else {
                "candidate"
            }
            .into(),
        },
        cpu: ticks(kernel) + ticks(user),
        io: io
            .ReadTransferCount
            .saturating_add(io.WriteTransferCount)
            .saturating_add(io.OtherTransferCount),
        parent,
    })
}

fn samples() -> Result<HashMap<u32, Sample>, String> {
    // SAFETY: snapshot and process handles are wrapped in RAII and closed on all paths.
    unsafe {
        let mut session = 0;
        ProcessIdToSessionId(GetCurrentProcessId(), &raw mut session).map_err(|e| e.to_string())?;
        let snapshot =
            Owned(CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).map_err(|e| e.to_string())?);
        let mut entry = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut result = HashMap::new();
        let mut next = Process32FirstW(snapshot.0, &raw mut entry).is_ok();
        while next {
            let mut other = u32::MAX;
            if ProcessIdToSessionId(entry.th32ProcessID, &raw mut other).is_ok() && session == other
            {
                if let Ok(h) = OpenProcess(
                    PROCESS_QUERY_LIMITED_INFORMATION,
                    false,
                    entry.th32ProcessID,
                ) {
                    let handle = Owned(h);
                    if let Ok(sample) =
                        read(handle.0, entry.th32ProcessID, entry.th32ParentProcessID)
                    {
                        result.insert(entry.th32ProcessID, sample);
                    }
                }
            }
            next = Process32NextW(snapshot.0, &raw mut entry).is_ok();
        }
        Ok(result)
    }
}

unsafe extern "system" fn window(hwnd: HWND, param: LPARAM) -> BOOL {
    // SAFETY: EnumWindows calls synchronously with our live HashSet pointer.
    unsafe {
        if IsWindowVisible(hwnd).as_bool() {
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, Some(&raw mut pid));
            (*(param.0 as *mut HashSet<u32>)).insert(pid);
        }
    }
    true.into()
}
fn visible() -> Result<HashSet<u32>, String> {
    let mut pids = HashSet::new();
    // SAFETY: stack set remains alive for the synchronous enumeration.
    unsafe { EnumWindows(Some(window), LPARAM((&raw mut pids) as isize)) }
        .map_err(|e| e.to_string())?;
    Ok(pids)
}
fn audio() -> Result<HashSet<u32>, String> {
    // SAFETY: this runs on a blocking worker; all COM interfaces drop before the
    // matching CoUninitialize. Failure blocks reclamation instead of guessing.
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)
            .ok()
            .map_err(|e| e.to_string())?;
        struct Com;
        impl Drop for Com {
            fn drop(&mut self) {
                // SAFETY: balanced successful initialization on this thread.
                unsafe {
                    CoUninitialize();
                }
            }
        }
        let _com = Com;
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
        let devices = enumerator
            .EnumAudioEndpoints(eAll, DEVICE_STATE_ACTIVE)
            .map_err(|e| e.to_string())?;
        let mut result = HashSet::new();
        for i in 0..devices.GetCount().map_err(|e| e.to_string())? {
            let device = devices.Item(i).map_err(|e| e.to_string())?;
            let manager: IAudioSessionManager2 = device
                .Activate(CLSCTX_ALL, None)
                .map_err(|e| e.to_string())?;
            let sessions = manager.GetSessionEnumerator().map_err(|e| e.to_string())?;
            for j in 0..sessions.GetCount().map_err(|e| e.to_string())? {
                let session = sessions.GetSession(j).map_err(|e| e.to_string())?;
                if session.GetState().map_err(|e| e.to_string())? == AudioSessionStateActive {
                    result.insert(
                        session
                            .cast::<IAudioSessionControl2>()
                            .map_err(|e| e.to_string())?
                            .GetProcessId()
                            .map_err(|e| e.to_string())?,
                    );
                }
            }
        }
        Ok(result)
    }
}

fn root(pid: u32, all: &HashMap<u32, Sample>) -> u32 {
    let mut current = pid;
    let mut seen = HashSet::new();
    while seen.insert(current) {
        let Some(sample) = all.get(&current) else {
            break;
        };
        let Some(parent) = all.get(&sample.parent) else {
            break;
        };
        if parent.row.reason == "system" || parent.row.name.eq_ignore_ascii_case("explorer.exe") {
            break;
        }
        current = sample.parent;
    }
    current
}
fn classify(
    before: Option<&Sample>,
    after: &Sample,
    protected: bool,
    elapsed: Duration,
) -> &'static str {
    if after.row.reason == "system" {
        return "system";
    }
    if protected {
        return "active";
    }
    let Some(before) = before.filter(|b| b.row.target.started == after.row.target.started) else {
        return "changed";
    };
    if after.cpu.saturating_sub(before.cpu) > (elapsed.as_secs_f64() * 100_000.0) as u64
        || after.io != before.io
    {
        return "busy";
    }
    if after.row.working_set < 32 * 1024 * 1024 {
        return "small";
    }
    "candidate"
}
fn scan_internal() -> Result<HashMap<u32, Sample>, String> {
    let before = samples()?;
    let start = Instant::now();
    std::thread::sleep(Duration::from_secs(1));
    let mut after = samples()?;
    let elapsed = start.elapsed();
    let mut protected = visible()?;
    protected.extend(audio()?);
    protected.insert(std::process::id());
    let roots: HashSet<_> = protected.iter().map(|pid| root(*pid, &after)).collect();
    let protected_names: HashSet<_> = after
        .iter()
        .filter(|(pid, _)| roots.contains(&root(**pid, &after)))
        .map(|(_, s)| s.row.name.to_lowercase())
        .collect();
    let reasons: Vec<_> = after
        .iter()
        .map(|(pid, s)| {
            (
                *pid,
                classify(
                    before.get(pid),
                    s,
                    roots.contains(&root(*pid, &after))
                        || protected_names.contains(&s.row.name.to_lowercase()),
                    elapsed,
                ),
            )
        })
        .collect();
    for (pid, reason) in reasons {
        if let Some(s) = after.get_mut(&pid) {
            s.row.reason = reason.into();
        }
    }
    Ok(after)
}
pub fn scan() -> Result<Vec<MemoryProcess>, String> {
    let mut rows: Vec<_> = scan_internal()?.into_values().map(|s| s.row).collect();
    rows.sort_by_key(|s| std::cmp::Reverse(s.working_set));
    Ok(rows)
}
pub fn reclaim(targets: &[MemoryTarget]) -> Result<MemoryReclaim, String> {
    if targets.len() > 128 {
        return Err("too many process targets".into());
    }
    let fresh = scan_internal()?;
    let mut result = MemoryReclaim::default();
    let mut seen = HashSet::new();
    for target in targets {
        if !seen.insert(target.pid) {
            continue;
        }
        let Some(sample) = fresh
            .get(&target.pid)
            .filter(|s| s.row.target.started == target.started && s.row.reason == "candidate")
        else {
            result.skipped += 1;
            continue;
        };
        // Recheck windows/audio just before each operation; never request debug privilege.
        let mut active = visible()?;
        active.extend(audio()?);
        active.insert(std::process::id());
        if active.iter().any(|pid| {
            root(*pid, &fresh) == root(target.pid, &fresh)
                || fresh
                    .get(pid)
                    .is_some_and(|s| s.row.name.eq_ignore_ascii_case(&sample.row.name))
        }) {
            result.skipped += 1;
            continue;
        }
        // SAFETY: minimal rights, identity checked on the same handle used for trim;
        // PID reuse, access denial, exited or recently active processes are skipped.
        unsafe {
            let Ok(h) = OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_QUOTA,
                false,
                target.pid,
            ) else {
                result.skipped += 1;
                continue;
            };
            let handle = Owned(h);
            let Ok(current) = read(handle.0, target.pid, sample.parent) else {
                result.skipped += 1;
                continue;
            };
            if current.row.target.started != target.started
                || current.row.reason == "system"
                || current.cpu != sample.cpu
                || current.io != sample.io
                || EmptyWorkingSet(handle.0).is_err()
            {
                result.skipped += 1;
                continue;
            }
            result.processed += 1;
            result.targets.push(target.clone());
            result.private_before += current.row.private_bytes;
            result.working_set_before += current.row.working_set;
            let after = read(handle.0, target.pid, sample.parent).unwrap_or(current);
            result.working_set_after += after.row.working_set;
            result.private_after += after.row.private_bytes;
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "read-only native smoke; explicitly run on Windows, never trims user processes"]
    fn native_resource_scan() {
        use crate::platform::maintenance::{SystemResources, WindowsResources};
        let snapshot = WindowsResources.sample().unwrap();
        let names = crate::platform::maintenance::running_process_names().unwrap();
        let own_name = std::env::current_exe()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_lowercase();
        assert!(names.contains(&own_name));
        assert!(snapshot.ram_total > 0 && snapshot.ram_available <= snapshot.ram_total);
        assert!(snapshot.disk_total > 0 && snapshot.disk_free <= snapshot.disk_total);
        let rows = scan().unwrap();
        let own = rows
            .iter()
            .find(|r| r.target.pid == std::process::id())
            .unwrap();
        assert_ne!(own.reason, "candidate");
        let measured = measure(std::slice::from_ref(&own.target)).unwrap();
        assert_eq!(measured.count, 1);
        println!(
            "Read-only scan: {} processes, {} eligible; own process protected",
            rows.len(),
            rows.iter().filter(|r| r.reason == "candidate").count()
        );
        // A stale identity must never reach EmptyWorkingSet, even for this test process.
        let result = reclaim(&[MemoryTarget {
            pid: std::process::id(),
            started: "stale".into(),
        }])
        .unwrap();
        assert_eq!((result.processed, result.skipped), (0, 1));
    }
    fn sample() -> Sample {
        Sample {
            row: MemoryProcess {
                target: MemoryTarget {
                    pid: 5,
                    started: "one".into(),
                },
                name: "app.exe".into(),
                working_set: 64 * 1024 * 1024,
                private_bytes: 100,
                reason: "candidate".into(),
            },
            cpu: 100,
            io: 0,
            parent: 0,
        }
    }
    #[test]
    fn candidates_require_quiet_unprotected_same_identity() {
        let before = sample();
        let mut after = sample();
        let elapsed = Duration::from_secs(1);
        assert_eq!(classify(Some(&before), &after, false, elapsed), "candidate");
        assert_eq!(classify(Some(&before), &after, true, elapsed), "active");
        after.io = 1;
        assert_eq!(classify(Some(&before), &after, false, elapsed), "busy");
        after.io = 0;
        after.cpu += 200000;
        assert_eq!(classify(Some(&before), &after, false, elapsed), "busy");
        after.row.target.started = "reused".into();
        assert_eq!(classify(Some(&before), &after, false, elapsed), "changed");
        after.row.reason = "system".into();
        assert_eq!(classify(Some(&before), &after, false, elapsed), "system");
    }
    #[test]
    fn related_children_share_root_but_unrelated_apps_do_not() {
        let mut a = sample();
        a.parent = 1;
        let mut b = sample();
        b.parent = 5;
        let mut shell = sample();
        shell.row.name = "explorer.exe".into();
        shell.row.reason = "system".into();
        let all = HashMap::from([(1, shell), (5, a), (6, b), (9, sample())]);
        assert_eq!(root(6, &all), 5);
        assert_eq!(root(9, &all), 9);
    }
}
