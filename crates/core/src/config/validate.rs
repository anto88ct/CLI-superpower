use super::TOOLS;
use serde_json::Value;
use std::collections::HashSet;

type Check = fn(&Value) -> Result<(), String>;
type Field<'a> = (&'a str, Check);

const PROFILES: &[&str] = &["pwsh", "powershell", "cmd"];
const SEGMENTS: &[&str] = &["directory", "git", "node", "dotnet", "duration", "status"];
const SHARED: &[Field<'_>] = &[
    ("appearance", appearance),
    ("commandPrefix", |v| one_of(v, &["", "/", "x-"])),
    ("commands", |v| items(v, command)),
    ("workspaces", |v| items(v, workspace)),
    ("shellBehavior", shell_behavior),
    ("banner", banner),
    ("prompt", prompt),
    ("tools", tools),
];

pub(super) fn personal(value: &Value) -> Result<(), String> {
    version(value)?;
    let mut fields = SHARED.to_vec();
    fields.extend([
        ("globalTerminal", global_terminal as Check),
        ("team", selection),
        ("overrides", overrides),
    ]);
    object(value, &[("schemaVersion", version_field)], &fields)
}

pub(super) fn team(value: &Value) -> Result<(), String> {
    version(value)?;
    object(
        value,
        &[
            ("schemaVersion", version_field),
            ("id", id),
            ("version", positive_integer),
        ],
        SHARED,
    )
}

fn version(value: &Value) -> Result<(), String> {
    version_field(&value["schemaVersion"]).map_err(|error| format!("schemaVersion: {error}"))
}

fn version_field(value: &Value) -> Result<(), String> {
    require(value.as_u64() == Some(6), "expected integer version 6")
}

fn appearance(value: &Value) -> Result<(), String> {
    map(value, |name, value| {
        require(PROFILES.contains(&name), "unknown profile")?;
        object(
            value,
            &[],
            &[
                ("colorScheme", non_empty_text),
                ("font", |v| {
                    object(
                        v,
                        &[],
                        &[
                            ("face", non_empty_text),
                            ("size", |v| {
                                require(
                                    v.as_f64().is_some_and(|n| n > 0.0),
                                    "expected a positive number",
                                )
                            }),
                        ],
                    )
                }),
                ("backgroundImage", windows_path),
                ("backgroundImageOpacity", |v| {
                    require(
                        v.as_f64().is_some_and(|n| (0.0..=1.0).contains(&n)),
                        "expected a number from 0 to 1",
                    )
                }),
                ("backgroundImageAlignment", |v| {
                    one_of(
                        v,
                        &[
                            "center",
                            "left",
                            "top",
                            "right",
                            "bottom",
                            "topLeft",
                            "topRight",
                            "bottomLeft",
                            "bottomRight",
                        ],
                    )
                }),
                ("opacity", |v| integer(v, 0, 100)),
                ("useAcrylic", boolean),
                ("cursorShape", |v| {
                    one_of(
                        v,
                        &[
                            "bar",
                            "vintage",
                            "underscore",
                            "filledBox",
                            "emptyBox",
                            "doubleUnderscore",
                        ],
                    )
                }),
                ("padding", |v| {
                    object(
                        v,
                        &[],
                        &[
                            ("left", nonnegative_integer),
                            ("top", nonnegative_integer),
                            ("right", nonnegative_integer),
                            ("bottom", nonnegative_integer),
                        ],
                    )
                }),
            ],
        )
    })
}

fn global_terminal(value: &Value) -> Result<(), String> {
    object(value, &[], &[("defaultProfile", profile)])
}

fn command(value: &Value) -> Result<(), String> {
    object(
        value,
        &[("id", id), ("name", id), ("shells", shells)],
        &[
            ("template", template),
            ("changeDirectory", windows_path),
            ("description", text),
            ("workingDirectory", windows_path),
            ("parameters", parameters),
        ],
    )?;
    if value.get("changeDirectory").is_some() {
        require(
            value.get("template").is_none()
                && value
                    .get("parameters")
                    .is_none_or(|p| p.as_array().is_some_and(Vec::is_empty))
                && value.get("workingDirectory").is_none(),
            "changeDirectory: non si combina con template, parametri o workingDirectory",
        )?;
        return Ok(());
    }
    require(
        value.get("template").is_some(),
        "template: obbligatorio se manca changeDirectory",
    )?;

    let mut remaining = HashSet::new();
    if let Some(parameters) = value["parameters"].as_array() {
        require(
            parameters.is_empty()
                || !value["shells"]
                    .as_array()
                    .is_some_and(|shells| shells.iter().any(|shell| shell == "cmd")),
            "shells: i comandi CMD supportano solo argomenti fissi",
        )?;
        for parameter in parameters {
            remaining.insert(
                parameter["name"]
                    .as_str()
                    .expect("validated parameter name"),
            );
        }
    }
    let declared = remaining.clone();
    for arg in value["template"]["args"]
        .as_array()
        .expect("validated arguments")
    {
        let arg = arg.as_str().expect("validated argument");
        if let Some(name) = placeholder(arg)? {
            require(
                declared.contains(name),
                format!("template.args: unknown parameter {name:?}"),
            )?;
            remaining.remove(name);
        }
    }
    require(
        remaining.is_empty(),
        "parameters: all declared parameters must be used",
    )
}

fn shells(value: &Value) -> Result<(), String> {
    array(value, true, |shell| one_of(shell, PROFILES))?;
    unique(value, |shell| shell.as_str().expect("validated shell"))
}

fn template(value: &Value) -> Result<(), String> {
    object(
        value,
        &[("program", program), ("args", |v| array(v, false, text))],
        &[],
    )
}

fn program(value: &Value) -> Result<(), String> {
    non_empty_text(value)?;
    let name = value.as_str().expect("validated program");
    if name.contains(['\\', '/', ':']) {
        windows_path(value)
    } else {
        require(
            valid_path_part(name),
            "expected an executable name or absolute Windows path",
        )
    }
}

fn parameters(value: &Value) -> Result<(), String> {
    array(value, false, parameter)?;
    unique(value, |p| p["name"].as_str().expect("validated name"))?;
    let mut optional_seen = false;
    for parameter in value.as_array().expect("validated parameters") {
        if parameter["required"] == true {
            require(
                !optional_seen,
                "required parameters must precede optional parameters",
            )?;
        } else {
            optional_seen = true;
        }
    }
    Ok(())
}

fn parameter(value: &Value) -> Result<(), String> {
    object(
        value,
        &[
            ("name", parameter_name),
            ("type", |v| one_of(v, &["string", "integer"])),
            ("required", boolean),
        ],
        &[("description", text), ("default", |_| Ok(()))],
    )?;
    if let Some(default) = value.get("default") {
        require(
            value["required"] == false,
            "default: permitted only for an optional parameter",
        )?;
        if value["type"] == "integer" {
            integer(default, i32::MIN, i32::MAX)
        } else {
            text(default)
        }
        .map_err(|error| format!("default: {error}"))?;
    }
    Ok(())
}

/// A placeholder is recognized only when it occupies the entire argument.
fn placeholder(argument: &str) -> Result<Option<&str>, String> {
    if let Some(name) = argument
        .strip_prefix('{')
        .and_then(|arg| arg.strip_suffix('}'))
    {
        if valid_id(name, b'_') {
            return Ok(Some(name));
        }
    }
    let unescaped = argument.replace("{{", "").replace("}}", "");
    require(
        !unescaped.contains(['{', '}']),
        "template.args: use a whole-argument parameter or doubled literal braces",
    )?;
    Ok(None)
}

fn workspace(value: &Value) -> Result<(), String> {
    object(
        value,
        &[
            ("id", id),
            ("name", non_empty_text),
            ("tabs", |v| array(v, true, tab)),
        ],
        &[],
    )
}

fn tab(value: &Value) -> Result<(), String> {
    object(
        value,
        &[("pane", pane)],
        &[("title", text), ("splits", |v| array(v, false, split))],
    )
}

fn split(value: &Value) -> Result<(), String> {
    object(
        value,
        &[
            ("direction", |v| one_of(v, &["right", "down"])),
            ("pane", pane),
        ],
        &[],
    )
}

fn pane(value: &Value) -> Result<(), String> {
    object(
        value,
        &[("profile", profile)],
        &[
            ("workingDirectory", windows_path),
            ("startupCommand", non_empty_text),
        ],
    )
}

fn profile(value: &Value) -> Result<(), String> {
    one_of(value, PROFILES)
}

fn prompt(value: &Value) -> Result<(), String> {
    object(
        value,
        &[],
        &[
            ("enabled", boolean),
            ("segments", |v| {
                array(v, false, segment)?;
                unique(v, |segment| {
                    segment["id"].as_str().expect("validated segment id")
                })
            }),
        ],
    )
}

fn segment(value: &Value) -> Result<(), String> {
    object(
        value,
        &[("id", |v| one_of(v, SEGMENTS))],
        &[
            ("color", color),
            ("thresholdMs", nonnegative_integer),
            ("pathStyle", |v| {
                one_of(v, &["short", "home", "full", "folder"])
            }),
        ],
    )?;
    require(
        value.get("thresholdMs").is_none() || value["id"] == "duration",
        "thresholdMs: permitted only for duration",
    )?;
    require(
        value.get("pathStyle").is_none() || value["id"] == "directory",
        "pathStyle: permitted only for directory",
    )
}

fn color(value: &Value) -> Result<(), String> {
    let valid = value
        .as_str()
        .and_then(|value| value.strip_prefix('#'))
        .is_some_and(|color| {
            color.len() == 6 && color.bytes().all(|byte| byte.is_ascii_hexdigit())
        });
    require(valid, "expected #RRGGBB")
}

fn shell_behavior(value: &Value) -> Result<(), String> {
    map(value, |shell, value| {
        require(PROFILES.contains(&shell), "unknown shell")?;
        object(
            value,
            &[],
            &[
                ("predictions", |v| one_of(v, &["history", "off"])),
                ("predictionView", |v| one_of(v, &["inline", "list"])),
                ("acceptKey", |v| one_of(v, &["right", "tab"])),
                ("hideSensitive", boolean),
                ("historyExclude", |v| array(v, false, history_pattern)),
            ],
        )?;
        require(
            value["historyExclude"]
                .as_array()
                .is_none_or(|v| v.len() <= 50),
            "historyExclude: al massimo 50 frammenti",
        )
    })
}

fn banner(value: &Value) -> Result<(), String> {
    object(value, &[], &[("enabled", boolean)])
}

fn history_pattern(value: &Value) -> Result<(), String> {
    non_empty_text(value)?;
    let pattern = value.as_str().expect("validated pattern");
    require(
        pattern.chars().count() <= 100 && !pattern.chars().any(char::is_control),
        "expected at most 100 characters without control characters",
    )
}

fn tools(value: &Value) -> Result<(), String> {
    map(value, |name, value| {
        require(TOOLS.contains(&name), "unknown tool")?;
        object(value, &[("enabled", boolean)], &[])
    })
}

fn selection(value: &Value) -> Result<(), String> {
    object(
        value,
        &[("packageId", id), ("installedVersion", positive_integer)],
        &[("sourceUrl", https_url)],
    )
}

fn https_url(value: &Value) -> Result<(), String> {
    let valid = value
        .as_str()
        .filter(|value| {
            value
                .get(..8)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
        })
        .and_then(|value| url::Url::parse(value).ok())
        .is_some_and(|url| url.host_str().is_some());
    require(valid, "expected an absolute HTTPS URL")
}

fn overrides(value: &Value) -> Result<(), String> {
    map(value, |package, value| {
        validate_id(package, b'-')?;
        object(
            value,
            &[],
            &[
                ("commands", |v| overrides_for(v, command)),
                ("workspaces", |v| overrides_for(v, workspace)),
            ],
        )
    })
}

fn overrides_for(value: &Value, check: Check) -> Result<(), String> {
    map(value, |key, value| {
        validate_id(key, b'-')?;
        match value["action"].as_str() {
            Some("disable") => object(value, &[("action", text)], &[]),
            Some("replace") => {
                object(value, &[("action", text), ("value", check)], &[])?;
                require(
                    value["value"]["id"] == key,
                    "value.id: must match the override key",
                )
            }
            _ => Err("action: expected disable or replace".into()),
        }
    })
}

fn items(value: &Value, check: Check) -> Result<(), String> {
    array(value, false, check)?;
    unique(value, |item| item["id"].as_str().expect("validated id"))
}

fn unique(value: &Value, key: impl Fn(&Value) -> &str) -> Result<(), String> {
    let mut seen = HashSet::new();
    for element in value.as_array().expect("validated array") {
        let key = key(element);
        if !seen.insert(key) {
            return Err(format!("duplicate value {key:?}"));
        }
    }
    Ok(())
}

fn object(value: &Value, required: &[Field<'_>], optional: &[Field<'_>]) -> Result<(), String> {
    let value = value.as_object().ok_or("expected an object")?;
    for &(key, _) in required {
        if !value.contains_key(key) {
            return Err(format!("{key}: required field is missing"));
        }
    }
    for (key, value) in value {
        let (_, check) = required
            .iter()
            .chain(optional)
            .find(|(field, _)| field == key)
            .ok_or_else(|| format!("{key}: unknown field"))?;
        check(value).map_err(|err| format!("{key}: {err}"))?;
    }
    Ok(())
}

fn map(value: &Value, check: impl Fn(&str, &Value) -> Result<(), String>) -> Result<(), String> {
    let value = value.as_object().ok_or("expected an object")?;
    for (key, value) in value {
        check(key, value).map_err(|err| format!("{key}: {err}"))?;
    }
    Ok(())
}

fn array(value: &Value, non_empty: bool, check: Check) -> Result<(), String> {
    let value = value.as_array().ok_or("expected an array")?;
    require(
        !non_empty || !value.is_empty(),
        "expected at least one element",
    )?;
    for (index, value) in value.iter().enumerate() {
        check(value).map_err(|err| format!("[{index}]: {err}"))?;
    }
    Ok(())
}

fn one_of(value: &Value, options: &[&str]) -> Result<(), String> {
    require(
        value.as_str().is_some_and(|value| options.contains(&value)),
        format!("expected one of {options:?}"),
    )
}

fn text(value: &Value) -> Result<(), String> {
    require(value.is_string(), "expected a string")
}

fn non_empty_text(value: &Value) -> Result<(), String> {
    require(
        value.as_str().is_some_and(|value| !value.trim().is_empty()),
        "expected a non-empty string",
    )
}

fn boolean(value: &Value) -> Result<(), String> {
    require(value.is_boolean(), "expected a boolean")
}

fn integer(value: &Value, min: i32, max: i32) -> Result<(), String> {
    require(
        value
            .as_i64()
            .is_some_and(|n| (i64::from(min)..=i64::from(max)).contains(&n)),
        format!("expected an integer from {min} to {max}"),
    )
}

fn positive_integer(value: &Value) -> Result<(), String> {
    integer(value, 1, i32::MAX)
}

fn nonnegative_integer(value: &Value) -> Result<(), String> {
    integer(value, 0, i32::MAX)
}

fn id(value: &Value) -> Result<(), String> {
    validate_id(value.as_str().ok_or("expected an ID string")?, b'-')
}

fn parameter_name(value: &Value) -> Result<(), String> {
    validate_id(value.as_str().ok_or("expected a parameter name")?, b'_')
}

fn validate_id(value: &str, separator: u8) -> Result<(), String> {
    require(
        valid_id(value, separator),
        format!(
            "expected a lowercase ASCII identifier with {:?} as separator",
            char::from(separator)
        ),
    )
}

fn valid_id(value: &str, separator: u8) -> bool {
    let mut bytes = value.bytes();
    value.len() <= 64
        && bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == separator)
}

fn windows_path(value: &Value) -> Result<(), String> {
    let path = value
        .as_str()
        .ok_or("expected a path string")?
        .replace('/', "\\");
    let absolute = if let Some(path) = path.strip_prefix(r"\\?\") {
        drive_path(path)
            || path
                .get(..4)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case(r"UNC\"))
                && unc_path(&path[4..])
    } else if let Some(path) = path.strip_prefix(r"\\") {
        unc_path(path)
    } else {
        drive_path(&path)
    };
    require(absolute, "expected an absolute Windows drive or UNC path")
}

fn drive_path(path: &str) -> bool {
    matches!(path.as_bytes(), [letter, b':', b'\\', ..] if letter.is_ascii_alphabetic())
        && path[3..].split('\\').all(valid_path_part)
}

fn unc_path(path: &str) -> bool {
    let mut parts = path.split('\\');
    let valid_unc_part = |part: &str| !matches!(part, "" | "." | "..") && valid_path_part(part);
    parts.next().is_some_and(valid_unc_part)
        && parts.next().is_some_and(valid_unc_part)
        && parts.all(valid_path_part)
}

fn valid_path_part(part: &str) -> bool {
    !part
        .chars()
        .any(|c| c.is_ascii_control() || matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*'))
}

fn require(condition: bool, message: impl Into<String>) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}
