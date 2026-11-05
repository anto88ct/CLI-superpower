//! Banner di avvio mostrato all'apertura di una shell interattiva.
use serde_json::Value;

const BANNER: &str = include_str!("banner.txt");

fn art() -> String {
    BANNER.replace("\r\n", "\n").trim_matches('\n').to_owned()
}

fn width() -> usize {
    art()
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0)
}

/// `banner.enabled` è `true` nella configurazione effettiva, dove vale per default.
pub fn banner_enabled(config: &Value) -> bool {
    config["banner"]["enabled"] == true
}

/// Frammento PowerShell: solo per sessioni interattive nella console, con output non
/// reindirizzato e una finestra abbastanza larga per il disegno.
pub(super) fn powershell(config: &Value) -> String {
    if !banner_enabled(config) {
        return String::new();
    }
    let width = width();
    let art = art();
    format!(
        r#"
# Banner di avvio
$__cspArgs = [Environment]::GetCommandLineArgs()
if ($Host.Name -eq 'ConsoleHost' -and -not [Console]::IsOutputRedirected -and -not ($__cspArgs | Where-Object {{ $_ -match '^-(c|command|f|file|e|encodedcommand)$' }})) {{
    try {{
        if ($Host.UI.RawUI.WindowSize.Width -ge {width}) {{
            Microsoft.PowerShell.Utility\Write-Host @'

{art}

'@
        }}
    }} catch {{ }}
}}
Remove-Variable -Name __cspArgs -ErrorAction SilentlyContinue
"#
    )
}

/// Frammento Lua per Clink: mostra il banner una volta, prima del primo prompt.
pub(super) fn lua(config: &Value) -> String {
    if !banner_enabled(config) {
        return String::new();
    }
    let width = width();
    let art = art();
    format!(
        "local banner_pending = true\nclink.onbeginedit(function()\n    if banner_pending then\n        banner_pending = false\n        if console.getwidth() >= {width} then\n            print(\"\\n\" .. [=[\n{art}\n]=])\n        end\n    end\nend)\n"
    )
}
