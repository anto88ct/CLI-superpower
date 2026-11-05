use super::{entries, string};
use serde_json::{json, Value};

pub fn oh_my_posh_config(config: &Value) -> Result<Option<String>, String> {
    let prompt = &config["prompt"];
    if prompt["enabled"] != true {
        return Ok(None);
    }
    let mut segments = entries(&prompt["segments"])
        .map(posh_segment)
        .collect::<Result<Vec<_>, _>>()?;
    if segments.is_empty() {
        return Err("Attivare almeno un segmento del prompt".into());
    }

    // A marker keeps the prompt visible when all conditional segments are hidden.
    segments.push(json!({"type": "text", "style": "plain", "template": ">"}));
    let config = json!({
        "version": 4,
        "final_space": true,
        "blocks": [{"type": "prompt", "alignment": "left", "segments": segments}]
    });
    serde_json::to_string_pretty(&config)
        .map(|output| Some(output + "\n"))
        .map_err(|error| error.to_string())
}

fn posh_segment(segment: &Value) -> Result<Value, String> {
    let id = string(segment, "id")?;
    let (kind, template, options) = match id {
        "directory" => match segment["pathStyle"].as_str().unwrap_or("short") {
            // `~\..\Desktop`
            "short" => ("path", "{{ .Path }} ", json!({"style": "agnoster_short"})),
            // `~\OneDrive\Desktop`
            "home" => ("path", "{{ .Path }} ", json!({"style": "full"})),
            // `C:\Users\anton\OneDrive\Desktop`: senza tilde, con le barre di Windows
            "full" => (
                "path",
                r#"{{ replace "/" "\\" .Location }} "#,
                json!({"style": "full"}),
            ),
            // `Desktop`
            "folder" => ("path", "{{ .Path }} ", json!({"style": "folder"})),
            other => return Err(format!("Stile del percorso sconosciuto: {other}")),
        },
        "git" => (
            "git",
            "{{ .HEAD }}{{ if .Working.Changed }} *{{ end }}{{ if .Staging.Changed }} +{{ end }}{{ if gt .Ahead 0 }} ahead:{{ .Ahead }}{{ end }}{{ if gt .Behind 0 }} behind:{{ .Behind }}{{ end }} ",
            json!({"branch_icon": ""}),
        ),
        "node" => (
            "node",
            "Node {{ .Full }} ",
            json!({"display_mode": "context"}),
        ),
        "dotnet" => (
            "dotnet",
            "{{ if .Unsupported }}.NET SDK missing {{ else }}.NET {{ .Full }} {{ end }}",
            json!({"display_mode": "context"}),
        ),
        "duration" => (
            "executiontime",
            "{{ .FormattedMs }} ",
            json!({"threshold": segment["thresholdMs"].as_u64().unwrap_or(2000), "style": "austin"}),
        ),
        "status" => (
            "status",
            "exit {{ .Code }} ",
            json!({"always_enabled": false}),
        ),
        _ => return Err(format!("Segmento del prompt sconosciuto: {id}")),
    };
    let color =
        segment["color"]
            .as_str()
            .unwrap_or(if id == "status" { "#F87171" } else { "#D8DEE9" });
    Ok(json!({
        "type": kind,
        "style": "plain",
        "foreground": color,
        "template": template,
        "options": options
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_prompt_has_no_artifact() {
        assert_eq!(oh_my_posh_config(&json!({})).unwrap(), None);
        assert_eq!(
            oh_my_posh_config(&json!({"prompt": {"enabled": false, "segments": [{"id": "git"}]}}))
                .unwrap(),
            None
        );
    }

    #[test]
    fn serializes_segment_order_colors_and_options() {
        let config = json!({"prompt": {"enabled": true, "segments": [
            {"id": "duration", "thresholdMs": 42, "color": "#AABBCC"},
            {"id": "git"},
            {"id": "directory"},
            {"id": "node"},
            {"id": "dotnet"},
            {"id": "status"}
        ]}});
        let theme: Value =
            serde_json::from_str(&oh_my_posh_config(&config).unwrap().unwrap()).unwrap();
        let segments = theme["blocks"][0]["segments"].as_array().unwrap();
        let kinds: Vec<_> = segments
            .iter()
            .map(|segment| segment["type"].as_str().unwrap())
            .collect();
        assert_eq!(
            kinds,
            [
                "executiontime",
                "git",
                "path",
                "node",
                "dotnet",
                "status",
                "text"
            ]
        );
        assert_eq!(segments[0]["foreground"], "#AABBCC");
        assert_eq!(segments[0]["options"]["threshold"], 42);
        assert_eq!(segments[5]["foreground"], "#F87171");
        assert_eq!(segments[5]["options"]["always_enabled"], false);
    }

    #[test]
    fn directory_path_style_controls_tilde_and_length() {
        let render = |style: Option<&str>| -> Value {
            let mut directory = json!({"id": "directory"});
            if let Some(style) = style {
                directory["pathStyle"] = json!(style);
            }
            let config = json!({"prompt": {"enabled": true, "segments": [directory]}});
            let theme: Value =
                serde_json::from_str(&oh_my_posh_config(&config).unwrap().unwrap()).unwrap();
            theme["blocks"][0]["segments"][0].clone()
        };
        let short = render(None);
        assert_eq!(short["options"]["style"], "agnoster_short");
        assert_eq!(render(Some("short")), short);
        let home = render(Some("home"));
        assert_eq!(home["options"]["style"], "full");
        assert_eq!(home["template"], "{{ .Path }} ");
        let full = render(Some("full"));
        assert_eq!(full["template"], r#"{{ replace "/" "\\" .Location }} "#);
        assert_eq!(render(Some("folder"))["options"]["style"], "folder");
        let bad = json!({"prompt": {"enabled": true, "segments": [{"id": "directory", "pathStyle": "x"}]}});
        assert!(oh_my_posh_config(&bad).is_err());
    }
}
