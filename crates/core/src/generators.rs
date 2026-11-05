//! Generatori puri. I documenti sono configurazioni effettive prodotte da config::resolve.
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};
use std::collections::BTreeMap;

mod banner;
mod behavior;
mod clink;
mod powershell;
mod prompt;
pub use banner::banner_enabled;
pub use behavior::has_behavior;
pub use clink::{clink_script, clink_script_with_theme};
pub use powershell::powershell_init;
pub use prompt::oh_my_posh_config;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalImage {
    pub source: String,
    pub fragment_file: String,
}

fn image(profile: &str, source: &str) -> TerminalImage {
    let extension = source
        .rsplit(['/', '\\'])
        .next()
        .and_then(|file| file.rsplit_once('.'))
        .map(|(_, extension)| extension)
        .filter(|extension| {
            (1..=10).contains(&extension.len())
                && extension.bytes().all(|b| b.is_ascii_alphanumeric())
        })
        .unwrap_or("img");
    TerminalImage {
        source: source.into(),
        fragment_file: format!("background-{profile}.{}", extension.to_ascii_lowercase()),
    }
}

/// Elenca le copie da collocare accanto al frammento prima di pubblicarlo.
pub fn terminal_images(config: &Value) -> Result<Vec<TerminalImage>, String> {
    Ok(config["appearance"]
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(profile, appearance)| {
            appearance["backgroundImage"]
                .as_str()
                .map(|source| image(profile, source))
        })
        .collect())
}

fn profile_id<'a>(ids: &'a BTreeMap<String, String>, profile: &str) -> Result<&'a str, String> {
    ids.get(profile)
        .map(String::as_str)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| format!("Profilo Windows Terminal non rilevato: {profile}"))
}

pub fn terminal_fragment(
    config: &Value,
    profile_ids: &BTreeMap<String, String>,
) -> Result<String, String> {
    let mut updates = Vec::new();
    for (profile, appearance) in config["appearance"].as_object().into_iter().flatten() {
        let Some(mut appearance) = appearance.as_object().cloned() else {
            return Err(format!("Aspetto non valido: {profile}"));
        };
        appearance.retain(|_, value| !value.as_object().is_some_and(|obj| obj.is_empty()));
        if appearance.is_empty() {
            continue;
        }
        appearance.insert("updates".into(), json!(profile_id(profile_ids, profile)?));
        if let Some(source) = appearance.get("backgroundImage").and_then(Value::as_str) {
            appearance.insert(
                "backgroundImage".into(),
                json!(image(profile, source).fragment_file),
            );
        }
        if let Some(padding) = appearance.get("padding") {
            // Terminal serializza tutti i lati in una proprietà atomica. Il default è 8.
            let values = ["left", "top", "right", "bottom"]
                .map(|side| padding[side].as_u64().unwrap_or(8).to_string());
            appearance.insert("padding".into(), json!(values.join(", ")));
        }
        updates.push(Value::Object(appearance));
    }
    serde_json::to_string_pretty(&json!({"profiles": updates}))
        .map(|output| output + "\n")
        .map_err(|error| error.to_string())
}

pub fn workspace_args(
    config: &Value,
    workspace_id: &str,
    profile_ids: &BTreeMap<String, String>,
) -> Result<Vec<String>, String> {
    let workspace = entries(&config["workspaces"])
        .find(|workspace| workspace["id"] == workspace_id)
        .ok_or_else(|| format!("Workspace non trovato: {workspace_id}"))?;
    let mut args = vec!["--window".into(), "new".into()];
    for (index, tab) in entries(&workspace["tabs"]).enumerate() {
        if index > 0 {
            args.push(";".into());
        }
        args.push("new-tab".into());
        if let Some(title) = tab["title"].as_str().filter(|title| !title.is_empty()) {
            args.extend([
                "--title".into(),
                wt_value(title)?,
                "--suppressApplicationTitle".into(),
            ]);
        }
        append_pane(&mut args, &tab["pane"], profile_ids)?;
        for split in entries(&tab["splits"]) {
            let orientation = match split["direction"].as_str() {
                Some("right") => "--vertical",
                Some("down") => "--horizontal",
                _ => return Err("Direzione di split non valida".into()),
            };
            args.extend([";".into(), "split-pane".into(), orientation.into()]);
            append_pane(&mut args, &split["pane"], profile_ids)?;
        }
    }
    Ok(args)
}

fn append_pane(
    args: &mut Vec<String>,
    pane: &Value,
    profile_ids: &BTreeMap<String, String>,
) -> Result<(), String> {
    let profile = string(pane, "profile")?;
    args.extend([
        "--profile".into(),
        wt_value(profile_id(profile_ids, profile)?)?,
    ]);
    if let Some(directory) = pane["workingDirectory"].as_str() {
        args.extend(["--startingDirectory".into(), wt_value(directory)?]);
    }
    if let Some(startup) = pane["startupCommand"].as_str() {
        match profile {
            "pwsh" | "powershell" => {
                let encoded = STANDARD.encode(
                    startup
                        .encode_utf16()
                        .flat_map(u16::to_le_bytes)
                        .collect::<Vec<_>>(),
                );
                args.extend([
                    format!("{profile}.exe"),
                    "-NoExit".into(),
                    "-EncodedCommand".into(),
                    encoded,
                ]);
            }
            "cmd" => {
                if startup.contains(['\0', '\r', '\n']) {
                    return Err("Il comando di avvio per cmd deve occupare una riga".into());
                }
                // Terminal racchiude gli argomenti con spazi tra virgolette. /s
                // rimuove questa coppia esterna lasciando intatte le quote interne.
                args.extend([
                    "cmd.exe".into(),
                    "/d".into(),
                    "/s".into(),
                    "/k".into(),
                    wt_value(&format!(" {startup}"))?,
                ]);
            }
            _ => return Err(format!("Profilo non supportato: {profile}")),
        }
    }
    Ok(())
}

fn wt_value(value: &str) -> Result<String, String> {
    if value.contains('\0') {
        return Err("Un argomento Windows Terminal contiene NUL".into());
    }
    // Terminal ha un proprio parser di delimitatori anche quando riceve argv.
    Ok(value.replace(';', "\\;"))
}

fn entries(value: &Value) -> impl Iterator<Item = &Value> {
    value.as_array().into_iter().flatten()
}

fn string<'a>(object: &'a Value, field: &str) -> Result<&'a str, String> {
    object[field]
        .as_str()
        .ok_or_else(|| format!("Campo non valido: {field}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ids() -> BTreeMap<String, String> {
        [
            (
                "pwsh".into(),
                "{12345678-1234-1234-1234-123456789abc}".into(),
            ),
            (
                "cmd".into(),
                "{0caa0dad-35be-5f56-a8ff-afceeeaa6101}".into(),
            ),
        ]
        .into()
    }

    #[test]
    fn fragment_references_the_staged_image_and_maps_padding() {
        let config = json!({"appearance": {"pwsh": {
            "font": {"size": 14}, "padding": {"left": 2}, "backgroundImage": "C:\\art\\bg.PNG"
        }}});
        let output: Value =
            serde_json::from_str(&terminal_fragment(&config, &ids()).unwrap()).unwrap();
        assert_eq!(
            output["profiles"][0],
            json!({"updates": ids()["pwsh"], "font": {"size": 14}, "padding": "2, 8, 8, 8", "backgroundImage": "background-pwsh.png"})
        );
        assert_eq!(
            terminal_images(&config).unwrap()[0].source,
            r"C:\art\bg.PNG"
        );
        assert!(terminal_fragment(&config, &BTreeMap::new()).is_err());
    }

    #[test]
    fn workspace_delimits_actions_and_preserves_literal_semicolons() {
        let config = json!({"workspaces": [{"id": "demo", "tabs": [{
            "title": "A;B", "pane": {"profile": "pwsh", "startupCommand": "echo 1; echo 2"},
            "splits": [{"direction": "down", "pane": {"profile": "cmd", "workingDirectory": "C:\\dev"}}]
        }]}]});
        let args = workspace_args(&config, "demo", &ids()).unwrap();
        assert!(args.windows(2).any(|a| a == ["--title", r"A\;B"]));
        assert!(args
            .windows(3)
            .any(|a| a == [";", "split-pane", "--horizontal"]));
        assert!(args
            .windows(2)
            .any(|a| a == ["--startingDirectory", r"C:\dev"]));
        assert!(args.iter().any(|a| a == "-EncodedCommand"));
        assert!(workspace_args(&config, "missing", &ids()).is_err());
    }

    #[test]
    fn powershell_generation_scopes_commands_and_uses_bound_optional_parameters() {
        let config = json!({"commandPrefix": "/", "commands": [{
            "name": "sample", "shells": ["pwsh"], "template": {
                "program": "demo.exe", "args": ["{{braces}}", "{value}", "{count}"]
            },
            "parameters": [
                {"name": "value", "type": "string", "required": false},
                {"name": "count", "type": "integer", "required": false, "default": 0}
            ]
        }]});
        let script = powershell_init(&config, "pwsh").unwrap();
        assert!(script.contains("function /sample"));
        assert!(script.contains("$PSBoundParameters.ContainsKey('value')"));
        assert!(script.contains("[int]$count = 0"));
        assert!(script.contains("'{braces}'"));
        assert!(!script.contains("Invoke-Expression"));
        assert!(!powershell_init(&config, "powershell")
            .unwrap()
            .contains("/sample"));
    }

    #[test]
    fn change_directory_generates_set_location_in_the_session() {
        let config = json!({"commandPrefix": "/", "commands": [{
            "id": "p", "name": "progetti", "shells": ["pwsh"],
            "changeDirectory": r"C:\Users\anton\O'Neil"
        }]});
        let script = powershell_init(&config, "pwsh").unwrap();
        assert!(script.contains("function /progetti {"));
        assert!(script.contains(
            r"Microsoft.PowerShell.Management\Set-Location -LiteralPath 'C:\Users\anton\O''Neil'"
        ));
        assert!(!script.contains("__CliSuperpowerInvokeNative -Program"));
    }

    #[test]
    fn powershell_behavior_guards_versions_and_filters_sensitive_history() {
        let config = json!({"shellBehavior": {"pwsh": {
            "predictions": "history", "predictionView": "list", "historyExclude": ["Mio-Segreto"]
        }}});
        let script = powershell_init(&config, "pwsh").unwrap();
        assert!(script.contains("-PredictionSource History"));
        assert!(script.contains("-ge [version]'2.1.0'"));
        assert!(script.contains("-PredictionViewStyle ListView"));
        assert!(script.contains("-ge [version]'2.2.0'"));
        assert!(script.contains("'password'") && script.contains("'mio-segreto'"));
        assert!(script.contains("MemoryOnly"));
        assert!(script.contains("__CspPrevHistoryHandler"));
        assert!(script.contains("if (-not $global:__CspHistoryHandler)"));
        assert!(!powershell_init(&config, "powershell")
            .unwrap()
            .contains("PSReadLine"));
    }

    #[test]
    fn powershell_behavior_can_disable_predictions_and_the_default_filter() {
        let config = json!({"shellBehavior": {"powershell": {
            "predictions": "off", "hideSensitive": false
        }}});
        let script = powershell_init(&config, "powershell").unwrap();
        assert!(script.contains("-PredictionSource None"));
        assert!(!script.contains("AddToHistoryHandler"));
        assert!(!script.contains("PredictionViewStyle"));
    }

    #[test]
    fn powershell_banner_runs_only_in_wide_interactive_consoles() {
        let on = json!({"banner": {"enabled": true}});
        let script = powershell_init(&on, "pwsh").unwrap();
        assert!(script.contains("██╗"));
        assert!(script.contains("Write-Host @'"));
        assert!(script.contains("WindowSize.Width -ge 107"));
        assert!(script.contains("IsOutputRedirected"));
        assert!(script.contains("encodedcommand"));
        for config in [json!({"banner": {"enabled": false}}), json!({})] {
            assert!(!powershell_init(&config, "pwsh").unwrap().contains("Banner"));
        }
        assert!(banner_enabled(&on));
    }

    #[test]
    fn powershell_tab_accepts_suggestions_only_when_requested() {
        let tab =
            json!({"shellBehavior": {"pwsh": {"predictions": "history", "acceptKey": "tab"}}});
        let script = powershell_init(&tab, "pwsh").unwrap();
        assert!(script.contains("-Key Tab"));
        assert!(script.contains("AcceptSuggestion()"));
        assert!(script.contains("__CspTabFallback"));
        assert!(script.contains("-ge [version]'2.1.0'"));
        let right = json!({"shellBehavior": {"pwsh": {"predictions": "history"}}});
        assert!(!powershell_init(&right, "pwsh")
            .unwrap()
            .contains("-Key Tab"));
        let off = json!({"shellBehavior": {"pwsh": {"predictions": "off", "acceptKey": "tab"}}});
        assert!(!powershell_init(&off, "pwsh").unwrap().contains("-Key Tab"));
    }
}
