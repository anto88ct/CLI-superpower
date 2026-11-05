# Porting di CLI Superpower su macOS — Piano di implementazione

> **Stato:** bozza v0.1 · **Data:** 6 novembre 2026 · **Analisi di riferimento:** [`ANALISI.md`](ANALISI.md) · **Regole aggiuntive:** [`CLAUDE-macos.md`](CLAUDE-macos.md)

Piano pensato per essere eseguito da Claude Code un task alla volta. Ogni task è piccolo, ha file coinvolti, azioni concrete e un criterio di completamento verificabile. I task marcati **[condivisa con Linux]** toccano astrazioni comuni con il porting Linux (`docs/porting/linux/`): prima di eseguirli confrontare i due piani e fissare nomi e confini una volta sola.

## Come usare questo piano

1. Leggere `CLAUDE.md`, `CLAUDE-macos.md` e la sezione di `ANALISI.md` citata dal task.
2. Eseguire **un task per volta**, nell'ordine; non anticipare task di fasi successive.
3. Prima di dichiarare un task concluso eseguire le verifiche indicate e riportare l'esito (anche i fallimenti e i limiti, come chiede `CLAUDE.md`).
4. Dove un task dipende da uno spike (S*n*), leggere prima l'esito in `docs/spikes/macos.md`; se lo spike ha smentito l'ipotesi, aggiornare il task prima di eseguirlo.

### Dove si esegue

- **[Ovunque]**: il task si può svolgere e verificare anche su Windows (codice comune, generatori puri, UI).
- **[Mac]**: serve macOS (macchina locale o job `macos-latest` della CI). Se si lavora da Windows, la verifica è il job macOS della CI verde.

### Verifiche standard

**V-base** (le stesse della CI attuale, da `README.md`):

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

```bash
cd app && npm run build && npm test -- --watch=false
```

**V-mac** (su macOS, oltre a V-base):

```bash
cd app && npm run tauri -- build --no-bundle
```

**V-snap**: i nuovi snapshot `insta` sono stati creati, letti e accettati (`cargo insta review` o file `.snap` controllati a mano); nessuno snapshot preesistente è cambiato salvo dove il task lo prevede.

### Dipendenze tra fasi

```
M0 ─► M1 ─► M2 ─► M3 ─► M4 ─► M5 ─┬─► M6 ─► M7
                                  ├─► M8
                                  └─► M10
M2 ─► M9 (csp, indipendente dal resto)
M5..M10 ─► M11 ─► M12
```

---

## M0 — Base: il progetto compila e si testa su macOS

Obiettivo: CI verde su Windows e macOS senza cambiare il comportamento su Windows.

### M0.1 — `fetch-clink` solo su Windows [Ovunque]
- **File:** `app/src-tauri/tauri.conf.json`, nuovo `app/src-tauri/tauri.windows.conf.json`, `app/package.json`.
- **Azioni:**
  1. Spostare in `tauri.windows.conf.json` (configurazione per piattaforma di Tauri 2, fusa sopra quella base) `build.beforeDevCommand` e `build.beforeBuildCommand` che chiamano `npm run fetch-clink`, e `bundle.resources` con `resources/clink/`.
  2. Nella configurazione base lasciare `npm run start` / `npm run build`.
  3. Non cambiare `scripts/fetch-clink.ps1`.
- **Fatto quando:** V-base verde su Windows; `npm run tauri -- build --no-bundle` su Windows scarica ancora Clink se manca (cancellare `app/src-tauri/resources/clink` e verificare che ricompaia); la configurazione base non contiene più riferimenti a PowerShell.

### M0.2 — Backend Tauri senza dipendenze Windows [Ovunque]
- **File:** `app/src-tauri/src/lib.rs`, `app/src-tauri/Cargo.toml`.
- **Azioni:**
  1. `open_link`: sostituire `rundll32` con `tauri-plugin-opener` (lato Rust, `OpenerExt::opener().open_url`), mantenendo la lista chiusa di URL consentiti.
  2. In `run`, chiamare `with_clink_bundle` solo con `#[cfg(windows)]`.
- **Fatto quando:** V-base verde; su Windows i pulsanti GitHub e Sito della UI aprono ancora il browser (prova manuale riportata).

### M0.3 — Job CI macOS [Mac] [condivisa con Linux]
- **File:** `.github/workflows/ci.yml`.
- **Azioni:**
  1. Aggiungere un job `macos` su `macos-latest` con gli stessi passi del job `windows` tranne `npm run fetch-clink`.
  2. Valutare con la sessione Linux una matrice unica (`windows-latest`, `macos-latest`, `ubuntu-latest`).
- **Fatto quando:** entrambi i job sono verdi. Se `cargo test` fallisce su macOS, correggere solo con `cfg` mirati e annotare la causa nel riepilogo del task (atteso: nessun fallimento, perché `tools.rs::platform`, `csp::run_port`, `clink.rs::hide_window` e `desktop.rs::shell_profile` hanno già varianti non-Windows).

---

## M1 — Spike macOS

Obiettivo: confermare le ipotesi marcate "(da verificare)" in `ANALISI.md` prima di scrivere codice di prodotto. Output unico: **`docs/spikes/macos.md`**, con tabelle "Ambiente / Prova / Risultato" come `docs/spikes/fase-0.md`, versione di macOS e degli strumenti, ed eventuali script di prova in `docs/spikes/`.

Ogni spike è un task [Mac]. **Fatto quando:** la sezione dello spike in `docs/spikes/macos.md` riporta prove ripetibili, risultato e decisione; le righe corrispondenti di `ANALISI.md` sono aggiornate (tolto "(da verificare)", corretto se smentito).

| ID | Domanda | Prova minima |
|---|---|---|
| S1 | Il prefisso `/` funziona in zsh, bash 3.2, bash 5, fish? E `x-`? | Definire `function /csp-spike { print -r -- "$1" }` (e varianti per bash/fish), invocarla con `"two words"`; ripetere con alias. |
| S2 | iTerm2 Dynamic Profiles: chiavi esatte (`Guid`, `Name`, `Dynamic Profile Parent Name`/`Parent GUID`, `Custom Command`, `Command`, `Normal Font`, `Transparency`, `Blur`, `Blur Radius`, `Background Image Location`, `Blend`, `Background Image Mode`, `Cursor Type`, margini, colori); codici numerici; ricarica a caldo; cosa succede alle modifiche manuali; chiave e semantica di `Default Bookmark Guid`; rimozione del file. | Esportare un profilo da iTerm2 in JSON, ridurlo, salvarlo in `DynamicProfiles`, osservare; `defaults read com.googlecode.iterm2 "Default Bookmark Guid"`. |
| S3 | Terminal.app: si possono aggiungere/rimuovere profili in `com.apple.Terminal` → `Window Settings` con Terminal aperto senza che li sovrascriva? Formato `NSKeyedArchiver` di colori e font; chiave della sfocatura; chiavi `Default Window Settings`/`Startup Window Settings`; alternativa import di un file `.terminal`. | Scrittura con `defaults import`/CFPreferences da un piccolo programma, Terminal aperto e chiuso; confronto con un profilo creato a mano ed esportato. |
| S4 | AppleScript verso iTerm2 e Terminal.app da un'app firmata con hardened runtime: entitlement `com.apple.security.automation.apple-events`, `NSAppleEventsUsageDescription`, a chi viene attribuita la richiesta TCC quando lo script è eseguito da `/usr/bin/osascript` figlio dell'app; codice d'errore con permesso negato. | App Tauri minima firmata ad-hoc che lancia lo script del §4.6 di `ANALISI.md`. |
| S5 | PATH di un'app avviata dal Dock; output di `/usr/libexec/path_helper -s`; costo e affidabilità di `zsh -lc 'print -r -- $PATH'`; nomi esatti di formule e cask del catalogo (§4.7); cask che chiedono la password; `brew` x86 sotto Rosetta. | App minima che stampa `PATH`; `brew info` per ogni pacchetto. |
| S6 | `oh-my-posh init <shell> --config <file>` per zsh, bash, fish e pwsh: funzionamento, durata dei comandi in bash 3.2; convivenza con oh-my-zsh, powerlevel10k (instant prompt e output all'avvio), starship. | Profili di prova con `ZDOTDIR` temporanea. |
| S7 | Cronologia: `zshaddhistory` (anche via `add-zsh-hook`) con ritorno `2`; fish `fish_should_add_to_history` e `fish_autosuggestion_enabled` (versioni minime); `commandline --showing-suggestion`; bash `HISTIGNORE` con pattern a classi di caratteri. | Sessioni interattive con `HISTFILE` temporaneo; verifica del file dopo l'uscita. |
| S8 | zsh-autosuggestions: widget `autosuggest-accept`, `$POSTDISPLAY`, disattivazione se già caricato da un plugin manager, doppio caricamento, binding di Tab precedente (`bindkey '^I'`). | Come S7. |
| S9 | `libproc` (crate): elenco PID, descrittori socket, stato `LISTEN`, IPv4/IPv6, `pidpath`, ora di avvio; visibilità di processi di altri utenti senza root; confronto con `lsof`. | Programma Rust minimo con `TcpListener` di prova. |
| S10 | Build universal: `tauri build --target universal-apple-darwin`, sidecar `csp` (`externalBin`, nome `csp-universal-apple-darwin` creato con `lipo`), firma ad-hoc, `createUpdaterArtifacts`, struttura di `latest.json` prodotta da `tauri-action`. | Ramo di prova, artefatti ispezionati con `lipo -info` e `codesign -dv`. |
| S11 | Dotfile: `~/.zshrc` symlink (stow/chezmoi), permessi `0600`, attributi estesi; cosa fa `atomic_write` oggi. | Test manuale con `apply::atomic_write` su file e link in una cartella temporanea. |

---

## M2 — Rifattorizzazione cross-platform senza cambi di comportamento [condivisa con Linux]

Obiettivo: separare scoperta, generazione e pianificazione (`ANALISI.md` §7) mantenendo **byte per byte** l'output Windows. Tutti i task sono [Ovunque] salvo dove indicato.

### M2.1 — Snapshot dell'output Windows attuale
- **File:** `crates/core/Cargo.toml` (dev-dependency `insta`), nuovo `crates/core/tests/fixtures/` (configurazioni effettive rappresentative), nuovo `crates/core/tests/windows_snapshots.rs`.
- **Azioni:** snapshot di `powershell_init` (pwsh e powershell), `clink_script_with_theme`, `terminal_fragment`, `workspace_args`, `oh_my_posh_config`, e del blocco prodotto da `profile_block` (esporre la funzione come `pub(crate)` o testarla dentro il modulo) su 3–4 fixture (vuota, solo comandi, tutto attivo, caratteri speciali).
- **Fatto quando:** V-base + V-snap; gli snapshot riflettono l'output corrente (nessuna modifica al codice di produzione).

### M2.2 — `PlatformPaths`
- **File:** nuovo `crates/core/src/platform.rs`, `crates/core/src/lib.rs`, `crates/core/src/desktop.rs`, `app/src-tauri/src/lib.rs`.
- **Azioni:**
  1. `pub struct PlatformPaths { pub home: PathBuf, pub data: PathBuf, pub state: PathBuf, pub system_local: Option<PathBuf> }` con `generated()` = `state/generated`.
  2. `Desktop::new` riceve `PlatformPaths` (i campi `data`, `local`, `system_local` diventano accessi a `paths`).
  3. `lib.rs::run` costruisce `PlatformPaths` da `data_dir`, `local_data_dir`, `home_dir`.
  4. Aggiornare il test `preview_apply_and_remove_integrate_without_touching_live_terminal`.
- **Fatto quando:** V-base; snapshot di M2.1 invariati.

### M2.3 — Modulo `shells` e blocco rc generico
- **File:** nuovo `crates/core/src/shells/mod.rs`, nuovo `crates/core/src/shells/powershell.rs`, `crates/core/src/desktop.rs`.
- **Azioni:**
  1. Spostare `shell_profile`, `ShellProfile`, `profile_block`, `update_profile_text`, `START`, `END`, `OH_MY_POSH_INIT` in `shells/powershell.rs`.
  2. In `shells/mod.rs` una funzione pura `rc_block(text: &str, block_lines: &[String], newline: Newline) -> Result<String, String>` con la logica dei marcatori di `update_profile_text`; `Newline::{Detect, Lf}` (`Detect` = comportamento attuale per PowerShell).
  3. Il blocco PowerShell resta identico (stesso testo con `$env:LOCALAPPDATA`).
  4. Definire `trait ShellIntegration` (`ANALISI.md` §7.2) e implementarlo per PowerShell; `Desktop::plan` itera sulle integrazioni registrate.
- **Fatto quando:** V-base; snapshot invariati; il test `profile_block_preserves_bom_and_user_lines_and_is_idempotent` passa senza modifiche alle asserzioni.

### M2.4 — Modulo `terminals` con Windows Terminal
- **File:** nuovo `crates/core/src/terminals/mod.rs`, nuovo `crates/core/src/terminals/windows_terminal.rs`, `crates/core/src/desktop.rs`, `crates/core/src/generators.rs`.
- **Azioni:**
  1. Spostare `TerminalSettings`, `profile_list`, `profile_shell`, `Desktop::profiles` e il ramo fragment/immagini/`defaultProfile`/anteprime workspace di `Desktop::plan` in `windows_terminal.rs`.
  2. Definire `trait TerminalIntegration` con `discover`, `capabilities`, `plan`, `launch_workspace`; `Desktop::launch` e `Desktop::terminal_profiles` delegano.
  3. `generators::terminal_fragment`, `terminal_images`, `workspace_args` restano generatori puri (eventualmente in `generators/windows_terminal.rs`).
- **Fatto quando:** V-base; snapshot invariati; test `terminal_discovery_accepts_jsonc_and_reports_overrides` e `preview_apply_and_remove_…` verdi.

### M2.5 — `PackageManager` e catalogo per gestore
- **File:** `crates/core/src/tools.rs` → `crates/core/src/packages/{mod.rs, winget.rs}` (o mantenere `tools.rs` come facciata), `crates/core/src/desktop.rs`.
- **Azioni:**
  1. `Tool { id, name, executables: &[(Os, &[&str])] o per OS, packages: &[PackageRef] , manual_urls per OS }` con `PackageRef::{Winget(&str), Brew { name, cask }, …}`.
  2. `trait PackageManager` (`locate`, `install`, `uninstall`); winget spostato da `tools.rs::platform` (cfg windows).
  3. `statuses`, `install`, `uninstall`, `install_recommended` usano il gestore della piattaforma; `STATE_FILE` invariato.
  4. JSON di `ToolStatus` invariato (test `reports_camel_case_fields`).
- **Fatto quando:** V-base; tutti i test di `tools.rs` passano invariati; su Windows la pagina Tool mostra gli stessi stati (prova manuale).

### M2.6 — Applicatore POSIX: permessi e symlink [Ovunque, test su Mac]
- **File:** `crates/core/src/apply.rs`.
- **Azioni:**
  1. In `atomic_write`, con `#[cfg(unix)]`, se il file di destinazione esiste copiare il suo modo (`PermissionsExt`) sul temporaneo prima del `rename`.
  2. Aggiungere `pub fn resolve_user_file(path) -> Result<PathBuf, String>` che segue un symlink fino al file reale (decisione D5 di `ANALISI.md`; se l'utente sceglie "rifiutare", restituisce un errore chiaro). Da usare nei planner per i file utente, non in `read`.
  3. Test `#[cfg(unix)]`: file `0600` resta `0600` dopo `apply`; file raggiunto via symlink viene modificato nel target e il link resta link; `remove` ripristina byte per byte.
- **Fatto quando:** V-base su Windows; job macOS verde con i nuovi test.

### M2.7 — `csp port` diviso in comune e piattaforma
- **File:** `crates/csp/src/main.rs`, `crates/csp/src/port.rs` → `crates/csp/src/port/{mod.rs, windows.rs}`, `crates/csp/src/tests.rs`.
- **Azioni:** in `port/mod.rs` il flusso (`run`, `confirmed_pid`, età del processo, stampa) e l'interfaccia `listeners(port)`, `process_info(pid)`, `terminate(port, pid, created)`; in `windows.rs` il codice `windows-sys` attuale.
- **Fatto quando:** V-base; i test `mod windows` di `tests.rs` passano invariati; output di `csp port` identico (prova manuale con `python -m http.server`).

---

## M3 — Schema v7 "cross-platform" [condivisa con Linux]

Da eseguire **una sola volta** per macOS e Linux. Regola di `CLAUDE.md`: incremento di `schemaVersion`, migrazione, test.

### M3.1 — Core: versione 7 [Ovunque]
- **File:** `crates/core/src/config.rs`, `crates/core/src/config/validate.rs`, `crates/core/src/config/tests.rs`.
- **Azioni:**
  1. `SCHEMA_VERSION = 7`; in `migrate` il ramo `1..=6` incrementa; commento "v6 → v7: valori di shell POSIX, percorsi POSIX, `globalTerminal.terminals`, nuovi tool".
  2. `version_field` usa `SCHEMA_VERSION` invece del `6` cablato.
  3. `PROFILES` += `zsh`, `bash`, `fish`.
  4. `windows_path` → `absolute_path`: accetta i percorsi Windows di oggi **oppure** POSIX assoluti (`/` iniziale, componenti senza NUL né caratteri di controllo); `program` riconosce come percorso anche `/…`. Messaggio "expected an absolute Windows or POSIX path".
  5. `global_terminal`: campo facoltativo `terminals` (array non vuoto di valori distinti fra gli ID di terminale concordati con Linux).
  6. `TOOLS` += ID concordati (almeno `zsh-autosuggestions`, `fish`, `bash`).
  7. Test: documento v6 migrato a 7 e valido; v7 con `zsh`/percorsi POSIX valido; percorsi relativi, `~/x` e vuoti rifiutati; `terminals` duplicati rifiutati; versione 8 rifiutata.
- **Fatto quando:** V-base; aggiornati i test esistenti che confrontano `"schemaVersion":6` (`desktop.rs`, `config/tests.rs`).

### M3.2 — Windows ignora ciò che non è suo [Ovunque]
- **File:** `crates/core/src/terminals/windows_terminal.rs`, `crates/core/src/shells/powershell.rs`, `crates/core/src/desktop.rs`, generatori.
- **Azioni:**
  1. Aspetto per `zsh`/`bash`/`fish`: escluso dal fragment con anteprima "valido su macOS/Linux" (oggi il ramo in `plan` etichetterebbe ogni profilo sconosciuto come "Prompt dei comandi").
  2. Workspace con pannelli su shell non Windows: non generati, anteprima esplicativa, **senza** far fallire l'intero `plan` (oggi `workspace_args` → `profile_id` restituirebbe errore).
  3. Comandi con percorsi POSIX su Windows: non generati, anteprima.
- **Fatto quando:** V-base; nuovo test con un pacchetto team misto Windows+macOS: anteprima ok, file Windows generati come prima, note per gli elementi esclusi.

### M3.3 — Documentazione dello schema [Ovunque]
- **File:** `docs/config-schema.md`.
- **Azioni:** `schemaVersion: 7`, nuovi valori, regola "validazione comune, generazione per OS", percorsi POSIX, `globalTerminal.terminals`, nuovi tool, nota sulla migrazione 6→7.
- **Fatto quando:** il documento è coerente con `validate.rs` (rileggere campo per campo).

### M3.4 — Tipi e validazione UI [Ovunque]
- **File:** `app/src/app/models.ts`, `app/src/app/config-validation.ts`, `app/src/app/config-bridge.ts`, spec relativi.
- **Azioni:** `Profile`/`Shell` estesi; `schemaVersion: 1|…|7`; `absolutePath` accetta POSIX; `pathHelp` per OS (parametro); mock `previewTeam` accetta 1–7.
- **Fatto quando:** V-base (parte Angular); nuovi test Vitest per percorsi POSIX e versione 7.

---

## M4 — Generatori shell POSIX (puri) [condivisa con Linux]

Tutti [Ovunque] per lo sviluppo e gli snapshot; la verifica di sintassi gira sul job macOS. I generatori non contengono `cfg`.

### M4.1 — Quoting
- **File:** nuovo `crates/core/src/generators/quote.rs`.
- **Azioni:** `sh_single(&str) -> Result<String, String>` (`'` → `'\''`, NUL = errore), `fish_single`, `applescript_string` (`\` e `"`). Test su apici, spazi, newline, Unicode, stringa vuota.
- **Fatto quando:** V-base; test unitari completi.

### M4.2 — Blocco rc POSIX
- **File:** `crates/core/src/shells/mod.rs`.
- **Azioni:** `posix_rc_block(init_path: &Path, guard: Option<&str>) -> Vec<String>` (righe del blocco di `ANALISI.md` §4.2) usato con `rc_block(…, Newline::Lf)`. Test: file vuoto, file senza newline finale, idempotenza, marcatori ambigui → errore, percorso con spazio e apice.
- **Fatto quando:** V-base + V-snap.

### M4.3 — `zsh_init`: comandi rapidi
- **File:** nuovo `crates/core/src/generators/zsh.rs`, `crates/core/src/generators.rs`.
- **Azioni:** funzioni per `template` (programma con `command --`, parametri posizionali, obbligatori con messaggio d'uso, facoltativi senza default saltati, `integer` validato in i32, `{{ }}` letterali), `workingDirectory` in subshell, `changeDirectory` con `cd --`; nomi riservati; prefisso consentito solo se S1 lo conferma (altrimenti errore come `powershell_init`).
- **Fatto quando:** V-base + V-snap; test di generazione sui casi dei test PowerShell esistenti (stessa fixture).

### M4.4 — `zsh_init`: comportamento, banner, prompt
- **File:** `crates/core/src/generators/zsh.rs`, `crates/core/src/generators/banner.rs`, `crates/core/src/generators/behavior.rs` (solo se servono campi nuovi).
- **Azioni:** caricamento condizionale di zsh-autosuggestions (percorsi Homebrew noti nello script, guardia "già caricato"), `predictions: off`, Tab che accetta con fallback memorizzato una volta, hook `zshaddhistory` con ritorno 2 e confronto letterale insensibile alle maiuscole, banner al primo `precmd` solo se interattivo, `-t 1` e `$COLUMNS` ≥ larghezza, `eval "$(oh-my-posh init zsh --config …)"` se il prompt è attivo. Tutto secondo gli esiti di S6–S8.
- **Fatto quando:** V-base + V-snap; sul job macOS un test `#[cfg(unix)]` scrive lo script in una cartella temporanea ed esegue `zsh -n` con successo.

### M4.5 — `bash_init` (compatibile con bash 3.2)
- **File:** nuovo `crates/core/src/generators/bash.rs`.
- **Azioni:** come M4.3/M4.4 con sintassi bash 3.2 (niente array associativi, niente `${x,,}`, niente `mapfile`), guardia `__CSP_INIT_LOADED`, `HISTIGNORE` con pattern a classi, banner con `PROMPT_COMMAND` una volta, `oh-my-posh init bash`. Predizioni: solo se ble.sh è presente (decisione D4).
- **Fatto quando:** V-base + V-snap; `/bin/bash -n` e `bash -n` (Homebrew) verdi sul job macOS.

### M4.6 — `fish_init`
- **File:** nuovo `crates/core/src/generators/fish.rs`.
- **Azioni:** funzioni con `$argv`, `fish_should_add_to_history`, `fish_autosuggestion_enabled`, binding Tab, banner con `fish_greeting` locale o evento `fish_prompt` una volta, `oh-my-posh init fish | source`. Nomi con prefisso solo se S1 lo consente.
- **Fatto quando:** V-base + V-snap; `fish --no-execute` verde sul job macOS (installare fish con `brew` nel job).

### M4.7 — Prompt e PowerShell portabili
- **File:** `crates/core/src/generators/prompt.rs`, `crates/core/src/generators/powershell.rs`, `crates/core/src/generators/invoke-native.ps1` (o un secondo helper `invoke-native-unix.ps1`), `crates/core/src/shells/powershell.rs`.
- **Azioni:**
  1. `oh_my_posh_config(config, flavor: PathFlavor)`: lo stile `full` sostituisce `/` con `\` solo per `PathFlavor::Windows`.
  2. `powershell_init(config, shell, flavor)`: su Unix helper con `ProcessStartInfo.ArgumentList`, senza ramo `.cmd/.bat` né `[Environment]::SystemDirectory`.
  3. `OH_MY_POSH_INIT` cerca `oh-my-posh` senza `.exe` su Unix; `shell_profile` usa `pwsh` senza `.exe` e salta `Get-ExecutionPolicy` su Unix.
- **Fatto quando:** V-base; snapshot Windows **invariati**; nuovi snapshot per la variante Unix.

---

## M5 — Integrazione delle shell su macOS (I/O)

### M5.1 — PATH dell'app GUI [Mac] [condivisa con Linux]
- **File:** `crates/core/src/platform.rs`.
- **Azioni:** `SearchPath::discover()` = output di `path_helper -s` (parser puro testato su stringhe) + `/opt/homebrew/bin`, `/usr/local/bin`, `~/.local/bin`, `~/.cargo/bin` + (best-effort, timeout 3 s) `zsh -lc 'print -r -- $PATH'`; `which(name)`; passaggio esplicito a ogni `Command` del core (shell, brew, osascript).
- **Fatto quando:** V-base; test del parser; sul job macOS `which("zsh")` trova `/bin/zsh`.

### M5.2 — Scoperta delle shell [Mac]
- **File:** `crates/core/src/shells/{zsh.rs, bash.rs, fish.rs, powershell.rs}`.
- **Azioni:** `ShellFacts { executable, version, rc_files: Vec<PathBuf>, login_shell: bool, collisions, notes }`:
  - zsh: `zsh -c 'print -r -- ${ZDOTDIR:-$HOME}'` → `.zshrc`; rilevamento (lettura, senza modifica) di oh-my-zsh, p10k, starship, zsh-autosuggestions.
  - bash: file di login secondo la regola di `ANALISI.md` §4.2 + `.bashrc`.
  - fish: `fish -c 'echo $__fish_config_dir'` → `conf.d/cli-superpower.fish`.
  - pwsh: `shell_profile` adattato (M4.7).
  - shell di login dell'utente da `$SHELL` o `dscl . -read /Users/$USER UserShell`.
  - collisioni senza file utente (`zsh -f`, `bash --noprofile --norc`, `fish --no-config`).
- **Fatto quando:** V-base; test di integrazione sul job macOS con `HOME` e `ZDOTDIR` temporanee.

### M5.3 — Pianificazione delle shell [Ovunque, test su Mac]
- **File:** `crates/core/src/shells/*.rs`, `crates/core/src/desktop.rs`.
- **Azioni:** per ogni shell con contenuto: file generato `generated/init.<shell>` (posseduto), blocco rc nei file utente (`shared_input` come per `$PROFILE`, file risolto con `resolve_user_file`), per fish solo il file in `conf.d` (posseduto, niente marcatori). Anteprime: file toccati, conflitti rilevati, shell non installata, prefisso non supportato.
- **Fatto quando:** V-base; test con `PlatformPaths` temporanei: apply → file attesi; secondo apply → nessuna transazione nuova in `history.json`; modifica manuale del blocco → conflitto; `remove` → file utente identici all'originale byte per byte.

### M5.4 — Test di comportamento delle shell [Mac]
- **File:** nuovo `crates/core/tests/macos_shells.rs` (`#![cfg(target_os = "macos")]`).
- **Azioni:** con HOME/ZDOTDIR temporanee e lo script generato caricato: comando rapido con parametri esegue il programma con gli argomenti esatti (programma di prova che stampa argv), `changeDirectory` cambia `$PWD`, `workingDirectory` non lo cambia, riga con `password` non finisce in `HISTFILE`, banner assente con output non terminale. zsh, `/bin/bash`, fish.
- **Fatto quando:** job macOS verde.

### M5.5 — Comandi Tauri per la piattaforma [Ovunque]
- **File:** `app/src-tauri/src/lib.rs`, `crates/core/src/desktop.rs`.
- **Azioni:** comando `platform_info` (`ANALISI.md` §7.4); `terminal_profiles` su macOS restituisce le shell utilizzabili.
- **Fatto quando:** V-base; su Windows `platform_info` restituisce `os: "windows"` e le tre shell attuali.

---

## M6 — Terminali macOS

### M6.1 — Catalogo degli schemi colore [Ovunque] [condivisa con Linux]
- **File:** nuovo `crates/core/src/color_schemes.rs`.
- **Azioni:** schemi con 16 colori ANSI + fondo, testo, cursore, selezione per almeno i nomi già offerti dalla UI (`app.ts::appearanceThemes`: Campbell, Campbell Powershell, One Half Dark/Light, Solarized Dark/Light, Tango Dark/Light) con i valori di Windows Terminal.
- **Fatto quando:** V-base; test: ogni schema ha tutti i colori in formato `#RRGGBB`; i nomi coincidono con quelli della UI.

### M6.2 — Generatore iTerm2 [Ovunque]
- **File:** nuovo `crates/core/src/generators/iterm2.rs`, `crates/core/Cargo.toml` (es. `uuid` con feature `v5`, se si sceglie il crate).
- **Azioni:** `iterm2_dynamic_profiles(config, shells: &[ShellLaunch], parent: Option<&str>) -> Result<(String, Vec<Warning>), String>`: un profilo per shell con aspetto o usata da un workspace, GUID UUID v5 deterministico, mappatura e avvisi di `ANALISI.md` §4.1 (chiavi confermate da S2).
- **Fatto quando:** V-base + V-snap; test sulle approssimazioni (cursore, allineamento, padding) e sulla stabilità del GUID.

### M6.3 — Integrazione iTerm2 [Mac]
- **File:** nuovo `crates/core/src/terminals/iterm2.rs`, `crates/core/src/desktop.rs`.
- **Azioni:** scoperta (bundle in `/Applications` o `~/Applications`, cartella `DynamicProfiles`, nome del profilo predefinito da usare come genitore), `capabilities`, piano con file posseduto `DynamicProfiles/cli-superpower.json` e immagini referenziate da `…/CliSuperpower/backgrounds`.
- **Fatto quando:** V-base; test con HOME temporanea (apply/idempotenza/remove); prova manuale riportata: il profilo compare in iTerm2 senza riavvio e sparisce dopo "Disinstalla integrazione".

### M6.4 — Applicatore: chiavi di preferenza macOS [Mac]
- **File:** `crates/core/src/apply.rs`.
- **Azioni:**
  1. `PlannedChange::{File(PlannedFile), Preference { domain, key, value: Option<plist> }}`; journal, history e manifest estesi in modo retrocompatibile (`#[serde(default)]`, i file esistenti si leggono ancora).
  2. Esecuzione tramite `/usr/bin/defaults` (scrittura tipizzata o `import` del dominio) — mai sul file `.plist`.
  3. Conflitto se il valore attuale non è quello registrato; ripristino del valore originale o rimozione della chiave.
  4. Test su un dominio temporaneo `it.clisuperpower.test.<pid>` ripulito a fine test.
- **Fatto quando:** V-base su Windows (nessun cambiamento di comportamento); job macOS verde con i nuovi test; journal di una versione precedente ancora recuperabile (test con fixture).

### M6.5 — Profilo predefinito di iTerm2 [Mac]
- **File:** `crates/core/src/terminals/iterm2.rs`.
- **Azioni:** `globalTerminal.defaultProfile` → `Preference { domain: "com.googlecode.iterm2", key: "Default Bookmark Guid" }` con il GUID del profilo dinamico; anteprima "il profilo predefinito di iTerm2 diventerà CLI Superpower · zsh".
- **Fatto quando:** test d'integrazione su dominio di prova; prova manuale: nuova finestra iTerm2 usa il profilo; `remove` ripristina il precedente.

### M6.6 — Terminal.app [Mac] (dipende da S3)
- **File:** nuovo `crates/core/src/terminals/apple_terminal.rs`, nuovo `crates/core/src/generators/apple_terminal.rs`.
- **Azioni (se S3 positivo):** generatore puro di profili (`NSKeyedArchiver` per colori e font con il crate `plist`), piano con `Preference` su `com.apple.Terminal` (`Window Settings` filtrato alle sole voci nostre, `Default Window Settings`), capacità ridotte, avviso se Terminal è aperto secondo l'esito di S3.
- **Azioni (se S3 negativo):** solo scoperta e capacità "aspetto non gestito", con messaggio nella UI.
- **Fatto quando:** V-base + V-snap; prova manuale riportata.

---

## M7 — Workspace su macOS

### M7.1 — Generatore AppleScript [Ovunque]
- **File:** nuovo `crates/core/src/generators/applescript.rs`.
- **Azioni:** `iterm2_workspace(config, id, profiles) -> Result<String, String>` e `apple_terminal_workspace(...) -> Result<(String, Vec<Warning>), String>` (`ANALISI.md` §4.6), con doppio quoting shell+AppleScript.
- **Fatto quando:** V-base + V-snap; sul job macOS `osacompile -o /dev/null` compila ogni snapshot.

### M7.2 — Avvio e permessi [Mac]
- **File:** `crates/core/src/terminals/{iterm2.rs, apple_terminal.rs}`, nuovi `app/src-tauri/Info.plist` (`NSAppleEventsUsageDescription`) e `app/src-tauri/entitlements.plist` (`com.apple.security.automation.apple-events`), `app/src-tauri/tauri.macos.conf.json` (riferimento agli entitlement).
- **Azioni:** `launch_workspace` → `/usr/bin/osascript -` con lo script su stdin; errore -1743 tradotto in messaggio con il percorso nelle Impostazioni; anteprima "Workspace: id" con lo script al posto di `windowsTerminalArguments`.
- **Fatto quando:** V-mac; prova manuale: workspace d'esempio (2 tab, 1 split) aperto in iTerm2; con permesso negato compare il messaggio.

---

## M8 — Tool con Homebrew

### M8.1 — Gestore Homebrew [Mac]
- **File:** nuovo `crates/core/src/packages/brew.rs`, catalogo (M2.5).
- **Azioni:** `locate` (prefissi noti + `SearchPath`), `install`/`uninstall` con variabili non interattive e stdin nullo, `can_install` falso per i cask che chiedono la password (lista da S5), rilevamento versioni (`--version` con timeout), Nerd Font da `~/Library/Fonts` e `/Library/Fonts`; `RECOMMENDED` per OS (`oh-my-posh`, più `zsh-autosuggestions` se le predizioni zsh sono attive).
- **Fatto quando:** V-base; test portati da `tools.rs` (ID sconosciuti rifiutati senza eseguire `brew`, tool manuali → link, solo i tool installati dall'app sono disinstallabili); prova manuale di installazione e disinstallazione di `jq`.

### M8.2 — Homebrew assente [Ovunque]
- **File:** catalogo, `app/src/app/app.html` (pagina Tool).
- **Azioni:** stato "Homebrew non trovato" con link a brew.sh; nessuna installazione automatica.
- **Fatto quando:** test Vitest con bridge simulato.

---

## M9 — `csp port` su macOS [Mac]
- **File:** nuovo `crates/csp/src/port/macos.rs`, `crates/csp/Cargo.toml` (`[target.'cfg(target_os = "macos")'.dependencies] libproc = "…"`), `crates/csp/src/main.rs`, `crates/csp/src/tests.rs`.
- **Azioni:** `listeners`, `process_info` (nome, percorso, ora di avvio), `terminate` con `SIGTERM`, attesa 5 s e verifica dell'ora di avvio; rifiuto di PID 1 e di processi di altri utenti; nota sui listener non visibili senza privilegi (esiti di S9).
- **Fatto quando:** V-base; test `#[cfg(target_os = "macos")]` analoghi a `mod windows` (listener IPv4 e IPv6 temporanei trovati con il PID corrente, `verify_listener`); prova manuale con `python3 -m http.server 4200` e `csp port 4200`.

---

## M10 — UI Angular: stessa UX, opzioni per piattaforma [Ovunque]

Ogni task: **Fatto quando** V-base (parte Angular) verde, nuovi test Vitest per `os = "macos"`, e i test Windows esistenti in `app.spec.ts` invariati.

### M10.1 — Bridge e mock per piattaforma
- **File:** `app/src/app/config-bridge.ts`, `app/src/app/models.ts`.
- **Azioni:** `platformInfo()`; mock nel browser selezionabile con `?os=macos|windows` (percorsi d'esempio, shell, capacità); nessun percorso `C:\` nel mock macOS.

### M10.2 — Shell, etichette, anteprime
- **File:** `app/src/app/app.ts` (`profiles`, `pathStyles`, `segmentOptions`, `behaviorShells`), `app/src/app/app.html` (Home, schede profilo, anteprima del terminale, stato "Pronto in …").
- **Azioni:** elenco shell da `platformInfo`; etichette e prompt d'esempio per shell; stato del terminale rilevato; icona generica per le shell POSIX.

### M10.3 — Capacità nell'editor
- **File:** `app/src/app/app.html`, `app/src/app/app.ts`.
- **Azioni:** campi di Aspetto e Autocomplete filtrati o disabilitati con spiegazione secondo le capacità (immagine in Terminal.app, vista a elenco, allineamento, cursori, acrilico → "Sfocatura").

### M10.4 — Percorsi e programmi
- **File:** `app/src/app/config-validation.ts`, `app/src/app/app.ts` (righe con `npm.cmd`).
- **Azioni:** validazione ed esempi per OS; `npm` al posto di `npm.cmd` su macOS (e ovunque se D7 approvata).

### M10.5 — Tipografia
- **File:** `app/src/styles.css`, `DESIGN.md` (solo dopo l'approvazione di D12).
- **Azioni:** pile di font con fallback macOS; `terminalFonts` per OS.

### M10.6 — Testi
- **File:** `app/src/app/app.html`, `app/src/app/app.ts` (walkthrough).
- **Azioni:** banner, prompt, comandi (nota CMD/Clink nascosta su macOS), workspace (permesso Automazione), Tool (Homebrew).

---

## M11 — Pacchetto, firma, aggiornamenti [Mac]

### M11.1 — Icone e configurazione macOS
- **File:** `app/src-tauri/icons/`, `app/src-tauri/tauri.conf.json` (`bundle.icon`), nuovo `app/src-tauri/tauri.macos.conf.json`.
- **Azioni:** `npx tauri icon app/public/terminal-spezzato.png` (genera anche `.icns`); `bundle.active: true`, `targets: ["app", "dmg"]`, `macOS.minimumSystemVersion` (D3), entitlement e `Info.plist` di M7.2.
- **Fatto quando:** `npm run tauri -- build` produce `.app` e `.dmg` che si avviano su un Mac (firma ad-hoc).

### M11.2 — Sidecar `csp` [condivisa con Linux]
- **File:** `tauri.macos.conf.json` (`bundle.externalBin`), nuovo script `app/scripts/build-csp.mjs` (Node, multipiattaforma), `crates/core/src/desktop.rs`, generatori shell.
- **Azioni:** compilare `csp` per il target (per universal: due target + `lipo -create`) e nominarlo `csp-<triple>`; l'applicatore copia `csp` in `…/CliSuperpower/bin/` (file posseduto, nel manifest) e gli script `init.*` aggiungono quella cartella al `PATH` (D13).
- **Fatto quando:** in una nuova finestra iTerm2 `csp --help` funziona; `remove` toglie la copia.

### M11.3 — Updater [condivisa con Linux]
- **File:** `app/src-tauri/Cargo.toml` (`tauri-plugin-updater`), `tauri.conf.json` (`plugins.updater.pubkey`, `endpoints`; `bundle.createUpdaterArtifacts: true`), `lib.rs`, UI (notifica nuova versione).
- **Azioni:** come `PROGETTO.md` §6; chiave generata una volta e custodita (`PROGETTO.md` §9); gestione dell'errore "app in cartella non scrivibile".
- **Fatto quando:** aggiornamento da una release di prova N a N+1 riuscito su Apple Silicon e Intel (o Rosetta).

### M11.4 — Pipeline di release
- **File:** nuovo `.github/workflows/release.yml`.
- **Azioni:** su tag `v*`, `tauri-action` con job macOS (`--target universal-apple-darwin`, segreti `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`/`APPLE_PASSWORD`/`APPLE_TEAM_ID` o chiave API, `TAURI_SIGNING_PRIVATE_KEY`) e job Windows; `latest.json` unico con `darwin-aarch64`, `darwin-x86_64`, `windows-x86_64`.
- **Fatto quando:** una release di prova produce `.dmg` firmato e notarizzato (`spctl -a -vv` → "accepted, source=Notarized Developer ID") e `latest.json` valido. Richiede D2.

---

## M12 — Collaudo e documentazione

### M12.1 — Checklist di smoke test macOS [Mac]
- **File:** nuovo `docs/smoke/macos.md`.
- **Azioni:** checklist su Mac pulito: primo avvio (Gatekeeper), Applica con zsh/bash/fish, iTerm2 e Terminal.app, workspace (permesso Automazione), Tool con e senza Homebrew, `csp port`, cronologia/ripristino, disinstallazione integrazione (file utente identici all'originale: `shasum` prima/dopo), aggiornamento.
- **Fatto quando:** checklist eseguita una volta con esiti riportati.

### M12.2 — Documenti di prodotto [Ovunque]
- **File:** `README.md`, `PROGETTO.md`, `PRODUCT.md`, `landing/` (se richiesto).
- **Azioni:** piattaforme supportate, percorsi macOS nella tabella "Dove scrive l'app", blocco rc d'esempio, prerequisiti di sviluppo macOS (Xcode CLT, Rust con i due target), badge piattaforma. Solo dopo l'approvazione dell'utente: sono documenti di prodotto.
- **Fatto quando:** i documenti citano solo comportamenti verificati in M12.1.

---

## Riepilogo dei task condivisi con Linux

| Task | Contenuto da concordare |
|---|---|
| M0.3 | Matrice CI unica |
| M2.1–M2.7 | Nomi di moduli e trait (`platform`, `shells`, `terminals`, `packages`), `PlatformPaths`, permessi/symlink in `apply.rs`, interfaccia di `csp port` |
| M3.1–M3.4 | Un solo schema v7: valori di shell, percorsi POSIX, ID dei terminali, nuovi tool |
| M4.1–M4.6 | Generatori zsh/bash/fish e blocco rc: identici sulle due piattaforme |
| M5.1 | Ricostruzione del PATH per app grafiche |
| M6.1 | Catalogo degli schemi colore |
| M11.2–M11.3 | Distribuzione di `csp` e updater |
