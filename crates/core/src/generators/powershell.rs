use super::banner;
use super::behavior::{behavior, Behavior};
use super::{entries, string};
use serde_json::Value;
use std::fmt::Write as _;

const NATIVE_HELPER: &str = include_str!("invoke-native.ps1");

pub fn powershell_init(config: &Value, shell: &str) -> Result<String, String> {
    if !matches!(shell, "pwsh" | "powershell") {
        return Err(format!("Shell non supportata: {shell}"));
    }
    let prefix = config["commandPrefix"].as_str().unwrap_or("");
    if !matches!(prefix, "" | "/" | "x-") {
        return Err(format!("Prefisso PowerShell non supportato: {prefix}"));
    }
    let mut output = String::from("# Generato da CLI Superpower.\n");
    let mut commands = entries(&config["commands"])
        .filter(|command| entries(&command["shells"]).any(|scope| scope == shell))
        .peekable();
    if commands.peek().is_some() {
        output.push_str(NATIVE_HELPER);
    }
    for command in commands {
        append_command(&mut output, prefix, command)?;
    }
    if let Some(behavior) = behavior(config, shell) {
        append_behavior(&mut output, &behavior);
    }
    output.push_str(&banner::powershell(config));
    Ok(output)
}

/// Configura PSReadLine solo se il modulo è caricato e abbastanza recente.
/// Ogni opzione ha il proprio controllo di versione, così Windows PowerShell 5.1
/// (PSReadLine 2.0) non genera errori all'avvio.
fn append_behavior(output: &mut String, behavior: &Behavior) {
    output.push_str(
        r"
# Autocomplete e cronologia (PSReadLine)
$__cspPsr = Microsoft.PowerShell.Core\Get-Module -Name PSReadLine
if ($Host.Name -eq 'ConsoleHost' -and $__cspPsr) {
    try {
",
    );
    if let Some(source) = behavior.predictions {
        let source = if source == "off" { "None" } else { "History" };
        writeln!(
            output,
            r"        if ($__cspPsr.Version -ge [version]'2.1.0') {{ PSReadLine\Set-PSReadLineOption -PredictionSource {source} }}"
        )
        .unwrap();
    }
    if let Some(view) = behavior.view {
        let view = if view == "list" {
            "ListView"
        } else {
            "InlineView"
        };
        writeln!(
            output,
            r"        if ($__cspPsr.Version -ge [version]'2.2.0') {{ PSReadLine\Set-PSReadLineOption -PredictionViewStyle {view} }}"
        )
        .unwrap();
    }
    if behavior.accept_with_tab {
        output.push_str(TAB_ACCEPT);
    }
    if !behavior.patterns.is_empty() {
        let list: Vec<_> = behavior.patterns.iter().map(|p| literal(p)).collect();
        writeln!(
            output,
            "        $global:__CspHistoryPatterns = @({})",
            list.join(", ")
        )
        .unwrap();
        output.push_str(HISTORY_FILTER);
    }
    output.push_str(
        "    } catch { }\n}\nRemove-Variable -Name __cspPsr -ErrorAction SilentlyContinue\n",
    );
}

/// Tab accetta il suggerimento in linea; se non cambia nulla esegue il completamento che Tab
/// aveva prima (memorizzato una sola volta, così ricaricare il profilo non lo perde).
/// `AcceptSuggestion` esiste da PSReadLine 2.1, la stessa versione delle predizioni.
const TAB_ACCEPT: &str = r#"        if ($__cspPsr.Version -ge [version]'2.1.0') {
            if (-not $global:__CspTabFallback) {
                $__cspTab = (PSReadLine\Get-PSReadLineKeyHandler -Bound | Where-Object { $_.Key -eq 'Tab' } | Select-Object -First 1).Function
                $global:__CspTabFallback = if ($__cspTab -and $__cspTab -ne 'CspAcceptSuggestion') { $__cspTab } else { 'TabCompleteNext' }
            }
            PSReadLine\Set-PSReadLineKeyHandler -Key Tab -BriefDescription 'CspAcceptSuggestion' -Description 'Accetta il suggerimento in linea, altrimenti completa' -ScriptBlock {
                $__cspLine = $null; $__cspCursor = $null
                [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState([ref]$__cspLine, [ref]$__cspCursor)
                [Microsoft.PowerShell.PSConsoleReadLine]::AcceptSuggestion()
                $__cspAfter = $null
                [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState([ref]$__cspAfter, [ref]$__cspCursor)
                if ($__cspAfter -ceq $__cspLine) {
                    $__cspMethod = [Microsoft.PowerShell.PSConsoleReadLine].GetMethod($global:__CspTabFallback, [type[]]@())
                    if (-not $__cspMethod) { $__cspMethod = [Microsoft.PowerShell.PSConsoleReadLine].GetMethod('TabCompleteNext', [type[]]@()) }
                    [void]$__cspMethod.Invoke($null, @())
                }
            }
        }
"#;

/// Le righe sensibili restano nella sessione ma non vengono scritte nel file della cronologia.
/// Un filtro già configurato dall'utente resta attivo e viene consultato per le altre righe.
const HISTORY_FILTER: &str = r#"        $__cspCurrent = (PSReadLine\Get-PSReadLineOption).AddToHistoryHandler
        # Il filtro dell'utente si memorizza una sola volta: se il profilo viene ricaricato non va concatenato a sé stesso.
        if (-not $global:__CspHistoryHandler) { $global:__CspPrevHistoryHandler = $__cspCurrent }
        $global:__CspHistoryHandler = {
            param([string]$line)
            foreach ($p in $global:__CspHistoryPatterns) {
                if ($line.IndexOf($p, [System.StringComparison]::OrdinalIgnoreCase) -ge 0) { return [Microsoft.PowerShell.AddToHistoryOption]::MemoryOnly }
            }
            $__cspPrev = $global:__CspPrevHistoryHandler
            if ($__cspPrev -is [scriptblock]) { return & $__cspPrev $line }
            if ($__cspPrev) { return $__cspPrev.Invoke($line) }
            return [Microsoft.PowerShell.AddToHistoryOption]::MemoryAndFile
        }
        PSReadLine\Set-PSReadLineOption -AddToHistoryHandler $global:__CspHistoryHandler
"#;

fn append_command(output: &mut String, prefix: &str, command: &Value) -> Result<(), String> {
    let name = string(command, "name")?;
    if !identifier(name, b'-') {
        return Err(format!("Nome comando PowerShell non valido: {name}"));
    }
    if prefix.is_empty() && KEYWORDS.contains(&name) {
        return Err(format!(
            "Il comando PowerShell richiede un prefisso: {name}"
        ));
    }
    writeln!(output, "\nfunction {prefix}{name} {{").unwrap();
    append_help(output, command, name);
    if let Some(directory) = command["changeDirectory"].as_str() {
        // Una funzione gira nella sessione corrente: Set-Location sposta la shell dell'utente.
        writeln!(
            output,
            "    Microsoft.PowerShell.Management\\Set-Location -LiteralPath {}\n}}",
            literal(directory)
        )
        .unwrap();
        return Ok(());
    }
    let parameters: Vec<_> = entries(&command["parameters"]).collect();
    let declarations = parameters
        .iter()
        .map(|parameter| declaration(parameter))
        .collect::<Result<Vec<_>, _>>()?;
    if !declarations.is_empty() {
        writeln!(output, "    param({})", declarations.join(", ")).unwrap();
    }
    for parameter in &parameters {
        if parameter["required"] == true {
            let name = string(parameter, "name")?;
            writeln!(
                output,
                "    if (!$PSBoundParameters.ContainsKey('{name}')) {{ throw 'Parametro obbligatorio: {name}' }}"
            )
            .unwrap();
        }
    }
    output.push_str("    $__cspArguments = @(\n");
    for argument in entries(&command["template"]["args"]) {
        let argument = argument
            .as_str()
            .ok_or("Argomento del template non valido")?;
        let parameter = argument
            .strip_prefix('{')
            .and_then(|s| s.strip_suffix('}'))
            .and_then(|name| {
                parameters
                    .iter()
                    .find(|parameter| parameter["name"] == name)
                    .map(|parameter| (name, *parameter))
            });
        match parameter {
            Some((name, parameter))
                if parameter["required"] == false && parameter.get("default").is_none() =>
            {
                writeln!(
                    output,
                    "        if ($PSBoundParameters.ContainsKey('{name}')) {{ [string]${name} }}"
                )
                .unwrap();
            }
            Some((name, _)) => writeln!(output, "        [string]${name}").unwrap(),
            None => writeln!(
                output,
                "        {}",
                literal(&argument.replace("{{", "{").replace("}}", "}"))
            )
            .unwrap(),
        }
    }
    let program = literal(string(&command["template"], "program")?);
    let directory = command["workingDirectory"]
        .as_str()
        .map(literal)
        .unwrap_or_else(|| "$null".into());
    writeln!(
        output,
        "    )\n    __CliSuperpowerInvokeNative -Program {program} -Values $__cspArguments -Directory {directory}\n}}"
    )
    .unwrap();
    Ok(())
}

const KEYWORDS: &[&str] = &[
    "begin",
    "break",
    "catch",
    "class",
    "clean",
    "configuration",
    "continue",
    "data",
    "do",
    "dynamicparam",
    "else",
    "elseif",
    "end",
    "enum",
    "exit",
    "filter",
    "finally",
    "for",
    "foreach",
    "function",
    "if",
    "in",
    "inlinescript",
    "parallel",
    "param",
    "process",
    "return",
    "sequence",
    "switch",
    "throw",
    "trap",
    "try",
    "until",
    "using",
    "while",
    "workflow",
];

fn declaration(parameter: &Value) -> Result<String, String> {
    let name = string(parameter, "name")?;
    let reserved = [
        "args",
        "error",
        "executioncontext",
        "false",
        "foreach",
        "home",
        "host",
        "input",
        "lastexitcode",
        "matches",
        "myinvocation",
        "nestedpromptlevel",
        "null",
        "ofs",
        "pid",
        "profile",
        "psboundparameters",
        "pscmdlet",
        "pscommandpath",
        "psculture",
        "psdebugcontext",
        "psedition",
        "pshome",
        "psitem",
        "psscriptroot",
        "psuiculture",
        "psversiontable",
        "pwd",
        "shellid",
        "stacktrace",
        "switch",
        "this",
        "true",
        "iswindows",
        "islinux",
        "ismacos",
        "iscoreclr",
    ];
    if !identifier(name, b'_') || reserved.contains(&name) {
        return Err(format!(
            "Nome parametro PowerShell riservato o non valido: {name}"
        ));
    }
    let kind = match string(parameter, "type")? {
        "string" => "string",
        "integer" => "int",
        _ => return Err(format!("Tipo parametro non valido: {name}")),
    };
    let mut declaration = format!("[{kind}]${name}");
    if let Some(default) = parameter.get("default") {
        let default = match kind {
            "string" => literal(default.as_str().ok_or("Default stringa non valido")?),
            _ => default
                .as_i64()
                .ok_or("Default intero non valido")?
                .to_string(),
        };
        write!(declaration, " = {default}").unwrap();
    }
    Ok(declaration)
}

fn append_help(output: &mut String, command: &Value, name: &str) {
    output.push_str("    # .SYNOPSIS\n");
    help_text(output, command["description"].as_str().unwrap_or(name));
    for parameter in entries(&command["parameters"]) {
        if let Some(name) = parameter["name"]
            .as_str()
            .filter(|name| identifier(name, b'_'))
        {
            writeln!(output, "    # .PARAMETER {name}").unwrap();
            help_text(output, parameter["description"].as_str().unwrap_or(name));
        }
    }
    output.push('\n');
}

fn help_text(output: &mut String, description: &str) {
    // PowerShell also recognizes U+0085 and the Unicode line separators.
    for line in description.split(['\r', '\n', '\u{0085}', '\u{2028}', '\u{2029}']) {
        writeln!(output, "    # {line}").unwrap();
    }
}

fn literal(value: &str) -> String {
    if value.bytes().all(|b| (32..=126).contains(&b)) {
        format!("'{}'", value.replace('\'', "''"))
    } else {
        // Smart quotes and line separators have special meanings in PowerShell.
        let units = value
            .encode_utf16()
            .map(|unit| unit.to_string())
            .collect::<Vec<_>>();
        format!("(-join [char[]]@({}))", units.join(","))
    }
}

fn identifier(value: &str, separator: u8) -> bool {
    let mut bytes = value.bytes();
    bytes.next().is_some_and(|first| first.is_ascii_lowercase())
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == separator)
}
