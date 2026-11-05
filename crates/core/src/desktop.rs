//! Plans Windows integration using paths discovered at runtime.
use crate::{
    apply::{self, PlannedFile, PreviewFile},
    clink, config, generators,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone)]
pub struct Desktop {
    data: PathBuf,
    local: PathBuf,
    system_local: PathBuf,
    clink_bundle: Option<PathBuf>,
}

#[derive(Serialize)]
pub struct TeamPreview {
    config: Value,
    changes: Vec<&'static str>,
    files: Vec<PreviewFile>,
}

impl Desktop {
    pub fn upload_background(&self, data: &str) -> Result<PathBuf, String> {
        const MAX_ENCODED_SIZE: usize = (8 * 1024 * 1024_usize).div_ceil(3) * 4;
        if data.len() > MAX_ENCODED_SIZE {
            return Err("L’immagine supera il limite di 8 MB.".to_owned());
        }
        let bytes = STANDARD
            .decode(data)
            .map_err(|_| "Dati immagine non validi.")?;
        if bytes.len() > 8 * 1024 * 1024 {
            return Err("L’immagine supera il limite di 8 MB.".to_owned());
        }
        let (extension, _) =
            background_format(&bytes).ok_or("Scegli un’immagine PNG, JPEG, GIF, BMP o WebP.")?;
        let hash = Sha256::digest(&bytes);
        let path = self
            .local
            .join("backgrounds")
            .join(format!("{hash:x}.{extension}"));
        let _lock = apply::lock(&self.local)?;
        if apply::read(&path)?.as_deref() != Some(bytes.as_slice()) {
            apply::atomic_write(&path, &bytes)?;
        }
        Ok(path)
    }

    pub fn background_preview(&self, path: &Path) -> Result<Option<String>, String> {
        let directory = self.local.join("backgrounds");
        if path.parent() != Some(directory.as_path()) {
            return Ok(None);
        }
        let Some(file) = path.file_stem().and_then(|name| name.to_str()) else {
            return Ok(None);
        };
        if file.len() != 64 || !file.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Ok(None);
        }
        let Some(bytes) = apply::read(path)? else {
            return Ok(None);
        };
        let Some((_, mime)) = background_format(&bytes) else {
            return Ok(None);
        };
        Ok(Some(format!(
            "data:{mime};base64,{}",
            STANDARD.encode(bytes)
        )))
    }
    pub fn new(data: PathBuf, local: PathBuf, system_local: PathBuf) -> Self {
        Self {
            data,
            local,
            system_local,
            clink_bundle: None,
        }
    }

    /// Clink copy shipped with the app, enabled when CMD shortcuts exist.
    pub fn with_clink_bundle(mut self, bundle: PathBuf) -> Self {
        self.clink_bundle = Some(bundle);
        self
    }

    fn sync_clink(&self, effective: &Value) -> Result<(), String> {
        let Some(bundle) = &self.clink_bundle else {
            return Ok(());
        };
        if generators::clink_script_with_theme(effective, Some(&self.prompt_theme()))?.is_some() {
            clink::enable(bundle, &self.local, &self.system_local)
        } else {
            clink::disable(&self.local)
        }
    }

    /// File di configurazione di Oh My Posh generato per il prompt informativo.
    fn prompt_theme(&self) -> PathBuf {
        self.local.join("generated/prompt.omp.json")
    }

    fn preferences(&self) -> PathBuf {
        self.data.join("config.personal.json")
    }

    pub fn load(&self) -> Result<Value, String> {
        match read_json(&self.preferences())? {
            Some(value) => config::migrate(&value),
            None => Ok(json!({"schemaVersion": config::SCHEMA_VERSION})),
        }
    }

    pub fn tool_statuses(&self) -> Vec<crate::tools::ToolStatus> {
        crate::tools::statuses(&self.local)
    }

    pub fn install_tool(&self, id: &str) -> Result<(), String> {
        crate::tools::install(id, &self.local)
    }

    pub fn uninstall_tool(&self, id: &str) -> Result<(), String> {
        crate::tools::uninstall(id, &self.local)
    }

    pub fn install_recommended_tools(&self) -> Result<Vec<&'static str>, String> {
        crate::tools::install_recommended(&self.local)
    }

    pub fn terminal_profiles(&self) -> Result<Vec<String>, String> {
        Ok(self.profiles()?.ids.into_keys().collect())
    }

    fn effective(&self, personal: &Value) -> Result<Value, String> {
        let team = if let Some(team) = personal.get("team") {
            let id = team["packageId"]
                .as_str()
                .filter(|id| {
                    !id.is_empty()
                        && id.bytes().all(|byte| {
                            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
                        })
                })
                .ok_or("ID del pacchetto team non valido")?;
            let version = team["installedVersion"]
                .as_u64()
                .ok_or("Versione del pacchetto team non valida")?;
            let path = self.data.join("team").join(format!("{id}-{version}.json"));
            Some(
                read_json(&path)?
                    .ok_or_else(|| format!("Pacchetto team mancante: {}", path.display()))?,
            )
        } else {
            None
        };
        config::resolve(personal, team.as_ref())
    }

    pub fn save(&self, personal: &Value) -> Result<(), String> {
        let _lock = apply::lock(&self.local)?;
        apply::recover(&self.local)?;
        self.effective(personal)?;
        apply::read(&self.preferences())?;
        apply::atomic_write(
            &self.preferences(),
            &serialize(&config::migrate(personal)?)?,
        )
    }

    pub fn preview_team(&self, personal: &Value, content: &str) -> Result<TeamPreview, String> {
        let (_, config, after) = self.team_candidate(personal, content)?;
        let before = self.effective(personal)?;
        let (files, mut previews) = self.plan(&after)?;
        previews.extend(apply::removals(&self.local, &files)?);
        let changes = [
            "appearance",
            "commandPrefix",
            "commands",
            "workspaces",
            "prompt",
            "tools",
        ]
        .into_iter()
        .filter(|key| before[key] != after[key])
        .collect();
        Ok(TeamPreview {
            config,
            changes,
            files: previews,
        })
    }

    pub fn install_team(&self, personal: &Value, content: &str) -> Result<Value, String> {
        let _lock = apply::lock(&self.local)?;
        apply::recover(&self.local)?;
        let (package, config, effective) = self.team_candidate(personal, content)?;
        self.plan(&effective)?;
        let id = package["id"].as_str().ok_or("ID del pacchetto mancante")?;
        let version = package["version"]
            .as_u64()
            .ok_or("Versione del pacchetto mancante")?;
        let path = self.data.join("team").join(format!("{id}-{version}.json"));
        match read_json(&path)? {
            Some(installed) if installed != package => {
                return Err(
                    "Questa revisione del pacchetto esiste già con contenuto diverso.".to_owned(),
                );
            }
            None => apply::atomic_write(&path, &serialize(&package)?)?,
            Some(_) => {}
        }
        apply::read(&self.preferences())?;
        apply::atomic_write(&self.preferences(), &serialize(&config)?)?;
        Ok(config)
    }

    fn team_candidate(
        &self,
        personal: &Value,
        content: &str,
    ) -> Result<(Value, Value, Value), String> {
        let package = config::parse_json(content.trim_start_matches('\u{feff}'))?;
        let mut personal = config::migrate(personal)?;
        let preferences = personal
            .as_object_mut()
            .ok_or("Configurazione personale non valida")?;
        preferences.insert(
            "team".to_owned(),
            json!({
                "packageId": package["id"],
                "installedVersion": package["version"],
            }),
        );
        let effective = config::resolve(&personal, Some(&package))?;
        Ok((package, personal, effective))
    }

    pub fn preview(&self, personal: &Value) -> Result<Vec<PreviewFile>, String> {
        let (files, mut previews) = self.plan(&self.effective(personal)?)?;
        previews.extend(apply::removals(&self.local, &files)?);
        Ok(previews)
    }

    pub fn apply(&self, personal: &Value) -> Result<Vec<PreviewFile>, String> {
        let _lock = apply::lock(&self.local)?;
        apply::recover(&self.local)?;
        let effective = self.effective(personal)?;
        self.sync_clink(&effective)?;
        let (files, mut previews) = self.plan(&effective)?;
        previews.extend(apply::removals(&self.local, &files)?);
        apply::apply(
            &self.local,
            files,
            Some((self.preferences(), serialize(&config::migrate(personal)?)?)),
        )?;
        Ok(previews)
    }

    pub fn remove(&self) -> Result<(), String> {
        let _lock = apply::lock(&self.local)?;
        apply::recover(&self.local)?;
        apply::apply(&self.local, Vec::new(), None)?;
        clink::disable(&self.local)
    }

    pub fn history(&self) -> Result<Vec<apply::HistoryEntry>, String> {
        let _lock = apply::lock(&self.local)?;
        apply::recover(&self.local)?;
        apply::history(&self.local)
    }

    pub fn restore_latest(&self) -> Result<(), String> {
        let _lock = apply::lock(&self.local)?;
        apply::recover(&self.local)?;
        apply::restore_latest(&self.local)
    }

    pub fn launch(&self, id: &str) -> Result<(), String> {
        let effective = self.effective(&self.load()?)?;
        let profiles = self.profiles()?;
        let args = generators::workspace_args(&effective, id, &profiles.ids)?;
        Command::new("wt.exe")
            .args(args)
            .spawn()
            .map_err(|error| format!("Avvio di Windows Terminal: {error}"))?;
        Ok(())
    }

    fn plan(&self, effective: &Value) -> Result<(Vec<PlannedFile>, Vec<PreviewFile>), String> {
        let mut files = Vec::new();
        let mut previews = Vec::new();
        let prompt = generators::oh_my_posh_config(effective)?;
        let has_prompt = prompt.is_some();
        if let Some(prompt) = prompt {
            files.push(text_file(
                self.local.join("generated/prompt.omp.json"),
                prompt,
                false,
            ));
        }
        let fragment_dir = self
            .system_local
            .join("Microsoft/Windows Terminal/Fragments/CliSuperpower");
        let appearance = effective["appearance"]
            .as_object()
            .is_some_and(|value| !value.is_empty());
        let workspaces = effective["workspaces"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default();
        let default_profile = effective["globalTerminal"]["defaultProfile"].as_str();
        if appearance || !workspaces.is_empty() || default_profile.is_some() {
            let terminal = self.profiles()?;
            if appearance {
                let mut applicable = effective.clone();
                if let Some(entries) = applicable["appearance"].as_object_mut() {
                    entries.retain(|profile, appearance| {
                        if appearance.as_object().is_some_and(|values| values.is_empty()) {
                            return false;
                        }
                        if terminal.ids.contains_key(profile) {
                            return true;
                        }
                        let name = match profile.as_str() {
                            "pwsh" => "PowerShell 7",
                            "powershell" => "Windows PowerShell",
                            _ => "Prompt dei comandi",
                        };
                        previews.push(PreviewFile {
                            path: terminal.path.clone(),
                            content: format!("Profilo {name} non rilevato in Windows Terminal. Le sue preferenze saranno salvate e potranno essere applicate quando il profilo sarà disponibile."),
                        });
                        false
                    });
                }
                if applicable["appearance"]
                    .as_object()
                    .is_some_and(|value| !value.is_empty())
                {
                    let fragment = generators::terminal_fragment(&applicable, &terminal.ids)?;
                    files.push(text_file(
                        fragment_dir.join("profiles.json"),
                        fragment,
                        false,
                    ));
                    for image in generators::terminal_images(&applicable)? {
                        let source = Path::new(&image.source);
                        let bytes = apply::read(source)?
                            .ok_or_else(|| format!("Immagine non trovata: {}", source.display()))?;
                        files.push(PlannedFile {
                            path: fragment_dir.join(image.fragment_file),
                            bytes,
                            preview: format!("Immagine copiata da {}", source.display()),
                            shared_input: None,
                        });
                    }
                    for warning in terminal.conflicts(&applicable["appearance"]) {
                        previews.push(PreviewFile {
                            path: terminal.path.clone(),
                            content: warning,
                        });
                    }
                }
            }
            if let Some(profile) = default_profile {
                let id = terminal
                    .ids
                    .get(profile)
                    .ok_or_else(|| format!("Profilo Windows Terminal non trovato: {profile}"))?;
                let cst =
                    jsonc_parser::cst::CstRootNode::parse(&terminal.source, &Default::default())
                        .map_err(|error| format!("Impostazioni Windows Terminal: {error}"))?;
                let object = cst
                    .object_value()
                    .ok_or("settings.json non contiene un oggetto")?;
                let mut existing = object.properties().into_iter().filter(|property| {
                    property.decoded_name().as_deref() == Some("defaultProfile")
                });
                let property = existing.next();
                if existing.next().is_some() {
                    return Err("settings.json contiene più chiavi defaultProfile.".to_owned());
                }
                let value = jsonc_parser::cst::CstInputValue::String(id.clone());
                if let Some(property) = property {
                    property.set_value(value);
                } else {
                    object.append("defaultProfile", value);
                }
                let mut file = text_file(terminal.path, cst.to_string(), terminal.bom);
                file.shared_input = Some(Some(terminal.bytes));
                files.push(file);
            }
            for workspace in workspaces {
                let id = workspace["id"].as_str().ok_or("Workspace senza ID")?;
                let args = generators::workspace_args(effective, id, &terminal.ids)?;
                previews.push(PreviewFile {
                    path: PathBuf::from(format!("Workspace: {id}")),
                    content: serde_json::to_string_pretty(&json!({
                        "tabs": workspace["tabs"],
                        "windowsTerminalArguments": args,
                    }))
                    .map_err(|error| error.to_string())?,
                });
            }
        }
        if let Some(script) =
            generators::clink_script_with_theme(effective, Some(&self.prompt_theme()))?
        {
            let path = clink::profile_directory(&self.system_local).join("cli-superpower.lua");
            previews.push(PreviewFile {
                path: path.clone(),
                content: if self.clink_bundle.is_some() {
                    "Scorciatoie, autocomplete e prompt saranno disponibili nelle nuove finestre del Prompt dei comandi."
                } else {
                    "Per usare le scorciatoie in CMD, installa Clink con avvio automatico, poi apri una nuova finestra del Prompt dei comandi."
                }
                .to_owned(),
            });
            files.push(text_file(path, script, false));
        }
        for shell in ["pwsh", "powershell"] {
            let prefix = effective["commandPrefix"].as_str().unwrap_or_default();
            let names: Vec<_> = effective["commands"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|command| {
                    command["shells"].as_array().is_some_and(|shells| {
                        shells.iter().any(|value| value.as_str() == Some(shell))
                    })
                })
                .filter_map(|command| command["name"].as_str())
                .map(|name| format!("{prefix}{name}"))
                .collect();
            let behavior =
                generators::has_behavior(effective, shell) || generators::banner_enabled(effective);
            if names.is_empty() && !has_prompt && !behavior {
                continue;
            }
            let Some(ShellProfile {
                path,
                policy,
                collisions,
            }) = shell_profile(shell, &names)?
            else {
                if names.is_empty() {
                    continue;
                }
                return Err(format!(
                    "{shell} non trovato. Installalo oppure rimuovilo dai comandi interessati."
                ));
            };
            let init_name = format!("init-{shell}.ps1");
            let mut init = generators::powershell_init(effective, shell)?;
            if has_prompt {
                init.push_str(OH_MY_POSH_INIT);
            }
            files.push(text_file(
                self.local.join("generated").join(&init_name),
                init,
                shell == "powershell",
            ));
            if matches!(policy.as_str(), "Restricted" | "AllSigned") {
                previews.push(PreviewFile {
                    path: path.clone(),
                    content: format!(
                        "La policy {policy} di {shell} può impedire l’avvio dello script generato. CLI Superpower non modifica la policy."
                    ),
                });
            }
            if !collisions.is_empty() {
                previews.push(PreviewFile {
                    path: path.clone(),
                    content: format!(
                        "Comandi già disponibili in {shell}: {}. Scegli un altro nome per evitare conflitti.",
                        collisions.join(", ")
                    ),
                });
            }
            let before = apply::read(&path)?;
            let bytes = profile_block(before.as_deref(), &init_name)?;
            files.push(PlannedFile {
                path,
                preview: format!("Aggiornamento del blocco CLI Superpower per {shell}.\nLo script {init_name} verrà caricato al prossimo avvio della shell."),
                bytes,
                shared_input: Some(before),
            });
        }
        previews.extend(files.iter().map(|file| PreviewFile {
            path: file.path.clone(),
            content: file.preview.clone(),
        }));
        Ok((files, previews))
    }

    fn profiles(&self) -> Result<TerminalSettings, String> {
        let candidates = [
            "Packages/Microsoft.WindowsTerminal_8wekyb3d8bbwe/LocalState/settings.json",
            "Packages/Microsoft.WindowsTerminalPreview_8wekyb3d8bbwe/LocalState/settings.json",
            "Microsoft/Windows Terminal/settings.json",
        ];
        for candidate in candidates {
            let path = self.system_local.join(candidate);
            if let Some(bytes) = apply::read(&path)? {
                let mut terminal = TerminalSettings::parse(path.clone(), bytes)?;
                if let Ok(Some(defaults)) = apply::read(&path.with_file_name("defaults.json")) {
                    terminal.add_inherited_profiles(&defaults);
                }
                return Ok(terminal);
            }
        }
        Err(
            "Impostazioni Windows Terminal non trovate. Apri Windows Terminal almeno una volta."
                .to_owned(),
        )
    }
}

fn background_format(bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(("png", "image/png"))
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        Some(("jpg", "image/jpeg"))
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some(("gif", "image/gif"))
    } else if bytes.starts_with(b"BM") {
        Some(("bmp", "image/bmp"))
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Some(("webp", "image/webp"))
    } else {
        None
    }
}

fn read_json(path: &Path) -> Result<Option<Value>, String> {
    apply::read(path)?
        .map(|bytes| {
            let text =
                std::str::from_utf8(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
            config::parse_json(text.trim_start_matches('\u{feff}'))
        })
        .transpose()
}

fn serialize(value: &Value) -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn text_file(path: PathBuf, preview: String, bom: bool) -> PlannedFile {
    let mut bytes = if bom {
        vec![0xef, 0xbb, 0xbf]
    } else {
        Vec::new()
    };
    bytes.extend_from_slice(preview.as_bytes());
    PlannedFile {
        path,
        bytes,
        preview,
        shared_input: None,
    }
}

struct TerminalSettings {
    path: PathBuf,
    bytes: Vec<u8>,
    source: String,
    bom: bool,
    ids: BTreeMap<String, String>,
    settings: Value,
}

impl TerminalSettings {
    fn parse(path: PathBuf, bytes: Vec<u8>) -> Result<Self, String> {
        let text = std::str::from_utf8(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        let bom = text.starts_with('\u{feff}');
        let source = text.trim_start_matches('\u{feff}').to_owned();
        let settings: Value = jsonc_parser::parse_to_serde_value(&source, &Default::default())
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if !settings.is_object() {
            return Err("settings.json non contiene un oggetto.".to_owned());
        }
        let profiles = profile_list(&settings);
        let mut ids = BTreeMap::new();
        let mut duplicates = BTreeSet::new();
        for profile in profiles {
            if profile["hidden"].as_bool().unwrap_or(false) {
                continue;
            }
            let Some(id) = profile["guid"].as_str() else {
                continue;
            };
            if let Some(shell) = profile_shell(profile) {
                if ids.insert(shell.to_owned(), id.to_owned()).is_some() {
                    duplicates.insert(shell);
                }
            }
        }
        for duplicate in duplicates {
            ids.remove(duplicate);
        }
        Ok(Self {
            path,
            bytes,
            source,
            bom,
            ids,
            settings,
        })
    }

    fn add_inherited_profiles(&mut self, defaults: &[u8]) {
        let Ok(defaults) = std::str::from_utf8(defaults) else {
            return;
        };
        let Ok(defaults) = jsonc_parser::parse_to_serde_value::<Value>(
            defaults.trim_start_matches('\u{feff}'),
            &Default::default(),
        ) else {
            return;
        };
        for inherited in profile_list(&defaults) {
            let Some(shell) = profile_shell(inherited) else {
                continue;
            };
            let Some(id) = inherited["guid"].as_str() else {
                continue;
            };
            let guid = profile_list(&self.settings).iter().find_map(|profile| {
                let guid = profile["guid"].as_str()?;
                (guid.eq_ignore_ascii_case(id) && profile["hidden"] != true).then_some(guid)
            });
            if let Some(guid) = guid {
                self.ids
                    .entry(shell.to_owned())
                    .or_insert_with(|| guid.to_owned());
            }
        }
    }

    fn conflicts(&self, appearance: &Value) -> Vec<String> {
        let profiles = profile_list(&self.settings);
        let defaults = &self.settings["profiles"]["defaults"];
        self.ids
            .iter()
            .filter_map(|(shell, id)| {
                let edits = appearance[shell].as_object()?;
                let profile = profiles
                    .iter()
                    .find(|profile| profile["guid"].as_str() == Some(id))?;
                let keys: Vec<_> = edits
                    .keys()
                    .filter(|key| {
                        profile.get(key.as_str()).is_some() || defaults.get(key.as_str()).is_some()
                    })
                    .map(String::as_str)
                    .collect();
                (!keys.is_empty()).then(|| {
                    format!(
                "Per {shell}, le impostazioni manuali di Windows Terminal prevalgono su: {}.",
                keys.join(", ")
            )
                })
            })
            .collect()
    }
}

fn profile_list(settings: &Value) -> &[Value] {
    settings["profiles"]
        .as_array()
        .or_else(|| settings["profiles"]["list"].as_array())
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn profile_shell(profile: &Value) -> Option<&'static str> {
    if let Some(source) = profile["source"].as_str() {
        return (source == "Windows.Terminal.PowershellCore").then_some("pwsh");
    }
    let Some(command) = profile["commandline"].as_str() else {
        return match profile["name"].as_str()? {
            "Windows PowerShell" => Some("powershell"),
            "Command Prompt" | "Prompt dei comandi" => Some("cmd"),
            _ => None,
        };
    };
    let command = command.trim_start().to_ascii_lowercase();
    let executable = if let Some(quoted) = command.strip_prefix('"') {
        quoted.split('"').next()?
    } else {
        command.split_ascii_whitespace().next()?
    };
    match executable.rsplit(['/', '\\']).next()? {
        "pwsh" | "pwsh.exe" => Some("pwsh"),
        "powershell" | "powershell.exe" => Some("powershell"),
        "cmd" | "cmd.exe" => Some("cmd"),
        _ => None,
    }
}

const START: &str = "# >>> CLI Superpower >>>";
const END: &str = "# <<< CLI Superpower <<<";
const OH_MY_POSH_INIT: &str = r#"
$__cspTheme = $__cspInit = $null
$__cspOmp = Microsoft.PowerShell.Core\Get-Command -Name oh-my-posh.exe -CommandType Application -ErrorAction SilentlyContinue
if ($__cspOmp) {
    $__cspTheme = [IO.Path]::Combine($PSScriptRoot, 'prompt.omp.json')
    if ([IO.File]::Exists($__cspTheme)) {
        $__cspInit = & $__cspOmp.Source init pwsh --config $__cspTheme --eval
        if ($LASTEXITCODE -eq 0 -and $__cspInit) {
            . ([scriptblock]::Create(($__cspInit -join "`n")))
        }
    }
}
Microsoft.PowerShell.Utility\Remove-Variable -Name __cspOmp,__cspTheme,__cspInit -ErrorAction SilentlyContinue
"#;

struct ShellProfile {
    path: PathBuf,
    policy: String,
    collisions: Vec<String>,
}

fn shell_profile(shell: &str, names: &[String]) -> Result<Option<ShellProfile>, String> {
    let mut command = Command::new(format!("{shell}.exe"));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let mut script = String::from(
        "[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false); $PROFILE.CurrentUserCurrentHost; Get-ExecutionPolicy;",
    );
    for name in names {
        let name = name.replace('\'', "''");
        script.push_str(&format!(
            " if (Get-Command -Name '{name}' -ErrorAction SilentlyContinue) {{ '{name}' }};"
        ));
    }
    let output = match command
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &script,
        ])
        .output()
    {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Avvio di {shell}: {error}")),
    };
    if !output.status.success() {
        return Err(format!("Impossibile rilevare il profilo di {shell}."));
    }
    let raw = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
    let mut lines = raw.lines();
    let path = PathBuf::from(lines.next().unwrap_or_default());
    if !path.is_absolute() || path.extension().and_then(|value| value.to_str()) != Some("ps1") {
        return Err(format!("Percorso del profilo non valido per {shell}."));
    }
    Ok(Some(ShellProfile {
        path,
        policy: lines.next().unwrap_or_default().to_owned(),
        collisions: lines.map(str::to_owned).collect(),
    }))
}

fn profile_block(before: Option<&[u8]>, init: &str) -> Result<Vec<u8>, String> {
    let before = before.unwrap_or_default();
    for (bom, little_endian) in [([0xff, 0xfe], true), ([0xfe, 0xff], false)] {
        if let Some(body) = before.strip_prefix(&bom) {
            let (pairs, remainder) = body.as_chunks::<2>();
            let units: Vec<_> = pairs
                .iter()
                .map(|&bytes| {
                    if little_endian {
                        u16::from_le_bytes(bytes)
                    } else {
                        u16::from_be_bytes(bytes)
                    }
                })
                .collect();
            if !remainder.is_empty() {
                return Err("Profilo PowerShell UTF-16 non valido.".to_owned());
            }
            let text =
                String::from_utf16(&units).map_err(|_| "Profilo PowerShell UTF-16 non valido.")?;
            let updated = update_profile_text(&text, init)?;
            let mut result = bom.to_vec();
            result.extend(updated.encode_utf16().flat_map(|unit| {
                if little_endian {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                }
            }));
            return Ok(result);
        }
    }
    let (bom, body) = if let Some(body) = before.strip_prefix(&[0xef, 0xbb, 0xbf]) {
        (&before[..3], body)
    } else {
        (&before[..0], before)
    };
    let text = std::str::from_utf8(body).map_err(|_| {
        "Profilo PowerShell con codifica non supportata. Nessuna modifica applicata.".to_owned()
    })?;
    let updated = update_profile_text(text, init)?;
    let mut result = bom.to_vec();
    result.extend_from_slice(updated.as_bytes());
    Ok(result)
}

fn update_profile_text(text: &str, init: &str) -> Result<String, String> {
    if text.contains('\0') {
        return Err("Profilo PowerShell con codifica non supportata.".to_owned());
    }
    let newline = if text.contains('\n') && !text.contains("\r\n") {
        "\n"
    } else {
        "\r\n"
    };
    let block = [
        START.to_owned(),
        format!("if (Test-Path -LiteralPath \"$env:LOCALAPPDATA\\CliSuperpower\\generated\\{init}\") {{"),
        format!("    . \"$env:LOCALAPPDATA\\CliSuperpower\\generated\\{init}\""),
        "}".to_owned(),
        END.to_owned(),
    ].join(newline);
    let starts: Vec<_> = text.match_indices(START).collect();
    let ends: Vec<_> = text.match_indices(END).collect();
    let updated = match (starts.as_slice(), ends.as_slice()) {
        ([], []) => {
            let separator = if text.is_empty() || text.ends_with('\n') {
                ""
            } else {
                newline
            };
            format!("{text}{separator}{block}{newline}")
        }
        ([(start, _)], [(end, _)])
            if start < end
                && (*start == 0 || text[..*start].ends_with('\n'))
                && (text.len() == end + END.len()
                    || text[end + END.len()..].starts_with(['\r', '\n'])) =>
        {
            format!("{}{block}{}", &text[..*start], &text[end + END.len()..])
        }
        _ => return Err("Marcatori CLI Superpower ambigui nel profilo PowerShell.".to_owned()),
    };
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_block_preserves_bom_and_user_lines_and_is_idempotent() {
        let before = b"\xef\xbb\xbf# note\r\nWrite-Host 'hello'";
        let result = profile_block(Some(before), "init-powershell.ps1").unwrap();
        assert!(result.starts_with(before));
        assert_eq!(
            profile_block(Some(&result), "init-powershell.ps1").unwrap(),
            result
        );
        assert!(profile_block(Some(START.as_bytes()), "init-powershell.ps1").is_err());
        let utf16: Vec<_> = [0xff, 0xfe]
            .into_iter()
            .chain(
                "Write-Host 'caffè'"
                    .encode_utf16()
                    .flat_map(u16::to_le_bytes),
            )
            .collect();
        let result = profile_block(Some(&utf16), "init-powershell.ps1").unwrap();
        assert!(result.starts_with(&utf16));
        assert_eq!(
            profile_block(Some(&result), "init-powershell.ps1").unwrap(),
            result
        );
    }

    #[test]
    fn terminal_discovery_accepts_jsonc_and_reports_overrides() {
        let fixture = br#"{
            // User preferences
            "profiles": {
                "defaults": { "opacity": 95 },
                "list": [
                    {
                        "guid": "{a433047e-a166-48cb-a410-8ed4b62c5312}",
                        "commandline": "%SystemRoot%\\System32\\cmd.exe",
                    },
                    {
                        "guid": "{1a33047e-a166-48cb-a410-8ed4b62c5312}",
                        "source": "Windows.Terminal.PowershellCore",
                    },
                ],
            },
        }"#;
        let settings =
            TerminalSettings::parse(PathBuf::from("settings.json"), fixture.to_vec()).unwrap();
        assert_eq!(
            settings.ids["cmd"],
            "{a433047e-a166-48cb-a410-8ed4b62c5312}"
        );
        assert_eq!(
            settings.ids["pwsh"],
            "{1a33047e-a166-48cb-a410-8ed4b62c5312}"
        );
        assert_eq!(settings.conflicts(&json!({"cmd":{"opacity":80}})).len(), 1);
    }

    #[test]
    fn preview_apply_and_remove_integrate_without_touching_live_terminal() {
        let root = std::env::temp_dir().join(format!("csp-desktop-test-{}", std::process::id()));
        let terminal = root.join("Microsoft/Windows Terminal");
        let settings = terminal.join("settings.json");
        let original = br#"{
            // This comment must survive.
            "defaultProfile": "old",
            "profiles": {"list": [{
                "guid": "{a433047e-a166-48cb-a410-8ed4b62c5312}",
                "commandline": "cmd.exe",
            }]},
        }"#;
        apply::atomic_write(&settings, original).unwrap();
        let desktop = Desktop::new(root.join("data"), root.join("local"), root.clone());
        let personal = json!({
            "schemaVersion": 1,
            // Il banner di default richiederebbe di scrivere il $PROFILE reale della macchina.
            "banner": {"enabled": false},
            "appearance": {"cmd": {"opacity": 80}},
            "globalTerminal": {"defaultProfile": "cmd"},
        });
        let fragment = terminal.join("Fragments/CliSuperpower/profiles.json");
        let pending_profile = json!({"schemaVersion":1,"banner":{"enabled":false},"appearance":{"pwsh":{"opacity":80}}});
        let pending = desktop.preview(&pending_profile).unwrap();
        assert_eq!(pending.len(), 1);
        assert!(pending[0].content.contains("PowerShell 7 non rilevato"));
        desktop.save(&pending_profile).unwrap();
        // Un documento v1 viene migrato alla versione corrente al salvataggio e al caricamento.
        assert_eq!(
            desktop.load().unwrap(),
            json!({"schemaVersion":6,"banner":{"enabled":false},"appearance":{"pwsh":{"opacity":80}}})
        );
        assert!(!fragment.exists());

        let png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aP3sAAAAASUVORK5CYII=";
        let image_path = desktop.upload_background(png).unwrap();
        assert_eq!(desktop.upload_background(png).unwrap(), image_path);
        assert_eq!(
            desktop.background_preview(&image_path).unwrap(),
            Some(format!("data:image/png;base64,{png}"))
        );
        assert!(desktop.background_preview(&settings).unwrap().is_none());
        assert!(desktop.upload_background("bm90IGFuIGltYWdl").is_err());
        assert_eq!(desktop.preview(&personal).unwrap().len(), 2);
        assert_eq!(std::fs::read(&settings).unwrap(), original);
        assert!(!fragment.exists());
        desktop.apply(&personal).unwrap();
        assert!(fragment.exists());
        let mut migrated = personal.clone();
        migrated["schemaVersion"] = json!(6);
        assert_eq!(desktop.load().unwrap(), migrated);
        let updated = String::from_utf8(std::fs::read(&settings).unwrap()).unwrap();
        assert!(updated.contains("// This comment must survive."));
        assert!(updated.contains(r#""defaultProfile": "{a433047e-a166-48cb-a410-8ed4b62c5312}""#));
        desktop.remove().unwrap();
        assert!(!fragment.exists());
        assert_eq!(std::fs::read(&settings).unwrap(), original);
        assert_eq!(desktop.load().unwrap(), migrated);
        let package =
            r#"{"schemaVersion":1,"id":"sample","version":1,"tools":{"pwsh":{"enabled":true}}}"#;
        let preview = desktop.preview_team(&personal, package).unwrap();
        assert_eq!(preview.changes, ["tools"]);
        assert_eq!(desktop.load().unwrap(), migrated);
        let selected = desktop.install_team(&personal, package).unwrap();
        assert_eq!(selected["team"]["packageId"], "sample");
        assert_eq!(desktop.load().unwrap(), selected);
        assert!(!fragment.exists());
        let replacement = package.replace("true", "false");
        assert!(desktop.install_team(&selected, &replacement).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
