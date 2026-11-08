//! Independent Linux URL handlers while retaining the internal binary name.

use std::path::Path;

fn handler_file_name(scheme: &str) -> Result<String, String> {
    match scheme {
        "yunswitch" | "yunswitch-dev" => Ok(format!("{scheme}-handler.desktop")),
        _ => Err(format!("Unsupported YunSwitch scheme: {scheme}")),
    }
}

// Desktop Entry Exec values are not shell commands. Quote the executable as
// one argument, escape quoted-argument characters, then apply the entry's
// string escaping. Literal percent signs must not become Exec field codes.
fn desktop_exec(executable: &Path) -> Result<String, String> {
    let path = executable
        .to_str()
        .ok_or_else(|| "Deep link executable path is not UTF-8".to_string())?;
    if !executable.is_absolute() || path.chars().any(|c| c.is_control()) {
        return Err(
            "Deep link executable must be an absolute path without control characters".to_string(),
        );
    }
    Ok(format!("{} %u", quote_exec_argument(path)))
}

fn quote_exec_argument(path: &str) -> String {
    let mut argument = String::new();
    for character in path.chars() {
        match character {
            '%' => argument.push_str("%%"),
            '"' | '`' | '$' | '\\' => {
                argument.push('\\');
                argument.push(character);
            }
            _ => argument.push(character),
        }
    }
    format!("\"{}\"", argument.replace('\\', "\\\\"))
}

fn desktop_entry(scheme: &str, executable: &Path) -> Result<String, String> {
    handler_file_name(scheme)?;
    let name = if scheme == "yunswitch-dev" {
        "YunSwitch Dev"
    } else {
        "YunSwitch"
    };
    Ok(format!(
        "[Desktop Entry]\nType=Application\nName={name}\nExec={}\nTerminal=false\nMimeType=x-scheme-handler/{scheme};\nNoDisplay=true\n",
        desktop_exec(executable)?
    ))
}

fn should_set_default(current: &str, own_file: &str) -> bool {
    let current = current.trim();
    // Repair handlers registered by earlier fork builds without touching the
    // upstream ccswitch scheme or a user-selected alternative for our scheme.
    current.is_empty() || current == own_file || current == "cc-switch-handler.desktop"
}

#[cfg(target_os = "linux")]
pub(crate) fn register(app: &tauri::AppHandle) -> Result<(), String> {
    use std::process::Command;
    use tauri::Manager;

    let scheme = crate::config::deep_link_scheme();
    let file_name = handler_file_name(scheme)?;
    let mime_type = format!("x-scheme-handler/{scheme}");
    let executable = match app.env().appimage {
        Some(path) => std::path::PathBuf::from(path),
        None => tauri::utils::platform::current_exe().map_err(|error| error.to_string())?,
    };
    let content = desktop_entry(scheme, &executable)?;
    let applications = app
        .path()
        .data_dir()
        .map_err(|error| error.to_string())?
        .join("applications");
    std::fs::create_dir_all(&applications).map_err(|error| error.to_string())?;
    let target = applications.join(&file_name);
    if std::fs::read_to_string(&target).ok().as_deref() != Some(content.as_str()) {
        crate::config::atomic_write(&target, content.as_bytes())
            .map_err(|error| error.to_string())?;
    }
    let status = Command::new("update-desktop-database")
        .arg(&applications)
        .status()
        .map_err(|error| format!("update-desktop-database failed: {error}"))?;
    if !status.success() {
        return Err(format!("update-desktop-database exited with {status}"));
    }
    let output = Command::new("xdg-mime")
        .args(["query", "default", &mime_type])
        .output()
        .map_err(|error| format!("xdg-mime query failed: {error}"))?;
    if !output.status.success() {
        return Err(format!("xdg-mime query exited with {}", output.status));
    }
    let current = String::from_utf8(output.stdout)
        .map_err(|error| format!("xdg-mime returned invalid UTF-8: {error}"))?;
    if should_set_default(&current, &file_name) {
        let status = Command::new("xdg-mime")
            .args(["default", &file_name, &mime_type])
            .status()
            .map_err(|error| format!("xdg-mime default failed: {error}"))?;
        if !status.success() {
            return Err(format!("xdg-mime default exited with {status}"));
        }
        log::info!("Registered Linux {scheme} handler: {}", target.display());
    } else {
        log::info!(
            "Preserving selected Linux {scheme} handler: {}",
            current.trim()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handlers_and_mime_types_are_separate_for_each_product() {
        let executable = if cfg!(windows) {
            Path::new(r"C:\\Apps\\YunSwitch\\cc-switch.exe")
        } else {
            Path::new("/opt/YunSwitch/cc-switch")
        };
        for scheme in ["yunswitch", "yunswitch-dev"] {
            let entry = desktop_entry(scheme, executable).unwrap();
            assert_eq!(
                handler_file_name(scheme).unwrap(),
                format!("{scheme}-handler.desktop")
            );
            assert!(entry.contains(&format!("MimeType=x-scheme-handler/{scheme};\n")));
            assert!(!entry.contains("x-scheme-handler/ccswitch"));
        }
        assert!(handler_file_name("ccswitch").is_err());
    }

    #[test]
    fn executable_quoting_preserves_literal_field_codes_and_reserved_characters() {
        assert_eq!(
            quote_exec_argument("/opt/My App/50%/a\"b$`\\c"),
            r#""/opt/My App/50%%/a\\"b\\$\\`\\\\c""#,
        );
        #[cfg(not(windows))]
        let (path, expected) = (
            Path::new("/opt/My App/50%/a\"b$`\\c"),
            r#""/opt/My App/50%%/a\\"b\\$\\`\\\\c" %u"#,
        );
        #[cfg(windows)]
        let (path, expected) = (
            Path::new(r"C:\My App\50%\cc-switch.exe"),
            r#""C:\\\\My App\\\\50%%\\\\cc-switch.exe" %u"#,
        );
        assert_eq!(desktop_exec(path).unwrap(), expected);
        assert!(desktop_exec(Path::new("relative-app")).is_err());
        let invalid = if cfg!(windows) {
            "C:\\apps\nInjected=1"
        } else {
            "/apps\nInjected=1"
        };
        assert!(desktop_exec(Path::new(invalid)).is_err());
    }

    #[test]
    fn moved_executable_refreshes_only_own_handler_without_replacing_user_choice() {
        let (old_path, new_path) = if cfg!(windows) {
            (
                Path::new(r"C:\old\cc-switch.exe"),
                Path::new(r"C:\new\cc-switch.exe"),
            )
        } else {
            (Path::new("/old/cc-switch"), Path::new("/new/cc-switch"))
        };
        let old_entry = desktop_entry("yunswitch", old_path).unwrap();
        let new_entry = desktop_entry("yunswitch", new_path).unwrap();
        assert_ne!(old_entry, new_entry);
        assert!(new_entry.contains(&desktop_exec(new_path).unwrap()));
        assert!(should_set_default("", "yunswitch-handler.desktop"));
        assert!(should_set_default(
            "yunswitch-handler.desktop\n",
            "yunswitch-handler.desktop"
        ));
        assert!(should_set_default(
            "cc-switch-handler.desktop\n",
            "yunswitch-handler.desktop"
        ));
        assert!(!should_set_default(
            "custom-handler.desktop",
            "yunswitch-handler.desktop"
        ));
        assert!(!should_set_default(
            "yunswitch-dev-handler.desktop",
            "yunswitch-handler.desktop"
        ));
    }
}
