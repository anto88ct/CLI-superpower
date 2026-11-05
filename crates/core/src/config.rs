//! Risoluzione pura della configurazione persistente (v2, con migrazione da v1).

use serde_json::{json, Map, Value};
use std::collections::HashSet;

mod parse;
mod validate;

pub use parse::parse_json;

const TOOLS: &[&str] = &[
    "pwsh",
    "clink",
    "oh-my-posh",
    "starship",
    "nerd-font",
    "zoxide",
    "fzf",
    "tealdeer",
    "eza",
    "bat",
    "jq",
    "lazygit",
];

/// Versione corrente del formato dei documenti di configurazione.
pub const SCHEMA_VERSION: u64 = 6;

/// Porta un documento supportato alla versione corrente, un salto alla volta.
///
/// v1 → v2: la versione 2 aggiunge `changeDirectory` ai comandi.
/// v2 → v3: la versione 3 aggiunge `shellBehavior`.
/// v3 → v4: la versione 4 aggiunge `banner`.
/// v4 → v5: la versione 5 aggiunge `shellBehavior.*.acceptKey`.
/// v5 → v6: la versione 6 aggiunge `prompt.segments[].pathStyle`.
/// Ogni documento della versione precedente è già valido: i salti cambiano solo `schemaVersion`.
pub fn migrate(document: &Value) -> Result<Value, String> {
    let mut document = document.clone();
    loop {
        let version = document["schemaVersion"]
            .as_u64()
            .ok_or("schemaVersion: expected an integer version")?;
        match version {
            1..=5 => document["schemaVersion"] = json!(version + 1),
            SCHEMA_VERSION => return Ok(document),
            _ => {
                return Err(format!(
                    "schemaVersion: la versione {version} non è supportata"
                ))
            }
        }
    }
}

/// Combina i documenti v1 e restituisce le preferenze effettive per i generatori.
///
/// Per input testuale usare prima [parse_json], che può individuare chiavi duplicate.
pub fn resolve(personal: &Value, team: Option<&Value>) -> Result<Value, String> {
    let personal = &migrate(personal).map_err(|err| format!("personal: {err}"))?;
    let team = team
        .map(migrate)
        .transpose()
        .map_err(|err| format!("team: {err}"))?;
    let team = team.as_ref();
    validate::personal(personal).map_err(|err| format!("personal: {err}"))?;
    let team = active_team(personal, team)?;
    let overrides = team.and_then(|team| personal["overrides"].get(team["id"].as_str()?));

    let mut effective = json!({
        "schemaVersion": SCHEMA_VERSION,
        "appearance": {},
        "globalTerminal": {},
        "commandPrefix": "",
        "shellBehavior": {},
        "banner": {"enabled": true},
        "prompt": {"enabled": false, "segments": []},
        "tools": TOOLS.iter().map(|tool| (tool.to_string(), json!({"enabled": false}))).collect::<Map<_, _>>(),
    });

    for layer in team.into_iter().chain([personal]) {
        for key in [
            "appearance",
            "globalTerminal",
            "commandPrefix",
            "shellBehavior",
            "banner",
            "prompt",
            "tools",
        ] {
            if let Some(value) = layer.get(key) {
                overlay(&mut effective[key], value);
            }
        }
    }
    for key in ["commands", "workspaces"] {
        effective[key] = merge_items(
            key,
            team.and_then(|team| team.get(key)),
            personal.get(key),
            overrides.and_then(|overrides| overrides.get(key)),
        )?;
    }
    validate_effective(&effective)?;
    set_nested_defaults(&mut effective);
    Ok(effective)
}

fn active_team<'a>(personal: &Value, team: Option<&'a Value>) -> Result<Option<&'a Value>, String> {
    match (personal.get("team"), team) {
        (None, None) => Ok(None),
        (None, Some(_)) => Err("team: provided without an active package".into()),
        (Some(_), None) => Err("team: active package is missing".into()),
        (Some(selection), Some(team)) => {
            validate::team(team).map_err(|err| format!("team: {err}"))?;
            if selection["packageId"] != team["id"] {
                return Err("team.packageId: does not match the active package id".into());
            }
            if selection["installedVersion"] != team["version"] {
                return Err(
                    "team.installedVersion: does not match the active package version".into(),
                );
            }
            Ok(Some(team))
        }
    }
}

fn overlay(base: &mut Value, top: &Value) {
    if let (Some(base), Some(top)) = (base.as_object_mut(), top.as_object()) {
        for (key, value) in top {
            overlay(base.entry(key.clone()).or_insert(Value::Null), value);
        }
    } else {
        *base = top.clone();
    }
}

fn merge_items(
    key: &str,
    team: Option<&Value>,
    personal: Option<&Value>,
    overrides: Option<&Value>,
) -> Result<Value, String> {
    let team = team
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    let personal = personal
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    let team_ids: HashSet<_> = team.iter().map(item_id).collect();
    if let Some(id) = personal
        .iter()
        .map(item_id)
        .find(|id| team_ids.contains(id))
    {
        return Err(format!("{key}: personal id {id:?} conflicts with the team"));
    }
    let mut items = Vec::with_capacity(team.len() + personal.len());
    for item in team {
        let replacement = overrides.and_then(|entries| entries.get(item_id(item)));
        match replacement {
            Some(override_item) if override_item["action"] == "disable" => {}
            Some(override_item) => items.push(override_item["value"].clone()),
            None => items.push(item.clone()),
        }
    }
    items.extend_from_slice(personal);
    Ok(Value::Array(items))
}

fn item_id(item: &Value) -> &str {
    item["id"].as_str().expect("item id was validated")
}

fn validate_effective(effective: &Value) -> Result<(), String> {
    if effective["prompt"]["enabled"] == true
        && effective["prompt"]["segments"]
            .as_array()
            .is_some_and(Vec::is_empty)
    {
        return Err("prompt.segments: required when the effective prompt is enabled".into());
    }
    let prefix = effective["commandPrefix"]
        .as_str()
        .expect("prefix was validated");
    let mut names = HashSet::new();
    for command in effective["commands"]
        .as_array()
        .expect("commands were merged")
    {
        let name = command["name"].as_str().expect("name was validated");
        for shell in command["shells"].as_array().expect("shells were validated") {
            let shell = shell.as_str().expect("shell was validated");
            if !names.insert((shell, name)) {
                return Err(format!(
                    "commands: duplicate public name {prefix}{name} in {shell}"
                ));
            }
        }
    }
    Ok(())
}

fn set_nested_defaults(effective: &mut Value) {
    if let Some(commands) = effective["commands"].as_array_mut() {
        for command in commands {
            set_default(command, "parameters", json!([]));
        }
    }
    if let Some(workspaces) = effective["workspaces"].as_array_mut() {
        for workspace in workspaces {
            if let Some(tabs) = workspace["tabs"].as_array_mut() {
                for tab in tabs {
                    set_default(tab, "splits", json!([]));
                }
            }
        }
    }
    if let Some(segments) = effective["prompt"]["segments"].as_array_mut() {
        for segment in segments {
            if segment["id"] == "duration" {
                set_default(segment, "thresholdMs", json!(2000));
            }
        }
    }
}

fn set_default(object: &mut Value, key: &str, value: Value) {
    object
        .as_object_mut()
        .expect("object was validated")
        .entry(key)
        .or_insert(value);
}

#[cfg(test)]
mod tests;
