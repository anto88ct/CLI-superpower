use super::*;
use serde_json::json;

#[test]
fn documented_layers_merge() {
    let mut examples = include_str!("../../../../docs/config-schema.md")
        .split("```json\n")
        .skip(1)
        .map(|block| parse_json(block.split("```").next().unwrap()).unwrap());
    let team = examples.next().unwrap();
    let personal = examples.next().unwrap();
    let effective = resolve(&personal, Some(&team)).unwrap();
    assert_eq!(effective["commandPrefix"], "");
    assert_eq!(
        effective["appearance"]["pwsh"]["font"],
        json!({"face":"Cascadia Mono","size":14})
    );
    assert_eq!(effective["commands"].as_array().unwrap().len(), 1);
    assert_eq!(effective["commands"][0]["id"], "node-version");
    assert_eq!(effective["workspaces"][0]["id"], "efm");
    assert_eq!(effective["tools"]["pwsh"]["enabled"], true);
    assert_eq!(effective["tools"]["jq"]["enabled"], false);
    assert!(effective.get("team").is_none());
}

#[test]
fn mismatched_team_blocks_resolution() {
    let personal = json!({"schemaVersion":1,"team":{"packageId":"shared","installedVersion":2}});
    let team = json!({"schemaVersion":1,"id":"shared","version":1});
    assert!(resolve(&personal, None).unwrap_err().contains("team"));
    assert!(resolve(&personal, Some(&team))
        .unwrap_err()
        .contains("installedVersion"));
}

#[test]
fn validates_each_layer_before_merge() {
    let invalid = [
        json!({"schemaVersion":7,"appearance":null}),
        json!({"schemaVersion":1,"appearance":{"pwsh":{"opacity":null}}}),
        json!({"schemaVersion":1,"appearance":{"cmd":{"opacity":101}}}),
        json!({"schemaVersion":1,"appearance":{"cmd":{"opactiy":10}}}),
        json!({"schemaVersion":1,"commandPrefix":","}),
        json!({"schemaVersion":1,"prompt":{"enabled":true,"segments":[]}}),
        json!({"schemaVersion":1,"tools":{"fzf":{}}}),
    ];
    for config in invalid {
        assert!(resolve(&config, None).is_err(), "{config}");
    }
}

#[test]
fn rejects_duplicate_keys_in_raw_json() {
    let error = parse_json(r#"{"appearance":{"pwsh":{"opacity":10,"opacity":20}}}"#).unwrap_err();
    assert!(error.contains("duplicate key \"opacity\""));
    assert!(parse_json(r#"{"commands":[null]}"#).is_err());
    assert!(parse_json(r#"{"schemaVersion":1,}"#).is_err());
    assert!(parse_json(r#"{"schemaVersion":1} {}"#).is_err());
}

fn command(id: &str, name: &str, shell: &str) -> Value {
    json!({"id":id,"name":name,"shells":[shell],"template":{"program":"dotnet","args":[]}})
}

#[test]
fn applies_only_active_overrides_and_keeps_team_order() {
    let team = json!({
        "schemaVersion": 1, "id": "shared", "version": 2,
        "commands": [
            command("first", "first", "pwsh"),
            command("second", "second", "pwsh"),
            command("third", "third", "pwsh")
        ]
    });
    let personal = json!({
        "schemaVersion": 1,
        "team": {"packageId": "shared", "installedVersion": 2},
        "overrides": {
            "shared": {"commands": {
                "first": {"action": "replace", "value": command("first", "replacement", "pwsh")},
                "second": {"action": "disable"},
                "retired": {"action": "replace", "value": command("retired", "retired", "pwsh")}
            }},
            "inactive": {"commands": {"third": {"action": "disable"}}}
        },
        "commands": [command("own", "own", "pwsh")]
    });
    let result = resolve(&personal, Some(&team)).unwrap();
    let names: Vec<_> = result["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| &v["name"])
        .collect();
    assert_eq!(names, ["replacement", "third", "own"]);
}

#[test]
fn detects_id_conflicts_and_public_name_conflicts_per_shell() {
    let team = json!({
        "schemaVersion": 1, "id": "shared", "version": 1,
        "commands": [command("shared", "same-name", "pwsh")]
    });
    let mut personal = json!({
        "schemaVersion": 1,
        "team": {"packageId": "shared", "installedVersion": 1},
        "commandPrefix": "/",
        "commands": [command("personal", "same-name", "powershell")]
    });
    assert!(resolve(&personal, Some(&team)).is_ok());
    personal["commands"][0]["shells"] = json!(["pwsh"]);
    assert!(resolve(&personal, Some(&team))
        .unwrap_err()
        .contains("/same-name"));
    personal["commands"][0]["id"] = json!("shared");
    personal["overrides"]["shared"]["commands"]["shared"] = json!({"action": "disable"});
    assert!(resolve(&personal, Some(&team))
        .unwrap_err()
        .contains("personal id"));
}

#[test]
fn distinguishes_windows_absolute_paths_from_rooted_or_relative_paths() {
    for path in [
        r"C:\",
        r"c:/src",
        r"\\server\share\a",
        r"\\?\C:\directory",
        r"\\?\UNC\server\share",
    ] {
        assert!(
            resolve(
                &json!({"schemaVersion":1,"appearance":{"pwsh":{"backgroundImage":path}}}),
                None
            )
            .is_ok(),
            "{path}"
        );
    }
    for path in [
        r"C:src",
        r"\src",
        r"\\server",
        r"\\.\COM1",
        r"C:\foo|bar",
        "bad\0name",
    ] {
        assert!(
            resolve(
                &json!({"schemaVersion":1,"appearance":{"pwsh":{"backgroundImage":path}}}),
                None
            )
            .is_err(),
            "{path}"
        );
    }
}

#[test]
fn validates_template_parameter_usage_and_types() {
    let mut entry = command("echo", "echo", "pwsh");
    entry["template"]["args"] = json!(["{count}", "{{literal}}", "{count}"]);
    entry["parameters"] = json!([{"name":"count","type":"integer","required":false,"default":0}]);
    let mut personal = json!({"schemaVersion":1,"commands":[entry]});
    assert!(resolve(&personal, None).is_ok());
    personal["commands"][0]["parameters"][0]["default"] = json!("0");
    assert!(resolve(&personal, None).unwrap_err().contains("default"));
    personal["commands"][0]["parameters"][0]["default"] = json!(0);
    personal["commands"][0]["template"]["args"][0] = json!("prefix{count}");
    assert!(resolve(&personal, None)
        .unwrap_err()
        .contains("whole-argument"));
    personal["commands"][0]["template"]["args"] = json!(["{other}"]);
    assert!(resolve(&personal, None)
        .unwrap_err()
        .contains("unknown parameter"));
}

#[test]
fn resolves_prompt_fields_after_overlay() {
    let team = json!({
        "schemaVersion":1,"id":"shared","version":1,
        "prompt":{"enabled":true,"segments":[{"id":"duration"}]}
    });
    let mut personal = json!({
        "schemaVersion":1, "team":{"packageId":"shared","installedVersion":1},
        "prompt":{"enabled":true}
    });
    let effective = resolve(&personal, Some(&team)).unwrap();
    assert_eq!(effective["prompt"]["segments"][0]["thresholdMs"], 2000);
    personal["prompt"]["segments"] = json!([]);
    assert!(resolve(&personal, Some(&team))
        .unwrap_err()
        .contains("segments"));
    personal["prompt"]["enabled"] = json!(false);
    assert_eq!(
        resolve(&personal, Some(&team)).unwrap()["prompt"]["segments"],
        json!([])
    );
}

#[test]
fn migrates_v1_documents_and_rejects_unknown_versions() {
    let v1 = json!({"schemaVersion":1,"commandPrefix":"/"});
    assert_eq!(
        migrate(&v1).unwrap(),
        json!({"schemaVersion":6,"commandPrefix":"/"})
    );
    assert_eq!(
        migrate(&json!({"schemaVersion":5})).unwrap(),
        json!({"schemaVersion":6})
    );
    let v6 = json!({"schemaVersion":6});
    assert_eq!(migrate(&v6).unwrap(), v6);
    assert!(migrate(&json!({"schemaVersion":7})).is_err());
    assert!(migrate(&json!({})).is_err());
    assert_eq!(resolve(&v1, None).unwrap()["schemaVersion"], 6);
}

#[test]
fn change_directory_command_is_exclusive_with_template() {
    let dir = json!({"id":"cd","name":"progetti","shells":["pwsh","cmd"],"changeDirectory":r"C:\Progetti"});
    let mut personal = json!({"schemaVersion":2,"commands":[dir]});
    assert!(resolve(&personal, None).is_ok());
    personal["commands"][0]["template"] = json!({"program":"node","args":[]});
    assert!(resolve(&personal, None)
        .unwrap_err()
        .contains("changeDirectory"));
    personal["commands"][0]
        .as_object_mut()
        .unwrap()
        .remove("template");
    personal["commands"][0]["workingDirectory"] = json!(r"C:\Altro");
    assert!(resolve(&personal, None).is_err());
    personal["commands"][0]
        .as_object_mut()
        .unwrap()
        .remove("workingDirectory");
    personal["commands"][0]["changeDirectory"] = json!("relativo");
    assert!(resolve(&personal, None).is_err());
    let missing = json!({"schemaVersion":2,"commands":[{"id":"x","name":"x","shells":["pwsh"]}]});
    assert!(resolve(&missing, None).unwrap_err().contains("template"));
}

#[test]
fn validates_shell_behavior() {
    let ok = json!({"schemaVersion":3,"shellBehavior":{
        "pwsh":{"predictions":"history","predictionView":"list","hideSensitive":true,"historyExclude":["mio-segreto"]},
        "cmd":{"predictions":"off"}
    }});
    assert_eq!(
        resolve(&ok, None).unwrap()["shellBehavior"]["pwsh"]["predictionView"],
        "list"
    );
    for bad in [
        json!({"shellBehavior":{"zsh":{}}}),
        json!({"shellBehavior":{"pwsh":{"predictions":"maybe"}}}),
        json!({"shellBehavior":{"pwsh":{"unknown":true}}}),
        json!({"shellBehavior":{"pwsh":{"historyExclude":[""]}}}),
        json!({"shellBehavior":{"pwsh":{"historyExclude":["a\nb"]}}}),
    ] {
        let mut document = json!({"schemaVersion":3});
        document["shellBehavior"] = bad["shellBehavior"].clone();
        assert!(resolve(&document, None).is_err(), "{bad}");
    }
    let team = json!({"schemaVersion":3,"id":"t","version":1,
        "shellBehavior":{"pwsh":{"predictions":"history","predictionView":"inline"}}});
    let personal = json!({"schemaVersion":3,"team":{"packageId":"t","installedVersion":1},
        "shellBehavior":{"pwsh":{"predictionView":"list"}}});
    let effective = resolve(&personal, Some(&team)).unwrap();
    assert_eq!(effective["shellBehavior"]["pwsh"]["predictions"], "history");
    assert_eq!(effective["shellBehavior"]["pwsh"]["predictionView"], "list");
}

#[test]
fn banner_defaults_to_enabled_and_can_be_disabled() {
    let effective = resolve(&json!({"schemaVersion":4}), None).unwrap();
    assert_eq!(effective["banner"]["enabled"], true);
    let off = resolve(&json!({"schemaVersion":4,"banner":{"enabled":false}}), None).unwrap();
    assert_eq!(off["banner"]["enabled"], false);
    assert!(resolve(&json!({"schemaVersion":4,"banner":{"enabled":"no"}}), None).is_err());
    assert!(resolve(&json!({"schemaVersion":4,"banner":{"color":"red"}}), None).is_err());
    let team = json!({"schemaVersion":4,"id":"t","version":1,"banner":{"enabled":false}});
    let personal = json!({"schemaVersion":4,"team":{"packageId":"t","installedVersion":1}});
    assert_eq!(
        resolve(&personal, Some(&team)).unwrap()["banner"]["enabled"],
        false
    );
}

#[test]
fn validates_directory_path_style() {
    let ok = json!({"schemaVersion":6,"prompt":{"enabled":true,"segments":[{"id":"directory","pathStyle":"full"}]}});
    assert!(resolve(&ok, None).is_ok());
    let bad_value = json!({"schemaVersion":6,"prompt":{"enabled":true,"segments":[{"id":"directory","pathStyle":"long"}]}});
    assert!(resolve(&bad_value, None).is_err());
    let wrong_segment = json!({"schemaVersion":6,"prompt":{"enabled":true,"segments":[{"id":"git","pathStyle":"full"}]}});
    assert!(resolve(&wrong_segment, None)
        .unwrap_err()
        .contains("pathStyle"));
}
