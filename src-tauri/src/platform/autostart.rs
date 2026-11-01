//! Per-user startup registration with a quoted executable and verified Run value.
use windows_registry::CURRENT_USER;

const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
const NAME: &str = "PCisland";

pub fn command(exe: &str, data_dir: Option<&str>) -> String {
    let mut value = format!("\"{exe}\" --autostart");
    if let Some(dir) = data_dir {
        // Windows command-line parsing consumes backslashes immediately before a quote.
        let trailing = dir.chars().rev().take_while(|c| *c == '\\').count();
        value.push_str(&format!(" --data-dir \"{dir}{}\"", "\\".repeat(trailing)));
    }
    value
}

pub fn sync(enabled: bool) -> Result<(), String> {
    let key = CURRENT_USER.create(RUN).map_err(|e| e.to_string())?;
    if !enabled {
        if key.get_string(NAME).is_ok() {
            key.remove_value(NAME).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    if !exe.is_file() {
        return Err("Startup executable does not exist".into());
    }
    let args: Vec<String> = std::env::args().collect();
    let data = crate::core::paths::arg_value(&args, "--data-dir");
    let expected = command(&exe.to_string_lossy(), data);
    key.set_string(NAME, &expected).map_err(|e| e.to_string())?;
    CURRENT_USER
        .create(APPROVED)
        .and_then(|key| {
            key.set_bytes(
                NAME,
                windows_registry::Type::Bytes,
                &[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            )
        })
        .map_err(|e| e.to_string())?;
    if key.get_string(NAME).map_err(|e| e.to_string())? != expected {
        return Err("Startup registration verification failed".into());
    }
    Ok(())
}

pub fn registered_command() -> Option<String> {
    CURRENT_USER.open(RUN).ok()?.get_string(NAME).ok()
}

#[cfg(test)]
mod tests {
    #[test]
    fn quotes_executable_and_preserves_custom_data_directory() {
        assert_eq!(
            super::command(r"D:\My Apps\PCisland.exe", Some(r"D:\My Data\")),
            r#""D:\My Apps\PCisland.exe" --autostart --data-dir "D:\My Data\\""#
        );
        assert_eq!(
            super::command(r"D:\app.exe", Some(r"D:\")),
            r#""D:\app.exe" --autostart --data-dir "D:\\""#
        );
    }
}
