//! Explicit, per-process working-set reclaim and user-confirmed app close. Never purges
//! standby; termination only follows a graceful close the user escalated.
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

/// One user application: every same-user process whose tree root has this executable.
#[derive(Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct MemoryApp {
    pub name: String,
    pub processes: Vec<MemoryTarget>,
    #[ts(type = "number")]
    pub working_set: u64,
    #[ts(type = "number")]
    pub private_bytes: u64,
    /// Visible top-level windows that a graceful close can address.
    pub windows: u32,
    pub audio: bool,
}
/// `status`: `closed` (every process exited), `pending` (asked to close, still running),
/// `no_window` (nothing to ask), `audio` (playing; needs explicit confirmation),
/// `skipped` (no target passed identity checks).
#[derive(Default, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AppCloseResult {
    pub status: String,
    pub closed: u32,
    pub remaining: Vec<MemoryTarget>,
    pub skipped: u32,
}
const MIN_APP_BYTES: u64 = 32 * 1024 * 1024;
const MAX_APPS: usize = 15;
const MAX_CLOSE_TARGETS: usize = 256;

fn app_key(pid: u32, all: &HashMap<u32, Sample>) -> Option<String> {
    all.get(&root(pid, all)).map(|s| s.row.name.to_lowercase())
}
/// Pure grouping, tested with fake samples. Excludes system/other-user processes and
/// the PCisland process tree (including its WebView2 children).
fn group_apps(
    all: &HashMap<u32, Sample>,
    own: u32,
    windows: &HashMap<u32, u32>,
    audio: &HashSet<u32>,
) -> Vec<MemoryApp> {
    let own_root = root(own, all);
    let own_name = all.get(&own).map(|s| s.row.name.to_lowercase());
    let mut apps: HashMap<String, MemoryApp> = HashMap::new();
    for (pid, sample) in all {
        if sample.row.reason == "system" || root(*pid, all) == own_root {
            continue;
        }
        let Some(key) = app_key(*pid, all) else {
            continue;
        };
        if Some(&key) == own_name.as_ref() {
            continue;
        }
        let app = apps.entry(key).or_insert_with(|| MemoryApp {
            name: all[&root(*pid, all)].row.name.clone(),
            processes: Vec::new(),
            working_set: 0,
            private_bytes: 0,
            windows: 0,
            audio: false,
        });
        app.processes.push(sample.row.target.clone());
        app.working_set += sample.row.working_set;
        app.private_bytes += sample.row.private_bytes;
        app.windows += windows.get(pid).copied().unwrap_or_default();
        app.audio |= audio.contains(pid);
    }
    let mut apps: Vec<_> = apps
        .into_values()
        .filter(|a| a.working_set >= MIN_APP_BYTES)
        .collect();
    apps.sort_by_key(|a| std::cmp::Reverse(a.working_set));
    apps.truncate(MAX_APPS);
    for app in &mut apps {
        app.processes.sort_by_key(|t| t.pid);
    }
    apps
}
/// Keeps targets that still match PID + creation time, belong to this user, are not
/// PCisland's tree and share the first valid target's application.
fn verify(targets: &[MemoryTarget], all: &HashMap<u32, Sample>, own: u32) -> Vec<MemoryTarget> {
    let own_root = root(own, all);
    let mut key = None;
    let mut seen = HashSet::new();
    let mut valid = Vec::new();
    for target in targets.iter().take(MAX_CLOSE_TARGETS) {
        if !seen.insert(target.pid) {
            continue;
        }
        let Some(sample) = all
            .get(&target.pid)
            .filter(|s| s.row.target.started == target.started && s.row.reason != "system")
        else {
            continue;
        };
        if root(target.pid, all) == own_root {
            continue;
        }
        let app = app_key(target.pid, all);
        if key.is_none() {
            key = app.clone();
        }
        if app.is_some() && app == key {
            valid.push(sample.row.target.clone());
        }
    }
    valid
}

unsafe extern "system" fn count_window(hwnd: HWND, param: LPARAM) -> BOOL {
    use windows::Win32::UI::WindowsAndMessaging::{GW_OWNER, GetWindow};
    // SAFETY: EnumWindows calls synchronously with our live Vec pointer.
    unsafe {
        let owned = GetWindow(hwnd, GW_OWNER).is_ok_and(|o| !o.is_invalid());
        if IsWindowVisible(hwnd).as_bool() && !owned {
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, Some(&raw mut pid));
            (*(param.0 as *mut Vec<(u32, isize)>)).push((pid, hwnd.0 as isize));
        }
    }
    true.into()
}
/// Visible, unowned top-level windows as (pid, hwnd).
fn top_level_windows() -> Result<Vec<(u32, isize)>, String> {
    let mut list: Vec<(u32, isize)> = Vec::new();
    // SAFETY: stack vector remains alive for the synchronous enumeration.
    unsafe { EnumWindows(Some(count_window), LPARAM((&raw mut list) as isize)) }
        .map_err(|e| e.to_string())?;
    Ok(list)
}

/// Read-only list of the heaviest user applications for the "close app" action.
pub fn apps() -> Result<Vec<MemoryApp>, String> {
    let all = samples()?;
    let mut windows = HashMap::new();
    for (pid, _) in top_level_windows()? {
        *windows.entry(pid).or_insert(0) += 1;
    }
    Ok(group_apps(&all, std::process::id(), &windows, &audio()?))
}

/// Opens each verified target and rechecks its creation time on that same handle.
fn open_verified(
    targets: &[MemoryTarget],
    access: windows::Win32::System::Threading::PROCESS_ACCESS_RIGHTS,
) -> Vec<(MemoryTarget, Owned)> {
    let mut handles = Vec::new();
    for target in targets {
        // SAFETY: handle is owned by `Owned`; GetProcessTimes writes into locals.
        unsafe {
            let Ok(h) = OpenProcess(access, false, target.pid) else {
                continue;
            };
            let handle = Owned(h);
            let (mut start, mut exit, mut kernel, mut user) = Default::default();
            if GetProcessTimes(
                handle.0,
                &raw mut start,
                &raw mut exit,
                &raw mut kernel,
                &raw mut user,
            )
            .is_ok()
                && ticks(start).to_string() == target.started
                && same_user(handle.0).unwrap_or(false)
            {
                handles.push((target.clone(), handle));
            }
        }
    }
    handles
}
/// Waits until every handle signals or the deadline passes; returns the survivors.
fn wait_exit(handles: Vec<(MemoryTarget, Owned)>, timeout: Duration) -> Vec<MemoryTarget> {
    use windows::Win32::{Foundation::WAIT_TIMEOUT, System::Threading::WaitForSingleObject};
    let deadline = Instant::now() + timeout;
    handles
        .into_iter()
        .filter(|(_, handle)| {
            let left = deadline
                .saturating_duration_since(Instant::now())
                .as_millis() as u32;
            // SAFETY: the handle has SYNCHRONIZE access and stays open for the wait.
            unsafe { WaitForSingleObject(handle.0, left) == WAIT_TIMEOUT }
        })
        .map(|(target, _)| target)
        .collect()
}

/// Graceful close: WM_CLOSE to the app's visible top-level windows, then wait up to
/// `timeout`. Never terminates. Apps playing audio need `allow_audio`.
pub fn close(
    targets: &[MemoryTarget],
    allow_audio: bool,
    timeout: Duration,
) -> Result<AppCloseResult, String> {
    use windows::Win32::{
        Foundation::WPARAM,
        System::Threading::PROCESS_SYNCHRONIZE,
        UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE},
    };
    let all = samples()?;
    let valid = verify(targets, &all, std::process::id());
    let mut result = AppCloseResult {
        skipped: (targets.len().min(MAX_CLOSE_TARGETS) - valid.len()) as u32,
        ..Default::default()
    };
    if valid.is_empty() {
        result.status = "skipped".into();
        return Ok(result);
    }
    let pids: HashSet<u32> = valid.iter().map(|t| t.pid).collect();
    if !allow_audio && audio()?.iter().any(|pid| pids.contains(pid)) {
        result.status = "audio".into();
        result.remaining = valid;
        return Ok(result);
    }
    let handles = open_verified(
        &valid,
        PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
    );
    result.skipped += (valid.len() - handles.len()) as u32;
    let windows: Vec<_> = top_level_windows()?
        .into_iter()
        .filter(|(pid, _)| handles.iter().any(|(t, _)| t.pid == *pid))
        .collect();
    if windows.is_empty() {
        result.status = "no_window".into();
        result.remaining = handles.into_iter().map(|(t, _)| t).collect();
        return Ok(result);
    }
    for (_, hwnd) in windows {
        // SAFETY: posting is asynchronous; a window destroyed meanwhile just fails.
        unsafe {
            let _ = PostMessageW(Some(HWND(hwnd as *mut _)), WM_CLOSE, WPARAM(0), LPARAM(0));
        }
    }
    let total = handles.len() as u32;
    result.remaining = wait_exit(handles, timeout);
    result.closed = total - result.remaining.len() as u32;
    result.status = if result.remaining.is_empty() {
        "closed"
    } else {
        "pending"
    }
    .into();
    Ok(result)
}

/// Force-terminates targets. The service only passes targets a graceful close already
/// left running; identity and user are rechecked on the terminate handle itself.
pub fn terminate(targets: &[MemoryTarget], timeout: Duration) -> Result<AppCloseResult, String> {
    use windows::Win32::System::Threading::{
        PROCESS_SYNCHRONIZE, PROCESS_TERMINATE, TerminateProcess,
    };
    let all = samples()?;
    let valid = verify(targets, &all, std::process::id());
    let handles = open_verified(
        &valid,
        PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE | PROCESS_TERMINATE,
    );
    let mut result = AppCloseResult {
        skipped: (targets.len().min(MAX_CLOSE_TARGETS) - handles.len()) as u32,
        ..Default::default()
    };
    let mut terminated = Vec::new();
    for (target, handle) in handles {
        // SAFETY: handle was opened for this verified PID + creation time.
        if unsafe { TerminateProcess(handle.0, 1) }.is_ok() {
            terminated.push((target, handle));
        } else {
            result.skipped += 1;
        }
    }
    let total = terminated.len() as u32;
    result.remaining = wait_exit(terminated, timeout);
    result.closed = total - result.remaining.len() as u32;
    result.status = if result.closed > 0 && result.remaining.is_empty() {
        "closed"
    } else if result.closed > 0 || !result.remaining.is_empty() {
        "pending"
    } else {
        "skipped"
    }
    .into();
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
    fn process(pid: u32, parent: u32, name: &str, mib: u64, reason: &str) -> Sample {
        Sample {
            row: MemoryProcess {
                target: MemoryTarget {
                    pid,
                    started: format!("t{pid}"),
                },
                name: name.into(),
                working_set: mib * 1024 * 1024,
                private_bytes: mib,
                reason: reason.into(),
            },
            cpu: 0,
            io: 0,
            parent,
        }
    }
    fn fake_tree() -> HashMap<u32, Sample> {
        HashMap::from([
            (1, process(1, 0, "explorer.exe", 80, "system")),
            (2, process(2, 0, "svchost.exe", 500, "system")),
            // PCisland and its WebView2 child.
            (10, process(10, 1, "pcisland.exe", 60, "candidate")),
            (11, process(11, 10, "msedgewebview2.exe", 400, "candidate")),
            // Browser with renderer children.
            (20, process(20, 1, "chrome.exe", 200, "candidate")),
            (21, process(21, 20, "chrome.exe", 300, "candidate")),
            (22, process(22, 20, "chrome.exe", 100, "active")),
            // Another app hosting its own WebView2: grouped under its root, not Edge.
            (30, process(30, 1, "Teams.exe", 50, "candidate")),
            (31, process(31, 30, "msedgewebview2.exe", 150, "candidate")),
            (40, process(40, 1, "tiny.exe", 4, "candidate")),
            (50, process(50, 1, "music.exe", 90, "candidate")),
        ])
    }
    #[test]
    fn apps_group_by_tree_root_and_exclude_system_and_pcisland() {
        let all = fake_tree();
        let windows = HashMap::from([(20, 2), (30, 1)]);
        let audio = HashSet::from([50]);
        let apps = group_apps(&all, 10, &windows, &audio);
        let names: Vec<_> = apps.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["chrome.exe", "Teams.exe", "music.exe"]);
        assert_eq!(apps[0].working_set, 600 * 1024 * 1024);
        assert_eq!(apps[0].processes.len(), 3);
        assert_eq!((apps[0].windows, apps[0].audio), (2, false));
        assert_eq!(
            apps[1].processes.iter().map(|t| t.pid).collect::<Vec<_>>(),
            [30, 31]
        );
        assert!(apps[2].audio);
    }
    #[test]
    fn close_targets_require_matching_identity_one_app_and_never_pcisland() {
        let all = fake_tree();
        let target = |pid: u32, started: &str| MemoryTarget {
            pid,
            started: started.into(),
        };
        let valid = verify(
            &[
                target(20, "t20"),
                target(21, "t21"),
                target(21, "t21"),
                target(22, "reused"),
                target(30, "t30"),
                target(2, "t2"),
                target(99, "t99"),
            ],
            &all,
            10,
        );
        assert_eq!(valid.iter().map(|t| t.pid).collect::<Vec<_>>(), [20, 21]);
        assert!(verify(&[target(10, "t10"), target(11, "t11")], &all, 10).is_empty());
        assert!(verify(&[target(2, "t2")], &all, 10).is_empty());
    }
}
