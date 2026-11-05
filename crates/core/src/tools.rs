//! Catalogo dei tool esterni e installazione esplicita tramite WinGet.

use crate::apply;
use serde::Serialize;
use std::path::Path;

/// Strumenti che servono alle funzioni già presenti nell'app e che pesano poco:
/// l'app propone di installarli. Oh My Posh (circa 6 MB) disegna il prompt informativo.
/// Gli altri del catalogo non hanno ancora una funzione: PowerShell 7 pesa oltre 300 MB e
/// zoxide, fzf, eza, bat, jq, lazygit, tealdeer e Starship arriveranno con le funzioni che li usano.
const RECOMMENDED: &[&str] = &["oh-my-posh"];

/// Elenco degli strumenti installati da CLI Superpower: solo questi si possono disinstallare dall'app.
const STATE_FILE: &str = "installed-tools.json";

struct Tool {
    id: &'static str,
    name: &'static str,
    executables: &'static [&'static str],
    winget_id: Option<&'static str>,
    manual_url: &'static str,
}

// IDs are checked against microsoft/winget-pkgs. Tools with machine-only or
// self-elevating installers use their official manual installation page.
const CATALOG: &[Tool] = &[
    Tool {
        id: "pwsh",
        name: "PowerShell 7",
        executables: &["pwsh.exe"],
        winget_id: Some("Microsoft.PowerShell"),
        manual_url:
            "https://learn.microsoft.com/powershell/scripting/install/install-powershell-on-windows",
    },
    Tool {
        id: "clink",
        name: "Clink",
        executables: &[
            "clink.exe",
            "clink_x64.exe",
            "clink_x86.exe",
            "clink_arm64.exe",
        ],
        winget_id: None,
        manual_url: "https://chrisant996.github.io/clink/clink.html#installing-clink",
    },
    Tool {
        id: "oh-my-posh",
        name: "Oh My Posh",
        executables: &["oh-my-posh.exe"],
        winget_id: Some("JanDeDobbeleer.OhMyPosh"),
        manual_url: "https://ohmyposh.dev/docs/installation/windows",
    },
    Tool {
        id: "starship",
        name: "Starship",
        executables: &["starship.exe"],
        winget_id: Some("Starship.Starship"),
        manual_url: "https://starship.rs/guide/#step-1-install-starship",
    },
    Tool {
        id: "nerd-font",
        name: "Nerd Font",
        executables: &[],
        winget_id: None,
        manual_url: "https://www.nerdfonts.com/font-downloads",
    },
    Tool {
        id: "zoxide",
        name: "zoxide",
        executables: &["zoxide.exe"],
        winget_id: Some("ajeetdsouza.zoxide"),
        manual_url: "https://github.com/ajeetdsouza/zoxide#installation",
    },
    Tool {
        id: "fzf",
        name: "fzf",
        executables: &["fzf.exe"],
        winget_id: Some("junegunn.fzf"),
        manual_url: "https://github.com/junegunn/fzf#installation",
    },
    Tool {
        id: "tealdeer",
        name: "tealdeer",
        executables: &["tldr.exe"],
        winget_id: Some("dbrgn.tealdeer"),
        manual_url: "https://tealdeer-rs.github.io/tealdeer/installing.html",
    },
    Tool {
        id: "eza",
        name: "eza",
        executables: &["eza.exe"],
        winget_id: Some("eza-community.eza"),
        manual_url: "https://github.com/eza-community/eza/blob/main/INSTALL.md",
    },
    Tool {
        id: "bat",
        name: "bat",
        executables: &["bat.exe"],
        winget_id: Some("sharkdp.bat"),
        manual_url: "https://github.com/sharkdp/bat#installation",
    },
    Tool {
        id: "jq",
        name: "jq",
        executables: &["jq.exe"],
        winget_id: Some("jqlang.jq"),
        manual_url: "https://jqlang.org/download/",
    },
    Tool {
        id: "lazygit",
        name: "lazygit",
        executables: &["lazygit.exe"],
        winget_id: Some("JesseDuffield.lazygit"),
        manual_url: "https://github.com/jesseduffield/lazygit#installation",
    },
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub id: &'static str,
    pub name: &'static str,
    pub installed: bool,
    pub version: Option<String>,
    pub manual_url: &'static str,
    pub can_install: bool,
    /// Lo strumento serve a una funzione dell'app ed è proposto in automatico.
    pub recommended: bool,
    /// Installato da CLI Superpower, quindi disinstallabile dall'app.
    pub managed: bool,
    pub can_uninstall: bool,
}

fn managed_ids(local: &Path) -> Vec<String> {
    apply::read(&local.join(STATE_FILE))
        .ok()
        .flatten()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn save_managed(local: &Path, ids: &[String]) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(ids).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    apply::atomic_write(&local.join(STATE_FILE), &bytes)
}

pub fn statuses(local: &Path) -> Vec<ToolStatus> {
    let detected = platform::detect();
    let winget = platform::winget().is_some();
    let managed = managed_ids(local);
    CATALOG
        .iter()
        .map(|tool| {
            let installed = detected.contains_key(tool.id);
            let managed = installed && managed.iter().any(|id| id == tool.id);
            ToolStatus {
                id: tool.id,
                name: tool.name,
                installed,
                version: detected.get(tool.id).cloned().flatten(),
                manual_url: tool.manual_url,
                can_install: winget && tool.winget_id.is_some() && !installed,
                recommended: RECOMMENDED.contains(&tool.id),
                managed,
                can_uninstall: winget && tool.winget_id.is_some() && managed,
            }
        })
        .collect()
}

fn find(id: &str) -> Result<&'static Tool, String> {
    CATALOG
        .iter()
        .find(|tool| tool.id == id)
        .ok_or_else(|| format!("Tool sconosciuto: {id}"))
}

pub fn install(id: &str, local: &Path) -> Result<(), String> {
    let tool = find(id)?;
    let fallback = || format!("Installazione manuale: {}", tool.manual_url);
    let package = tool.winget_id.ok_or_else(fallback)?;
    let winget =
        platform::winget().ok_or_else(|| format!("WinGet non disponibile. {}", fallback()))?;
    // Uno strumento già presente non è nostro: non va registrato né potrà essere disinstallato dall'app.
    if platform::detect().contains_key(tool.id) {
        return Ok(());
    }
    platform::install(&winget, package).map_err(|error| format!("{error}. {}", fallback()))?;
    let mut ids = managed_ids(local);
    if !ids.iter().any(|known| known == id) {
        ids.push(id.to_owned());
        save_managed(local, &ids)?;
    }
    Ok(())
}

/// Disinstalla uno strumento installato da CLI Superpower; quelli che l'utente aveva già restano intatti.
pub fn uninstall(id: &str, local: &Path) -> Result<(), String> {
    let tool = find(id)?;
    let mut ids = managed_ids(local);
    if !ids.iter().any(|known| known == id) {
        return Err(format!(
            "{} non è stato installato da CLI Superpower: disinstallalo da Impostazioni di Windows.",
            tool.name
        ));
    }
    let package = tool
        .winget_id
        .ok_or_else(|| format!("Disinstallazione manuale: {}", tool.manual_url))?;
    let winget = platform::winget().ok_or("WinGet non disponibile.")?;
    platform::uninstall(&winget, package)?;
    ids.retain(|known| known != id);
    save_managed(local, &ids)
}

/// Installa gli strumenti consigliati che mancano. Restituisce i nomi di quelli installati ora.
pub fn install_recommended(local: &Path) -> Result<Vec<&'static str>, String> {
    let detected = platform::detect();
    let mut installed = Vec::new();
    let mut failures = Vec::new();
    for tool in CATALOG
        .iter()
        .filter(|tool| RECOMMENDED.contains(&tool.id) && !detected.contains_key(tool.id))
    {
        match install(tool.id, local) {
            Ok(()) => installed.push(tool.name),
            Err(error) => failures.push(format!("{}: {error}", tool.name)),
        }
    }
    if failures.is_empty() {
        Ok(installed)
    } else {
        Err(failures.join(" "))
    }
}

#[cfg(windows)]
mod platform {
    use super::CATALOG;
    use std::collections::BTreeMap;
    use std::env;
    use std::ffi::OsString;
    use std::os::windows::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    pub(super) fn winget() -> Option<PathBuf> {
        let root = absolute_env("LOCALAPPDATA")?;
        let path = root.join(r"Microsoft\WindowsApps\winget.exe");
        path.is_file().then_some(path)
    }

    pub(super) fn install(winget: &Path, package: &str) -> Result<(), String> {
        let output = Command::new(winget)
            .creation_flags(CREATE_NO_WINDOW)
            .stdin(Stdio::null())
            .args([
                "install",
                "--id",
                package,
                "--exact",
                "--source",
                "winget",
                "--scope",
                "user",
                "--no-upgrade",
                "--disable-interactivity",
            ])
            .output()
            .map_err(|e| format!("Impossibile avviare WinGet: {e}"))?;
        if output.status.success() {
            return Ok(());
        }
        let code = output.status.code().unwrap_or_default() as u32;
        Err(format!(
            "WinGet non ha completato l'installazione (codice 0x{code:08X})"
        ))
    }

    pub(super) fn uninstall(winget: &Path, package: &str) -> Result<(), String> {
        let output = Command::new(winget)
            .creation_flags(CREATE_NO_WINDOW)
            .stdin(Stdio::null())
            .args([
                "uninstall",
                "--id",
                package,
                "--exact",
                "--source",
                "winget",
                "--disable-interactivity",
            ])
            .output()
            .map_err(|e| format!("Impossibile avviare WinGet: {e}"))?;
        if output.status.success() {
            return Ok(());
        }
        let code = output.status.code().unwrap_or_default() as u32;
        Err(format!(
            "WinGet non ha completato la disinstallazione (codice 0x{code:08X})"
        ))
    }

    pub(super) fn detect() -> BTreeMap<String, Option<String>> {
        let paths = search_paths();
        let found = CATALOG.iter().filter_map(|tool| {
            let path = paths.iter().find_map(|path| {
                tool.executables
                    .iter()
                    .map(|exe| path.join(exe))
                    .find(|exe| exe.is_file())
            })?;
            Some((tool.id.to_owned(), path))
        });
        let found: BTreeMap<_, _> = found.collect();
        let mut detected: BTreeMap<_, _> = found.keys().map(|id| (id.clone(), None)).collect();
        if let Some(report) = file_metadata(&found) {
            detected.extend(report);
        }
        detected
    }

    fn search_paths() -> Vec<PathBuf> {
        let mut paths: Vec<_> = env::var_os("PATH")
            .iter()
            .flat_map(env::split_paths)
            .filter(|path| path.is_absolute())
            .collect();
        for root in [
            "ProgramFiles",
            "ProgramFiles(x86)",
            "ProgramW6432",
            "LOCALAPPDATA",
        ] {
            if let Some(root) = absolute_env(root) {
                paths.extend([root.join(r"PowerShell\7"), root.join("clink")]);
            }
        }
        if let Some(root) = absolute_env("LOCALAPPDATA") {
            paths.extend([
                root.join(r"Microsoft\WinGet\Links"),
                root.join(r"Microsoft\WindowsApps"),
                root.join(r"Programs\PowerShell\7"),
                root.join(r"Programs\clink"),
                root.join(r"CliSuperpower\tools\clink"),
            ]);
        }
        paths
    }

    fn absolute_env(key: &str) -> Option<PathBuf> {
        env::var_os(key)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
    }

    fn file_metadata(
        found: &BTreeMap<String, PathBuf>,
    ) -> Option<BTreeMap<String, Option<String>>> {
        let powershell =
            absolute_env("SystemRoot")?.join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
        let mut command = Command::new(powershell);
        command.creation_flags(CREATE_NO_WINDOW);
        // Pass paths through environment data instead of interpolating shell source.
        let mut ids = Vec::with_capacity(found.len());
        for (index, (id, path)) in found.iter().enumerate() {
            ids.push(id.as_str());
            command.env(format!("CSP_TOOL_{index}"), path.as_os_str());
        }
        let ids: OsString = ids.join(",").into();
        let output = command.env("CSP_TOOL_IDS", ids).stdin(Stdio::null()).args([
            "-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
            r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
$report = @{}
if ($env:CSP_TOOL_IDS) {
    $ids = $env:CSP_TOOL_IDS.Split(',')
    for ($i = 0; $i -lt $ids.Length; $i++) {
        try {
            $path = [Environment]::GetEnvironmentVariable('CSP_TOOL_' + $i)
            $info = [Diagnostics.FileVersionInfo]::GetVersionInfo($path)
            $version = $info.ProductVersion
            if (-not $version) { $version = $info.FileVersion }
            if ($version) { $report[$ids[$i]] = [string]$version }
        } catch {}
    }
}
foreach ($hive in @('HKCU:', 'HKLM:')) {
    $key = Get-Item -LiteralPath ($hive + '\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts') -ErrorAction SilentlyContinue
    if ($key -and ($key.GetValueNames() -match 'Nerd Font')) {
        $report['nerd-font'] = $null
        break
    }
}
ConvertTo-Json -InputObject $report -Compress
            "#,
        ]).output().ok()?;
        if !output.status.success() {
            return None;
        }
        serde_json::from_slice(&output.stdout).ok()
    }
}

#[cfg(not(windows))]
mod platform {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    pub(super) fn winget() -> Option<PathBuf> {
        None
    }

    pub(super) fn detect() -> BTreeMap<String, Option<String>> {
        // The catalog includes Windows executable names even when reported from another OS.
        let _ = super::CATALOG
            .iter()
            .map(|tool| tool.executables.len())
            .sum::<usize>();
        BTreeMap::new()
    }

    pub(super) fn install(_winget: &Path, _package: &str) -> Result<(), String> {
        Err("WinGet è disponibile su Windows".into())
    }

    pub(super) fn uninstall(_winget: &Path, _package: &str) -> Result<(), String> {
        Err("WinGet è disponibile su Windows".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_ids_outside_the_catalog_without_running_winget() {
        let local = std::env::temp_dir();
        assert!(install("Microsoft.PowerShell", &local)
            .unwrap_err()
            .contains("sconosciuto"));
        assert!(install("--override", &local)
            .unwrap_err()
            .contains("sconosciuto"));
        assert!(uninstall("--override", &local)
            .unwrap_err()
            .contains("sconosciuto"));
    }

    #[test]
    fn manual_only_tools_return_their_official_link() {
        let local = std::env::temp_dir();
        assert!(install("clink", &local)
            .unwrap_err()
            .contains("https://chrisant996.github.io/clink"));
        assert!(install("nerd-font", &local)
            .unwrap_err()
            .contains("https://www.nerdfonts.com"));
    }

    #[test]
    fn reports_camel_case_fields() {
        let status = ToolStatus {
            id: "pwsh",
            name: "PowerShell 7",
            installed: true,
            version: Some("7.6".into()),
            manual_url: "https://example.test",
            can_install: false,
            recommended: false,
            managed: true,
            can_uninstall: true,
        };
        let status = serde_json::to_value(status).unwrap();
        assert_eq!(status["canInstall"], false);
        assert_eq!(status["manualUrl"], "https://example.test");
        assert_eq!(status["version"], "7.6");
        assert_eq!(status["canUninstall"], true);
    }

    #[test]
    fn only_tools_installed_by_the_app_can_be_uninstalled() {
        let local = std::env::temp_dir().join(format!("csp-tools-test-{}", std::process::id()));
        let error = uninstall("jq", &local).unwrap_err();
        assert!(error.contains("non è stato installato da CLI Superpower"));
        save_managed(&local, &["jq".to_owned()]).unwrap();
        assert_eq!(managed_ids(&local), ["jq"]);
        // Con lo strumento registrato la richiesta arriva a WinGet, che qui non è garantito.
        let error = uninstall("clink", &local).unwrap_err();
        assert!(error.contains("non è stato installato"));
        std::fs::remove_dir_all(local).unwrap();
    }

    #[test]
    fn recommended_tools_exist_in_the_catalog_and_are_light() {
        for id in RECOMMENDED {
            let tool = find(id).unwrap();
            assert!(
                tool.winget_id.is_some(),
                "{id} deve avere un pacchetto WinGet"
            );
        }
        // PowerShell 7 pesa oltre 300 MB: resta una scelta esplicita dell'utente.
        assert!(!RECOMMENDED.contains(&"pwsh"));
    }
}
