//! Comportamento di autocomplete e cronologia (`shellBehavior`), condiviso dai generatori.
use serde_json::Value;

/// Frammenti che indicano un dato sensibile. Il confronto ignora maiuscole e minuscole.
const SENSITIVE: &[&str] = &[
    "password",
    "passwd",
    "secret",
    "token",
    "apikey",
    "api_key",
    "api-key",
    "bearer ",
    "authorization:",
    "-asplaintext",
    "private_key",
    "connectionstring",
];

pub(super) struct Behavior<'a> {
    /// `history` o `off`; `None` lascia l'impostazione dell'utente.
    pub predictions: Option<&'a str>,
    /// `inline` o `list`; solo PowerShell.
    pub view: Option<&'a str>,
    /// Tab accetta il suggerimento in linea; con le predizioni disattivate non ha effetto.
    pub accept_with_tab: bool,
    /// Frammenti in minuscolo: le righe che li contengono non vanno nella cronologia su disco.
    pub patterns: Vec<String>,
}

pub(super) fn behavior<'a>(config: &'a Value, shell: &str) -> Option<Behavior<'a>> {
    let entry = config["shellBehavior"].get(shell)?.as_object()?;
    let mut patterns: Vec<String> = Vec::new();
    let mut add = |pattern: String| {
        if !pattern.is_empty() && !patterns.contains(&pattern) {
            patterns.push(pattern);
        }
    };
    if entry.get("hideSensitive").and_then(Value::as_bool) != Some(false) {
        SENSITIVE
            .iter()
            .for_each(|pattern| add((*pattern).to_owned()));
    }
    for pattern in entry
        .get("historyExclude")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        add(pattern.trim().to_lowercase());
    }
    Some(Behavior {
        predictions: entry.get("predictions").and_then(Value::as_str),
        view: entry.get("predictionView").and_then(Value::as_str),
        accept_with_tab: entry.get("acceptKey").and_then(Value::as_str) == Some("tab")
            && entry.get("predictions").and_then(Value::as_str) != Some("off"),
        patterns,
    })
}

/// Indica se la shell ha un comportamento di autocomplete da generare.
pub fn has_behavior(config: &Value, shell: &str) -> bool {
    behavior(config, shell).is_some()
}
