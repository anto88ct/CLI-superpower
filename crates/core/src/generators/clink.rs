use super::banner;
use super::behavior::behavior;
use serde_json::Value;
use std::fmt::Write;
use std::path::Path;

/// Expand only exact shortcuts at the interactive CMD prompt.
pub fn clink_script(config: &Value) -> Result<Option<String>, String> {
    clink_script_with_theme(config, None)
}

/// Come [clink_script], con in più il prompt informativo: se `theme` è il file di
/// configurazione di Oh My Posh e il prompt è attivo, lo script lo inizializza per CMD.
pub fn clink_script_with_theme(
    config: &Value,
    theme: Option<&Path>,
) -> Result<Option<String>, String> {
    let prefix = config["commandPrefix"].as_str().unwrap_or_default();
    let mut entries = String::new();
    for command in config["commands"].as_array().into_iter().flatten() {
        if !command["shells"]
            .as_array()
            .is_some_and(|shells| shells.iter().any(|shell| shell == "cmd"))
        {
            continue;
        }
        let name = command["name"].as_str().ok_or("Comando CMD senza nome")?;
        let shortcut = format!("{prefix}{name}");
        if command["parameters"]
            .as_array()
            .is_some_and(|params| !params.is_empty())
        {
            return Err(format!("{shortcut}: in CMD usa argomenti fissi. I parametri inseriti al prompt richiedono PowerShell."));
        }
        if let Some(directory) = command["changeDirectory"].as_str() {
            // La riga sostitutiva è eseguita dal CMD interattivo, quindi `cd` ne cambia la cartella.
            let line = format!("cd /d {}", cmd_literal(directory, &shortcut)?);
            writeln!(
                entries,
                "    [{}] = {},",
                lua_literal(&shortcut),
                lua_literal(&line)
            )
            .unwrap();
            continue;
        }
        let template = &command["template"];
        let program = template["program"]
            .as_str()
            .ok_or("Comando CMD senza programma")?;
        let mut invocation = String::new();
        if let Some(directory) = command["workingDirectory"].as_str() {
            write!(
                invocation,
                "pushd {} && ",
                cmd_literal(directory, &shortcut)?
            )
            .unwrap();
        }
        // A quoted bare batch name can make CMD resolve %~dp0 to the current
        // directory. Keep simple PATH lookups unquoted.
        if program
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        {
            invocation.push_str(program);
        } else {
            invocation.push_str(&cmd_literal(program, &shortcut)?);
        }
        for arg in template["args"].as_array().into_iter().flatten() {
            let arg = arg.as_str().ok_or("Argomento CMD non valido")?;
            let arg = arg.replace("{{", "{").replace("}}", "}");
            write!(invocation, " {}", cmd_literal(&arg, &shortcut)?).unwrap();
        }
        let line = format!(r#"cmd.exe /d /v:off /s /c "{invocation}""#);
        if line.encode_utf16().count() > 8191 {
            return Err(format!(
                "{shortcut}: il comando supera il limite di lunghezza di CMD."
            ));
        }
        writeln!(
            entries,
            "    [{}] = {},",
            lua_literal(&shortcut),
            lua_literal(&line)
        )
        .unwrap();
    }
    let behavior = behavior_script(config);
    let prompt = theme
        .filter(|_| config["prompt"]["enabled"] == true)
        .map(prompt_script)
        .unwrap_or_default();
    if entries.is_empty() && behavior.is_empty() && prompt.is_empty() {
        return Ok(None);
    }
    // Il banner non basta da solo a richiedere Clink: abilitarlo modifica l'avvio automatico di CMD.
    let mut script = format!(
        "-- Generato da CLI Superpower.\n{behavior}{prompt}{}",
        banner::lua(config)
    );
    if !entries.is_empty() {
        write!(
            script,
            r#"local shortcuts = {{
{entries}}}
clink.onfilterinput(function(line)
    local name = line:match("^%s*(%S+)%s*$")
    local replacement = name and shortcuts[name:lower()]
    if replacement then
        return replacement, false
    end
end)
"#
        )
        .unwrap();
    }
    Ok(Some(script))
}

/// Oh My Posh genera lo script Lua del prompt per Clink (`init cmd`). Se Oh My Posh o il file
/// del tema mancano non succede nulla: CMD mantiene il suo prompt.
fn prompt_script(theme: &Path) -> String {
    format!(
        r#"local function csp_prompt(theme)
    local file = io.open(theme, "r")
    if not file then return end
    file:close()
    local pipe = io.popen('oh-my-posh.exe init cmd --config "' .. theme .. '"')
    if not pipe then return end
    local script = pipe:read("*a")
    pipe:close()
    if script and script ~= "" then
        local chunk = load(script)
        if chunk then pcall(chunk) end
    end
end
csp_prompt({})
"#,
        lua_literal(&theme.to_string_lossy())
    )
}

/// Tab inserisce il suggerimento se c'è, altrimenti esegue il comando che aveva prima.
const TAB_ACCEPT: &str = r#"local tab_command, tab_kind = rl.getbinding([["\t"]])
if tab_kind ~= "function" then tab_command = "complete" end
function csp_tab_accept(rl_buffer)
    if rl_buffer:hassuggestion() then
        rl_buffer:insertsuggestion("all")
    else
        rl.invokecommand(tab_command)
    end
end
rl.describemacro([["luafunc:csp_tab_accept"]], "Insert the suggestion, or complete")
rl.setbinding([["\t"]], [["luafunc:csp_tab_accept"]])
"#;

/// `settings.set` vale solo per la sessione e non tocca il file delle impostazioni di Clink:
/// rimuovere questo script ripristina le impostazioni dell'utente.
fn behavior_script(config: &Value) -> String {
    let Some(behavior) = behavior(config, "cmd") else {
        return String::new();
    };
    let mut script = String::new();
    match behavior.predictions {
        Some("off") => script.push_str("settings.set(\"autosuggest.enable\", false)\n"),
        Some(_) => {
            // Il suggerimento in linea è disattivato di default in Clink (`autosuggest.inline`).
            // La vista a elenco parte già aperta; in quel caso Clink non mostra il suggerimento in linea.
            let list = behavior.view == Some("list");
            write!(
                script,
                "settings.set(\"autosuggest.enable\", true)\nsettings.set(\"autosuggest.strategy\", \"history\")\nsettings.set(\"autosuggest.inline\", {})\nsettings.set(\"suggestionlist.default\", {list})\n",
                !list
            )
            .unwrap();
        }
        None => {}
    }
    if behavior.accept_with_tab {
        script.push_str(TAB_ACCEPT);
    }
    if !behavior.patterns.is_empty() {
        let patterns: Vec<_> = behavior.patterns.iter().map(|p| lua_literal(p)).collect();
        write!(
            script,
            r#"local sensitive = {{ {} }}
clink.onhistory(function(line)
    local lower = line:lower()
    for _, pattern in ipairs(sensitive) do
        if lower:find(pattern, 1, true) then
            return false
        end
    end
end)
"#,
            patterns.join(", ")
        )
        .unwrap();
    }
    script
}

fn cmd_literal(value: &str, shortcut: &str) -> Result<String, String> {
    if value
        .chars()
        .any(|ch| ch.is_control() || matches!(ch, '"' | '%' | '!' | '&' | '|' | '<' | '>' | '^'))
    {
        return Err(format!("{shortcut}: CMD non può passare questo valore come testo letterale. Usa PowerShell per questo comando."));
    }
    // CMD strips the outer command string quotes. The inner quotes preserve spaces.
    Ok(format!("\"{value}\""))
}

fn lua_literal(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use serde_json::json;

    #[test]
    fn rundev_generates_an_exact_clink_shortcut() {
        let effective = config::resolve(
            &json!({
                "schemaVersion": 1,
                "commandPrefix": "/",
                "commands": [{
                    "id": "rundev", "name": "rundev", "shells": ["cmd"],
                    "template": {"program": "npm.cmd", "args": ["run", "dev"]}
                }]
            }),
            None,
        )
        .unwrap();
        let script = clink_script(&effective).unwrap().unwrap();
        assert!(script
            .contains(r#"["/rundev"] = "cmd.exe /d /v:off /s /c \"npm.cmd \"run\" \"dev\"\"""#));
        assert!(script.contains(r#"line:match("^%s*(%S+)%s*$")"#));
    }

    #[test]
    fn change_directory_runs_in_the_interactive_cmd() {
        let effective = config::resolve(
            &json!({
                "schemaVersion": 3,
                "commandPrefix": "/",
                "commands": [{
                    "id": "progetti", "name": "progetti", "shells": ["cmd"],
                    "changeDirectory": r"C:\Users\anton\PROGETTI"
                }]
            }),
            None,
        )
        .unwrap();
        let script = clink_script(&effective).unwrap().unwrap();
        assert!(script.contains(r#"["/progetti"] = "cd /d \"C:\\Users\\anton\\PROGETTI\"","#));
        assert!(!script.contains("cmd.exe"));
    }

    #[test]
    fn behavior_sets_session_settings_and_filters_history() {
        let effective = config::resolve(
            &json!({
                "schemaVersion": 3,
                "shellBehavior": {"cmd": {"predictions": "history", "historyExclude": ["Mio"]}}
            }),
            None,
        )
        .unwrap();
        let script = clink_script(&effective).unwrap().unwrap();
        assert!(script.contains(r#"settings.set("autosuggest.enable", true)"#));
        assert!(script.contains(r#"settings.set("autosuggest.inline", true)"#));
        assert!(script.contains(r#"settings.set("suggestionlist.default", false)"#));
        assert!(script.contains(r#""password""#) && script.contains(r#""mio""#));
        assert!(script.contains("clink.onhistory"));
        assert!(!script.contains("onfilterinput"));
        let off = config::resolve(
            &json!({"schemaVersion": 3, "shellBehavior": {"cmd": {"predictions": "off", "hideSensitive": false}}}),
            None,
        )
        .unwrap();
        let script = clink_script(&off).unwrap().unwrap();
        assert!(script.contains(r#"settings.set("autosuggest.enable", false)"#));
        assert!(!script.contains("onhistory"));
        let none = config::resolve(&json!({"schemaVersion": 3}), None).unwrap();
        assert!(clink_script(&none).unwrap().is_none());
    }

    #[test]
    fn list_view_starts_with_the_suggestion_list_open() {
        let effective = config::resolve(
            &json!({"schemaVersion": 3, "shellBehavior": {"cmd": {"predictions": "history", "predictionView": "list", "hideSensitive": false}}}),
            None,
        )
        .unwrap();
        let script = clink_script(&effective).unwrap().unwrap();
        assert!(script.contains(r#"settings.set("autosuggest.inline", false)"#));
        assert!(script.contains(r#"settings.set("suggestionlist.default", true)"#));
    }

    #[test]
    fn banner_is_added_only_when_clink_is_already_needed() {
        let with_command = config::resolve(
            &json!({"schemaVersion": 4, "commands": [{
                "id": "a", "name": "a", "shells": ["cmd"],
                "changeDirectory": "C:/Progetti"
            }]}),
            None,
        )
        .unwrap();
        let script = clink_script(&with_command).unwrap().unwrap();
        assert!(script.contains("clink.onbeginedit"));
        assert!(script.contains("console.getwidth() >= 107"));
        assert!(script.contains("[=["));
        let off = config::resolve(
            &json!({"schemaVersion": 4, "banner": {"enabled": false}, "commands": [{
                "id": "a", "name": "a", "shells": ["cmd"],
                "changeDirectory": "C:/Progetti"
            }]}),
            None,
        )
        .unwrap();
        assert!(!clink_script(&off).unwrap().unwrap().contains("onbeginedit"));
        // Da solo il banner non genera lo script, quindi non abilita Clink.
        let only_banner = config::resolve(&json!({"schemaVersion": 4}), None).unwrap();
        assert!(clink_script(&only_banner).unwrap().is_none());
    }

    #[test]
    fn prompt_initializes_oh_my_posh_only_when_enabled_and_a_theme_is_given() {
        let theme =
            Path::new(r"C:\Users\anton\AppData\Local\CliSuperpower\generated\prompt.omp.json");
        let on = config::resolve(
            &json!({"schemaVersion": 5, "prompt": {"enabled": true, "segments": [{"id": "directory"}]}}),
            None,
        )
        .unwrap();
        let script = clink_script_with_theme(&on, Some(theme)).unwrap().unwrap();
        assert!(script.contains("oh-my-posh.exe init cmd --config"));
        assert!(script.contains(r#"csp_prompt("C:\\Users\\anton\\AppData\\Local\\CliSuperpower\\generated\\prompt.omp.json")"#));
        assert!(script.contains("pcall(chunk)"));
        assert!(clink_script_with_theme(&on, None).unwrap().is_none());
        let off = config::resolve(&json!({"schemaVersion": 5}), None).unwrap();
        assert!(clink_script_with_theme(&off, Some(theme))
            .unwrap()
            .is_none());
    }

    #[test]
    fn tab_accepts_the_suggestion_and_falls_back_to_completion() {
        let tab = config::resolve(
            &json!({"schemaVersion": 5, "shellBehavior": {"cmd": {"predictions": "history", "acceptKey": "tab", "hideSensitive": false}}}),
            None,
        )
        .unwrap();
        let script = clink_script(&tab).unwrap().unwrap();
        assert!(script.contains("rl_buffer:hassuggestion()"));
        assert!(script.contains("rl_buffer:insertsuggestion(\"all\")"));
        assert!(script.contains("rl.invokecommand(tab_command)"));
        assert!(script.contains("luafunc:csp_tab_accept"));
        let right = config::resolve(
            &json!({"schemaVersion": 5, "shellBehavior": {"cmd": {"predictions": "history", "acceptKey": "right", "hideSensitive": false}}}),
            None,
        )
        .unwrap();
        assert!(!clink_script(&right)
            .unwrap()
            .unwrap()
            .contains("csp_tab_accept"));
        let off = config::resolve(
            &json!({"schemaVersion": 5, "shellBehavior": {"cmd": {"predictions": "off", "acceptKey": "tab", "hideSensitive": false}}}),
            None,
        )
        .unwrap();
        assert!(!clink_script(&off)
            .unwrap()
            .unwrap()
            .contains("csp_tab_accept"));
    }
}
