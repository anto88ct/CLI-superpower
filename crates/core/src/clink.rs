//! Clink for CMD shortcuts: profile discovery and the bundled copy enabled through AutoRun.
use crate::apply;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const STATE_FILE: &str = "clink-autorun.txt";

#[cfg(target_arch = "aarch64")]
const EXECUTABLE: &str = "clink_arm64.exe";
#[cfg(target_arch = "x86")]
const EXECUTABLE: &str = "clink_x86.exe";
#[cfg(not(any(target_arch = "aarch64", target_arch = "x86")))]
const EXECUTABLE: &str = "clink_x64.exe";

/// Folder where Clink loads scripts in a new CMD window.
pub fn profile_directory(system_local: &Path) -> PathBuf {
    std::env::var_os("CLINK_PROFILE")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            autorun_values()
                .unwrap_or_default()
                .iter()
                .find_map(|value| injection(value, system_local))
                .flatten()
        })
        .unwrap_or_else(|| system_local.join("clink"))
}

/// Enables the bundled Clink unless CMD already injects a Clink at startup.
/// The caller holds the integration lock.
pub fn enable(bundle: &Path, local: &Path, system_local: &Path) -> Result<(), String> {
    if has_injection(system_local)? {
        return Ok(());
    }
    let target = local.join("tools").join("clink");
    copy_tree(bundle, &target)?;
    autorun(&target, "install")?;
    if !has_injection(system_local)? {
        return Err("Clink non risulta attivo dopo l’installazione dell’avvio automatico.".into());
    }
    apply::atomic_write(&local.join(STATE_FILE), target.to_string_lossy().as_bytes())
}

/// Reverts `enable`, leaving AutoRun alone if it no longer starts our copy.
/// The caller holds the integration lock.
pub fn disable(local: &Path) -> Result<(), String> {
    let state = local.join(STATE_FILE);
    let Some(bytes) = apply::read(&state)? else {
        return Ok(());
    };
    let target = PathBuf::from(String::from_utf8_lossy(&bytes).into_owned());
    let ours = target.to_string_lossy().to_lowercase();
    let values = autorun_values().ok_or("Impossibile leggere l’avvio automatico di CMD.")?;
    if values
        .iter()
        .any(|value| value.to_lowercase().contains(&ours))
    {
        autorun(&target, "uninstall")?;
    }
    fs::remove_file(&state).map_err(|error| format!("{}: {error}", state.display()))
}

fn has_injection(system_local: &Path) -> Result<bool, String> {
    let values = autorun_values().ok_or("Impossibile leggere l’avvio automatico di CMD.")?;
    Ok(values
        .iter()
        .any(|value| injection(value, system_local).is_some()))
}

fn autorun(target: &Path, verb: &str) -> Result<(), String> {
    let mut command = Command::new(target.join(EXECUTABLE));
    hide_window(&mut command);
    let output = command
        .args(["autorun", verb])
        .output()
        .map_err(|error| format!("Avvio di Clink: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Clink non ha completato «autorun {verb}»: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

// Unchanged files are skipped, so DLLs loaded by open CMD windows stay untouched.
fn copy_tree(source: &Path, target: &Path) -> Result<(), String> {
    let entries = fs::read_dir(source).map_err(|error| format!("{}: {error}", source.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("{}: {error}", source.display()))?;
        let from = entry.path();
        let to = target.join(entry.file_name());
        if from.is_dir() {
            copy_tree(&from, &to)?;
        } else {
            let bytes = fs::read(&from).map_err(|error| format!("{}: {error}", from.display()))?;
            if apply::read(&to)?.as_deref() != Some(bytes.as_slice()) {
                apply::atomic_write(&to, &bytes)?;
            }
        }
    }
    Ok(())
}

fn hide_window(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    #[cfg(not(windows))]
    let _ = command;
}

/// CMD runs the current user's and the machine's AutoRun values.
fn autorun_values() -> Option<Vec<String>> {
    let powershell = PathBuf::from(std::env::var_os("SystemRoot")?)
        .join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    let mut command = Command::new(powershell);
    hide_window(&mut command);
    let output = command
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            r#"[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false); foreach ($hive in 'HKCU:', 'HKLM:') { (Get-ItemProperty -LiteralPath "$hive\Software\Microsoft\Command Processor" -ErrorAction SilentlyContinue).AutoRun }"#,
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = String::from_utf8(output.stdout).ok()?;
    Some(raw.lines().map(str::to_owned).collect())
}

/// `None` when the AutoRun value does not inject Clink, otherwise its
/// explicit profile, if any.
fn injection(autorun: &str, system_local: &Path) -> Option<Option<PathBuf>> {
    let tokens = command_tokens(autorun);
    let inject = tokens
        .iter()
        .position(|token| token.eq_ignore_ascii_case("inject"))?;
    if !tokens[..inject]
        .iter()
        .any(|token| token.to_ascii_lowercase().contains("clink"))
    {
        return None;
    }
    let value = tokens.iter().enumerate().find_map(|(index, token)| {
        if token == "--profile" || token == "-p" {
            tokens.get(index + 1).map(String::as_str)
        } else {
            token.strip_prefix("--profile=")
        }
    });
    // Clink quirk: in --profile, ~\ means %LOCALAPPDATA%, not the user's home.
    let path = value.map(|value| {
        value
            .strip_prefix(r"~\")
            .map_or_else(|| PathBuf::from(value), |rest| system_local.join(rest))
    });
    Some(path.filter(|path| path.is_absolute()))
}

fn command_tokens(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for ch in line.chars() {
        match ch {
            '"' => quoted = !quoted,
            ch if ch.is_whitespace() && !quoted => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            ch => current.push(ch),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injection_follows_the_clink_profile() {
        let profile = |autorun| injection(autorun, Path::new(r"C:\Users\me\AppData\Local"));
        assert_eq!(
            profile(
                r#""C:\Program Files (x86)\clink\clink.bat" inject --autorun --profile ~\clink"#
            ),
            Some(Some(PathBuf::from(r"C:\Users\me\AppData\Local\clink")))
        );
        assert_eq!(
            profile(r#"clink.bat inject --profile "D:\my tools\clink" --quiet"#),
            Some(Some(PathBuf::from(r"D:\my tools\clink")))
        );
        assert_eq!(
            profile(r"clink.bat inject --profile=D:\clink"),
            Some(Some(PathBuf::from(r"D:\clink")))
        );
        // Clink is present, but its own default profile applies.
        assert_eq!(
            profile(r#""C:\clink\clink.bat" inject --autorun"#),
            Some(None)
        );
        assert_eq!(profile(r"clink.bat inject --profile"), Some(None));
        assert_eq!(
            profile(r"clink.bat inject --profile relative\dir"),
            Some(None)
        );
        // Other AutoRun commands do not start Clink.
        assert_eq!(profile(r"other.bat inject --profile D:\clink"), None);
        assert_eq!(profile(r"clink.bat autorun --profile D:\clink"), None);
        assert_eq!(profile(""), None);
    }

    #[test]
    fn copy_tree_mirrors_the_bundle_and_skips_unchanged_files() {
        let root = std::env::temp_dir().join(format!("csp-clink-test-{}", std::process::id()));
        let source = root.join("bundle");
        let target = root.join("tools/clink");
        apply::atomic_write(&source.join("clink.bat"), b"bat").unwrap();
        apply::atomic_write(&source.join("themes/dark.clinktheme"), b"theme").unwrap();
        copy_tree(&source, &target).unwrap();
        assert_eq!(fs::read(target.join("clink.bat")).unwrap(), b"bat");
        assert_eq!(
            fs::read(target.join("themes/dark.clinktheme")).unwrap(),
            b"theme"
        );

        // A read-only target proves an identical file is not rewritten.
        let batch = target.join("clink.bat");
        let mut permissions = fs::metadata(&batch).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&batch, permissions.clone()).unwrap();
        copy_tree(&source, &target).unwrap();
        apply::atomic_write(&source.join("clink.bat"), b"new").unwrap();
        assert!(copy_tree(&source, &target).is_err());
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        fs::set_permissions(&batch, permissions).unwrap();
        copy_tree(&source, &target).unwrap();
        assert_eq!(fs::read(&batch).unwrap(), b"new");
        fs::remove_dir_all(root).unwrap();
    }
}
