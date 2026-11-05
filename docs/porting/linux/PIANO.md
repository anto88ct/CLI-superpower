# Porting di CLI Superpower su Linux — Piano di implementazione

> **Per Claude Code.** Eseguire i task **in ordine**, uno per volta, ciascuno in un commit (o PR) separato. Prima di ogni task leggere `CLAUDE.md`, [`CLAUDE-linux.md`](CLAUDE-linux.md) e la sezione di [`ANALISI.md`](ANALISI.md) citata. Le decisioni D1-D12 di `ANALISI.md` §0 sono il riferimento; le domande aperte di §16 vanno chiuse con l'utente prima delle fasi che le riguardano (indicate con ❓).
> I task marcati **[macOS]** producono codice condiviso con il porting macOS (`docs/porting/macos/`): prima di iniziarli verificare se l'altra sessione li ha già fatti o pianificati con nomi diversi, e allinearsi.

## Regole valide per ogni task

1. **Verifica standard** (deve passare prima di dichiarare il task finito, su Windows in locale e su Linux in CI dal task L0.1 in poi):
   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```
   ```bash
   cd app && npm run build && npm test -- --watch=false
   ```
   Nei task che toccano Tauri anche `cd app && npm run tauri -- build --no-bundle`.
2. **Nessuna regressione Windows:** i test esistenti (`crates/core/src/apply.rs`, `desktop.rs`, `config/tests.rs`, `generators.rs` e sottomoduli, `crates/csp/src/tests.rs`, `app/src/app/*.spec.ts`) non si modificano per farli passare, salvo spostamenti di file dichiarati nel task.
3. **Generatori puri** con snapshot `insta` (dal task L1.1). **Solo l'applicatore scrive su disco.** Ogni cambio di struct/schema: `schemaVersion` + migrazione + test + `models.ts`.
4. Le funzionalità Linux coprono **ciò che il codice Windows fa già** (F1-F8 e il sottoinsieme di F9 presente). Quake mode, `csp notify`, `csp table` restano fuori finché non esistono su Windows (`CLAUDE.md`: non anticipare funzionalità).
5. Nei limiti dichiarati a fine task elencare ciò che non è stato verificato su una macchina Linux reale.

## Mappa delle fasi

| Fase | Contenuto | Dipende da | Parallelizzabile con |
|---|---|---|---|
| L0 | CI Linux e refactoring senza cambi funzionali | — | — |
| L1 | Schema v7, catalogo temi, filtri per piattaforma | L0 | — |
| L2 | Shell Unix: bash, zsh, fish, pwsh su Linux | L1 | L4, L5 |
| L3 | Terminali Linux: Konsole, GNOME Terminal, Ptyxis | L1, L2.6 | L4, L5 |
| L4 | Tool esterni senza root | L1 | L2, L3, L5 |
| L5 | `csp port` su Linux | L0.1 | L2, L3, L4 |
| L6 | UI Angular guidata dalla piattaforma | L1, L2, L3 | — |
| L7 | Packaging, icone, release | L6 | — |
| L8 | Verifica finale su distro reali | tutte | — |

---

## Fase L0 — Fondamenta (nessun cambio di comportamento)

### L0.1 — Job CI Linux sul codice attuale
- **File:** `.github/workflows/ci.yml`, nuovo `app/src-tauri/tauri.linux.conf.json`, eventuali `#[cfg]` minimi in `crates/**` solo se clippy segnala codice morto su Linux.
- **Azioni:**
  1. Aggiungere il job `linux` su `ubuntu-22.04`: checkout, Node 24 con cache npm, Rust stabile con `rustfmt` e `clippy`, `sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev`.
  2. Passi: `npm ci`, `npm run build`, `npm test -- --watch=false`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm run tauri -- build --no-bundle`. **Non** eseguire `npm run fetch-clink` (script PowerShell).
  3. Creare `tauri.linux.conf.json` che sovrascrive `build.beforeDevCommand` (`npm run start`) e `build.beforeBuildCommand` (`npm run build`) ed esclude la risorsa `resources/clink/` (verificare con `npm run tauri -- build --no-bundle` che la configurazione unita sia valida).
- **Fatto quando:** il job `linux` è verde; il job `windows` è invariato e verde.

### L0.2 — Tipi di piattaforma e `Probe` **[macOS]**
- **Prima di iniziare:** se il porting macOS ha già creato `platform.rs`, `shells/mod.rs` o `terminals/mod.rs` (vedi `docs/porting/macos/PIANO.md`), **riusarli** e aggiungere solo ciò che manca (`Probe`, `user_bin`, `startup_args`).
- **File:** nuovi `crates/core/src/platform.rs`, `crates/core/src/shells/mod.rs`, `crates/core/src/terminals/mod.rs`, `crates/core/src/lib.rs`.
- **Azioni:** definire `Os`, `PlatformPaths { home, data, state, generated, user_bin }`, `Capabilities`, trait `Probe` (`run`, `read`, `env`, `which`), `SystemProbe` (implementazione reale con `std::process::Command` e `apply::read`), `FakeProbe` in `#[cfg(test)]` (risposte programmate), trait `ShellIntegration`/`ShellFacts` e `TerminalIntegration`/`TerminalFacts` come in `ANALISI.md` §8.2 (`PackageManager` arriva in L4). Nessun utilizzo ancora.
- **Fatto quando:** verifica standard verde; test unitario di `FakeProbe`.

### L0.3 — Estrarre Windows Terminal da `Desktop`
- **File:** `crates/core/src/desktop.rs` → nuovo `crates/core/src/terminals/windows_terminal.rs` (e `terminals/mod.rs`).
- **Azioni:** spostare `TerminalSettings`, `profile_list`, `profile_shell`, `Desktop::profiles`, la parte di `Desktop::plan` che produce fragment, immagini, `defaultProfile` e anteprime dei workspace, e la logica di `Desktop::launch` (`wt.exe`) in `WindowsTerminal: TerminalIntegration`. `Desktop::plan` chiama `terminal.plan(...)`. I moduli Windows si compilano su tutti gli OS (nessuna API Windows qui); la registrazione delle integrazioni per OS avviene in L0.6.
- **Fatto quando:** `terminal_discovery_accepts_jsonc_and_reports_overrides` e `preview_apply_and_remove_integrate_without_touching_live_terminal` passano invariati (eventualmente spostati nel nuovo modulo); verifica standard verde su Windows e Linux.

### L0.4 — Estrarre PowerShell e cmd/Clink
- **File:** `desktop.rs` → nuovi `crates/core/src/shells/powershell.rs` (comune: pwsh esiste anche su Linux) e `crates/core/src/shells/cmd.rs`.
- **Azioni:**
  1. `PowerShellShell { id: "pwsh"|"powershell", binary, block_path_style }` implementa `ShellIntegration`: contiene `shell_profile` (binario parametrizzato, oggi `"{shell}.exe"`), `profile_block`, `update_profile_text`, `OH_MY_POSH_INIT`, gli avvisi su policy e collisioni.
  2. `CmdShell` implementa `ShellIntegration` con lo script Clink (`clink_script_with_theme`, `clink::profile_directory`) e assorbe `Desktop::sync_clink` e la chiamata a `clink::disable` in `Desktop::remove` (aggiungere al trait un metodo `after_remove` o equivalente).
- **Fatto quando:** `profile_block_preserves_bom_and_user_lines_and_is_idempotent` passa invariato; comportamento Windows identico (stessi file e anteprime: aggiungere un test che confronta l'elenco di `PreviewFile` di un caso con comandi pwsh + cmd prima/dopo, usando `FakeProbe`).

### L0.5 — `rc_block` **[macOS]**
- **File:** `crates/core/src/shells/mod.rs` (funzione `rc_block`), `shells/powershell.rs`.
- **Azioni:** estrarre da `update_profile_text` una funzione pura `rc_block(text, body_lines, comment: &str, default_newline: &str) -> Result<String, String>` e la rimozione corrispondente; marcatori `{comment} >>> CLI Superpower >>>` / `{comment} <<< CLI Superpower <<<`. `update_profile_text` la usa con `"#"` e `"\r\n"`.
- **Fatto quando:** snapshot/test per: file vuoto (`\n` e `\r\n`), file senza newline finale, blocco esistente aggiornato, marcatori ambigui → errore, idempotenza; test PowerShell esistenti invariati.

### L0.6 — `Desktop` orchestratore e percorsi per OS
- **File:** `desktop.rs`, `platform.rs` (funzione unica, con `cfg(target_os)`, che registra terminali, shell e gestori pacchetti dell'OS corrente), `app/src-tauri/src/lib.rs`.
- **Azioni:** `Desktop::new(paths: PlatformPaths, terminals, shells)` (mantenere un costruttore di compatibilità per i test se utile). In `lib.rs`: su Windows **gli stessi percorsi di oggi** (`data_dir()/CliSuperpower`, `local_data_dir()/CliSuperpower`, `system_local = local_data_dir()`); su Linux `data = config_dir()/cli-superpower`, `state = data_dir()/cli-superpower`, `generated = state/generated` (❓ Q9 per il nome). `Desktop::terminal_profiles` delega alle integrazioni. Su Linux, per ora, nessun terminale e nessuna shell registrati: l'app parte e mostra la UI.
- **Fatto quando:** verifica standard verde; su Linux `npm run tauri dev` apre la finestra e `load_config`/`save_config` funzionano in `~/.config/cli-superpower` (verifica manuale o test con `PlatformPaths` temporanei).

### L0.7 — Apertura link portabile **[macOS]**
- **File:** `app/src-tauri/Cargo.toml`, `app/src-tauri/src/lib.rs`, capabilities Tauri.
- **Azioni:** sostituire `rundll32` in `open_link` con `tauri-plugin-opener` mantenendo la lista chiusa di URL (`github`, `website`).
- **Fatto quando:** verifica standard + build Tauri verdi su Windows e Linux; il link si apre su entrambi (manuale).

---

## Fase L1 — Modello e schema

### L1.1 — Snapshot test con `insta`
- **File:** `crates/core/Cargo.toml` (`[dev-dependencies] insta`), un test pilota in `generators/prompt.rs`.
- **Azioni:** aggiungere `insta`; convertire **un** test esistente in snapshot come esempio (`oh_my_posh_config`); documentare in `CLAUDE-linux.md` il comando `cargo insta review`.
- **Fatto quando:** snapshot salvato in `crates/core/src/generators/snapshots/`; verifica standard verde.

### L1.2 — `schemaVersion` 7 **[macOS]** ❓Q5
- **File:** `crates/core/src/config.rs`, `config/validate.rs`, `config/tests.rs`, `docs/config-schema.md`, `app/src/app/models.ts`, `config-bridge.ts`, `config-validation.ts`, `app.spec.ts`/`config-bridge.spec.ts`.
- **Azioni:**
  1. `SCHEMA_VERSION = 7`; in `migrate` aggiungere il salto 6→7 (solo versione); `version_field` accetta 7.
  2. `PROFILES` += `bash`, `zsh`, `fish` (vale per `appearance`, `shells`, `pane.profile`, `shellBehavior`, `defaultProfile`).
  3. `windows_path` → `absolute_path`: percorsi Windows attuali **oppure** POSIX assoluti (`/` iniziale, nessun componente vuoto/`.`/`..`, niente NUL).
  4. `globalTerminal.terminals` facoltativo (array di ID distinti, solo nel file personale): `windows-terminal | iterm2 | apple-terminal | konsole | gnome-terminal | ptyxis`. Omesso = tutti i terminali supportati e rilevati sull'OS corrente (stessa definizione dell'analisi macOS §6).
  5. `TOOLS` (`config.rs`) += `zsh-autosuggestions`, `ble-sh` (e quelli concordati con macOS).
  6. `version_field` usa `SCHEMA_VERSION` invece del `6` cablato.
  7. TS: `Profile` con i nuovi valori, `schemaVersion: 1..7`, mock (`config-bridge.ts::previewTeam`) e validazione TS che accettano 7 e percorsi POSIX.
- **Coordinamento:** è **lo stesso task** del salto v7 nel piano macOS: farlo una sola volta, includendo i valori richiesti da entrambi.
- **Fatto quando:** test di migrazione 1..6 → 7; test di validazione per i nuovi valori, per percorsi POSIX validi/invalidi e per `terminals` (ID sconosciuto o duplicato → errore); test esistenti aggiornati **solo** dove asseriscono il numero di versione (`desktop.rs` righe 958 e 977, `config/tests.rs` righe 207-297); `docs/config-schema.md` aggiornato; verifica standard verde.

### L1.3 — Applicabilità per piattaforma (puro)
- **File:** nuovo `crates/core/src/applicable.rs`.
- **Azioni:** funzione pura `applicable(effective: &Value, os: Os, shells: &[&str]) -> (Value, Vec<PreviewFile>)` che toglie comandi, workspace e voci di aspetto/behavior non applicabili (shell assente sull'OS, percorsi non nativi) e produce avvisi leggibili. `Desktop::plan` la applica prima di terminali e shell.
- **Fatto quando:** snapshot per un pacchetto team misto (cmd + bash + percorsi `C:\` e `/home`) valutato come Windows e come Linux; su Windows il risultato per una config solo-Windows è identico all'input.

### L1.4 — Catalogo temi con palette **[macOS]**
- **File:** nuovo `crates/core/src/color_schemes.rs`.
- **Azioni:** per gli 8 schemi di `appearanceThemes` (`app.ts:342`): 16 colori ANSI, foreground, background, cursore, selezione (valori delle definizioni ufficiali di Windows Terminal). API `scheme(name) -> Option<&ColorScheme>`. Se il porting macOS ha già creato `color_schemes.rs`, solo verificare che contenga i campi necessari a Konsole e GNOME.
- **Fatto quando:** test che ogni nome usato dalla UI esiste nel catalogo e ha colori `#RRGGBB` validi.

### L1.5 — Stile `full` del percorso indipendente dall'OS
- **File:** `crates/core/src/generators/prompt.rs`.
- **Azioni:** sostituire il template `{{ replace "/" "\\" .Location }}` con un template che usa `\` solo su Windows (variabile `.OS` di Oh My Posh, da verificare nella documentazione della versione installata) **oppure** passare `Os` al generatore. Scegliere la prima se `.OS` è disponibile.
- **Fatto quando:** snapshot del segmento `directory` con `pathStyle: full`; verifica manuale su Windows che il prompt resti `C:\Users\…`.

---

## Fase L2 — Shell Unix **[macOS]**

> Spike preliminare **S1** (`ANALISI.md` §15): prefisso `/` in bash, zsh, fish. Risultati in `docs/spikes/linux.md` (nuovo file) prima di L2.8.

### L2.1 — Generatore comandi rapidi bash/zsh
- **File:** nuovo `crates/core/src/generators/posix.rs`.
- **Azioni:** `posix_init(config, shell: "bash"|"zsh") -> Result<String, String>`: intestazione, guardia interattiva, `PATH` con `~/.local/bin` se mancante, una funzione per comando (`template`, `parameters` posizionali, obbligatori/opzionali/default, `{{`/`}}`, `workingDirectory` in subshell, `changeDirectory`, `--help`). Quoting con `sh_literal` (apici singoli, `'\''`). Solo sintassi compatibile **bash 3.2** (niente array associativi, `${x,,}`, `mapfile`). Nomi validati come in `powershell.rs` (`identifier`) + parole riservate sh.
- **Fatto quando:** snapshot per: comando semplice, parametri obbligatori/opzionali/default, graffe letterali, `workingDirectory`, `changeDirectory`, prefisso `x-`; test `#[cfg(unix)]` che esegue `bash -n` e `zsh -n` sul risultato (saltato con messaggio se la shell manca) e un test comportamentale: `bash -c '. init.bash; demo "a b" "c'"'"'d"'` con programma `printf '%s\n'` → argomenti esatti.

### L2.2 — Generatore fish
- **File:** nuovo `crates/core/src/generators/fish.rs`.
- **Azioni:** `fish_init(config)`: `status is-interactive; or exit`, `fish_add_path -g` condizionale per `~/.local/bin`, `function nome --description`, `argparse h/help`, parametri da `$argv`, `workingDirectory` con `begin; pushd …; …; popd; end` o `fish -c` in sottoprocesso (scegliere la forma che garantisce il ritorno di cartella anche in errore), `changeDirectory` con `cd`.
- **Fatto quando:** snapshot analoghi a L2.1; test `fish --no-execute` e test comportamentale con `fish -c` se fish è presente.

### L2.3 — Autocomplete e cronologia per bash/zsh/fish
- **File:** `generators/posix.rs`, `generators/fish.rs`, `generators/behavior.rs` (solo se servono helper comuni).
- **Azioni:** tradurre `Behavior` (`predictions`, `view`, `accept_with_tab`, `patterns`) come da `ANALISI.md` §3.3: zsh-autosuggestions e `zshaddhistory`; bash `HISTIGNORE` insensibile alle maiuscole e ble.sh solo se il tool `ble-sh` è attivo; fish nativo; fzf per la vista `list`. Ogni integrazione controlla la presenza del file/plugin e la versione prima di usarlo (come `append_behavior` controlla PSReadLine).
- **Fatto quando:** snapshot per ogni combinazione significativa; test comportamentale zsh: una riga con `password` resta in `fc -l` ma non finisce in `$HISTFILE`; bash: la riga non finisce in `HISTFILE`.

### L2.4 — Banner e prompt per bash/zsh/fish
- **File:** `generators/banner.rs` (nuove funzioni `posix`, `fish`), `generators/posix.rs`, `generators/fish.rs`.
- **Azioni:** banner solo interattivo, output su terminale e larghezza ≥ disegno; inizializzazione Oh My Posh con il file `prompt.omp.json` se `oh-my-posh` e il file esistono (equivalente di `OH_MY_POSH_INIT`).
- **Fatto quando:** snapshot; test comportamentale che `bash -ic` con output reindirizzato non stampa il banner.

### L2.5 — Robustezza dell'applicatore su Unix **[macOS]**
- **File:** `crates/core/src/apply.rs`.
- **Azioni:**
  1. `atomic_write` conserva i permessi del file sostituito (`#[cfg(unix)]` con `PermissionsExt`) e fa `fsync` della cartella padre dopo `rename`.
  2. Per i file condivisi (`shared_input`) che sono symlink: risolvere il target; se è un file regolare scrivibile sotto la home, pianificare la scrittura sul target (percorso risolto nel manifest e nell'anteprima); altrimenti errore con messaggio e blocco da copiare. I file di proprietà dell'app restano rifiutati se symlink (comportamento attuale di `read`).
- **Fatto quando:** test `#[cfg(unix)]`: file 0600 resta 0600; `.bashrc` symlink verso file in temp viene aggiornato attraverso il link e il link resta; symlink verso file in sola lettura → errore e nessuna scrittura; test esistenti invariati.

### L2.6 — `ShellIntegration` per bash, zsh, fish
- **File:** nuovi `crates/core/src/shells/{bash,zsh,fish}.rs` (se il porting macOS li ha già creati, aggiungere solo le differenze Linux), `platform.rs` (registrazione delle integrazioni Linux).
- **Azioni:** `discover` via `Probe`: presenza e versione; rc di bash `~/.bashrc`; rc di zsh da `zsh -c 'print -r -- ${ZDOTDIR:-$HOME}'`; cartella fish da `fish -c 'echo $__fish_config_dir'`; collisioni (`type -t`, `whence -w`, `type -q`). `plan`: file `init.bash`/`init.zsh` in `PlatformPaths::generated`, blocco in rc con `rc_block` (`"#"`, `"\n"`, percorso assoluto quotato), file fish in `conf.d/cli-superpower.fish` (nessun file utente). `startup_args` per i workspace (`ANALISI.md` §7). Registrare le tre shell per Linux in `platform.rs`.
- **Fatto quando:** test con `FakeProbe` e cartelle temporanee per preview/apply/remove (stile di `preview_apply_and_remove_integrate_without_touching_live_terminal`): dopo `remove` i file utente tornano byte per byte all'originale; nessun test tocca la vera home (impostare `HOME`/`XDG_*` solo nei processi figli o usare `PlatformPaths` temporanei).

### L2.7 — pwsh su Linux
- **File:** `shells/powershell.rs`, `platform.rs` (registrazione delle integrazioni Linux).
- **Azioni:** binario `pwsh`; blocco del profilo con percorso assoluto (non `$env:LOCALAPPDATA`) e newline `\n`; nome `oh-my-posh` nell'inizializzazione; nessun avviso di policy; `powershell` (5.1) non registrata su Linux.
- **Fatto quando:** snapshot del blocco e di `init-pwsh.ps1` per Linux; test `#[cfg(unix)]` con `pwsh` reale se presente (sul runner Ubuntu di GitHub `pwsh` è preinstallato): la funzione generata passa gli argomenti intatti.

### L2.8 — Prefisso `/` per shell
- **File:** `config.rs` (`validate_effective`), `generators/posix.rs`, `generators/fish.rs`, `docs/config-schema.md`.
- **Azioni:** secondo l'esito di S1: zsh con widget ZLE di riscrittura, fish con `abbr --regex`, bash → errore di validazione «Il prefisso / non è disponibile in bash: usa x- oppure nessun prefisso».
- **Fatto quando:** test di validazione; test comportamentale zsh/fish dove possibile (zsh: `zle` richiede un terminale → test con `zpty` o solo snapshot, dichiarando il limite).

---

## Fase L3 — Terminali Linux ❓Q1

> Spike **S2** (Konsole), **S3** (dconf), **S4** (workspace) prima dei rispettivi task; fixture reali in `crates/core/src/generators/fixtures/linux/`.

### L3.1 — Preferenze dconf nell'applicatore **[macOS: stesso meccanismo di `PlannedChange::Preference`]**
- **File:** `crates/core/src/apply.rs`, nuovo `crates/core/src/apply/store.rs` (o modulo equivalente).
- **Prima di iniziare:** se il porting macOS ha già introdotto `PlannedChange::Preference` per `defaults`, aggiungere dconf come secondo «store» dello stesso meccanismo invece di crearne uno nuovo.
- **Azioni:** `read`/`replace` instradati da uno store scelto in base al tipo di modifica o al prefisso del percorso: file normali (comportamento attuale), `dconf-dir:/…/` (`dconf dump` / `dconf reset -f` + `dconf load`), `dconf-key:/…` (`dconf read` / `dconf write` / `dconf reset`). Il journal, il manifest e la cronologia restano identici. Errore chiaro se `dconf` manca.
- **Fatto quando:** test con uno `Store` finto in memoria (apply, conflitto, rollback di `recover`, `restore_latest`); test di integrazione in CI Linux sotto `dbus-run-session` con `XDG_CONFIG_HOME` temporanea; test Windows invariati.

### L3.2 — Generatori Konsole
- **File:** nuovo `crates/core/src/generators/konsole.rs`, nuovo `generators/ini.rs` (modifica mirata di chiavi INI preservando il resto).
- **Azioni:** `konsole_profile(config, shell, shell_path) -> String` (nome «CLI Superpower · <shell>», comando, font, cursore, margine, schema colori), `konsole_colorscheme(config, shell) -> String` (palette da `color_schemes.rs`, `Opacity`, `Blur` da `useAcrylic`, `Wallpaper` dall'immagine copiata), `konsole_tabs(workspace) -> String` per `--tabs-from-file`; `ini::set_key(text, section, key, value)` / `remove_key` per `konsolerc` `DefaultProfile`.
- **Fatto quando:** snapshot verificati contro le fixture di S2; test di `ini` su commenti, sezioni ripetute, chiave assente, idempotenza.

### L3.3 — `TerminalIntegration` per Konsole
- **File:** nuovo `crates/core/src/terminals/konsole.rs`.
- **Azioni:** `discover` (binario, versione, `XDG_DATA_HOME`/`XDG_CONFIG_HOME`, installazione Flatpak → non supportata con avviso); `plan`: file nostri `CliSuperpower-<shell>.profile`/`.colorscheme` in `~/.local/share/konsole/`, immagine copiata accanto, `konsolerc` come file condiviso solo con `defaultProfile`; avvisi per capacità mancanti; `launch_workspace`: file di tab temporaneo in `PlatformPaths::state` + `konsole --tabs-from-file` (split secondo S4).
- **Fatto quando:** test con `FakeProbe` e home temporanea: apply → remove ripristina tutto; anteprima elenca avvisi; verifica manuale su KDE (limite dichiarato se non disponibile).

### L3.4 — Generatori GNOME Terminal e Ptyxis
- **File:** nuovo `crates/core/src/generators/gnome.rs`.
- **Azioni:** UUID deterministico per shell (derivato da SHA-256 di `cli-superpower/<terminale>/<shell>`, formattato come UUID valido); keyfile dconf del profilo (`visible-name`, `use-custom-command`, `custom-command`, `font`/`use-system-font`, `palette`, colori, `cursor-shape`, trasparenza); parser/serializzatore puro per array GVariant di stringhe (`['a', 'b']`) per aggiungere/rimuovere il nostro UUID dalla lista; per Ptyxis file `.palette` e chiavi equivalenti (font/cursore globali → avviso).
- **Fatto quando:** snapshot + test del parser GVariant (spazi, apici, lista vuota `@as []`, idempotenza).

### L3.5 — `TerminalIntegration` per GNOME Terminal e Ptyxis
- **File:** nuovo `crates/core/src/terminals/gnome.rs`.
- **Azioni:** `discover` (binari, `dconf`, lista profili e predefinito); `plan` con percorsi virtuali `dconf-dir:`/`dconf-key:` (L3.1), lista e `default` come input condivisi; `launch_workspace` con `gnome-terminal --window --tab …` / `ptyxis --new-window …`; split → tab con avviso.
- **Fatto quando:** test con `FakeProbe`; test di integrazione dconf in `dbus-run-session` (apply, remove, lista ripristinata identica); verifica manuale su GNOME.

### L3.6 — Scelta dei terminali
- **File:** `platform.rs` (registrazione delle integrazioni Linux), `desktop.rs`.
- **Azioni:** terminali configurati = `globalTerminal.terminals` filtrati per quelli installati; se il campo è omesso, tutti i terminali supportati e rilevati (stessa regola di macOS). Il terminale **del desktop** (`XDG_CURRENT_DESKTOP`: KDE → Konsole; GNOME → Ptyxis se presente, poi GNOME Terminal) è quello usato da «Apri workspace» e mostrato per primo nella UI. Un terminale scelto ma non installato produce un avviso, non un errore bloccante per le shell.
- **Fatto quando:** test della funzione di scelta (pura, input = rilevamento simulato), compresi: campo omesso con due terminali installati → entrambi; terminale in elenco ma assente → avviso.

---

## Fase L4 — Strumenti esterni senza root ❓Q6

### L4.1 — Catalogo per OS
- **File:** `crates/core/src/tools.rs` (eventualmente `tools/catalog.rs`).
- **Azioni:** `Tool` con `executables` per OS e sorgenti per OS (`winget_id` per Windows; `release` con URL, SHA-256, tipo archivio, percorso del binario per `linux-x86_64`/`linux-aarch64`); nuovi ID `zsh-autosuggestions`, `ble-sh`; `clink` e `powershell` nascosti su Linux. `ToolStatus` invariato nella forma (aggiungere campi solo se la UI ne ha bisogno, aggiornando `models.ts`).
- **Fatto quando:** test esistenti di `tools.rs` invariati; nuovo test che ogni voce Linux ha URL HTTPS e SHA-256 di 64 cifre esadecimali; comportamento Windows identico.

### L4.2 — Sorgente «release» **[macOS]**
- **File:** nuovo `crates/core/src/packages/release.rs`, `crates/core/Cargo.toml` (`ureq` con rustls, `flate2`, `tar`, `zip`).
- **Azioni:** download in file temporaneo, verifica SHA-256 **prima** dell'estrazione, estrazione in `<state>/tools/<id>/<versione>/` rifiutando percorsi con `..` o assoluti, link o copia in `PlatformPaths::user_bin`, record in `installed-tools.json` (percorsi creati + hash). `uninstall` rimuove solo i percorsi registrati e solo se l'hash coincide.
- **Fatto quando:** test senza rete con archivi creati nel test (funzione `install_from_archive(bytes, …)` separata dal download): hash errato → nessun file; archivio con `../` → rifiutato; installa → disinstalla lascia la cartella com'era.

### L4.3 — Rilevamento su Linux
- **File:** `tools.rs` (`mod platform` per `target_os = "linux"`).
- **Azioni:** ricerca in `PATH` + percorsi noti (`ANALISI.md` §5.3), versione con `<bin> --version` (timeout 3 s, prima riga), Nerd Font con `fc-list : family`, plugin per presenza file; alias `batcat`.
- **Fatto quando:** test con `FakeProbe`; `statuses` su runner Ubuntu elenca correttamente `jq` (preinstallato) come installato.

### L4.4 — Homebrew e gestore di sistema **[macOS per brew]**
- **File:** nuovi `crates/core/src/packages/{brew,system}.rs`.
- **Azioni:** `brew install <formula>` se `brew` esiste; per `apt`/`dnf`/`pacman`/`zypper` (da `/etc/os-release`) solo **comando proposto** + esecuzione con `pkexec` su richiesta esplicita dalla UI (nuovo comando IPC dedicato, mai dentro «Installa consigliati»). Distro immutabili: nessuna proposta di sistema.
- **Fatto quando:** test della costruzione dei comandi (puri) per ogni distro; nessun test esegue davvero `pkexec`.

### L4.5 — Nerd Font
- **File:** `packages/release.rs` o `packages/font.rs`.
- **Azioni:** zip ufficiale con SHA-256 fisso in `~/.local/share/fonts/NerdFonts/<nome>/`, poi `fc-cache -f` (errore non bloccante).
- **Fatto quando:** test con archivio finto; rimozione completa alla disinstallazione.

---

## Fase L5 — `csp port` su Linux

### L5.1 — Parser `/proc/net/tcp` comune **[macOS: interfaccia]**
- **File:** nuovo `crates/csp/src/procnet.rs` (compilato su ogni OS), `crates/csp/src/tests.rs`.
- **Azioni:** funzione pura `parse_tcp_table(text, ipv6: bool) -> Vec<(SocketAddr, state, inode, uid)>` (indirizzi esadecimali in ordine host little-endian).
- **Fatto quando:** test con fixture IPv4/IPv6, righe malformate ignorate, stato LISTEN `0A` filtrato.

### L5.2 — Implementazione Linux
- **File:** nuovo `crates/csp/src/port_linux.rs`, `crates/csp/src/main.rs` (`#[cfg(target_os = "linux")] mod port_linux;` e `run_port`), `crates/csp/Cargo.toml` (`rustix` con feature `process`, `fs` in `[target.'cfg(target_os = "linux")'.dependencies]`).
- **Azioni:** stessa UX di `port::run` (stesso testo, `confirmed_pid` riusato spostandolo in modulo comune); inode → PID da `/proc/<pid>/fd`; nome/percorso/età da `/proc`; rifiuto per PID 1, thread del kernel, altro UID; terminazione con `pidfd_open` + verifica `starttime` + `pidfd_send_signal(SIGTERM)` + attesa 5 s; porta occupata da processo non visibile → messaggio con `sudo ss -ltnp`.
- **Fatto quando:** test `#[cfg(target_os = "linux")]` analoghi a quelli Windows (`finds_temporary_ipv4_listener`, IPv6); test di `confirmed_pid` comune; `csp port 1` su Linux non restituisce più «il comando è disponibile su Windows».

---

## Fase L6 — UI Angular

### L6.1 — `platform_info` e `list_fonts` **[macOS]**
- **File:** `crates/core/src/desktop.rs`, `app/src-tauri/src/lib.rs`, `app/src/app/models.ts`, `config-bridge.ts`, `config-bridge.spec.ts`.
- **Azioni:** comando IPC `platform_info` → `{os, pathExample, terminal: {id, name, capabilities} | null, terminals: […], shells: [{id, label, installed, version, note, supportsPrefix}]}`; `list_fonts` → font monospazio (Linux `fc-list :spacing=mono family`; Windows: elenco attuale come fallback). Mock del bridge con `?os=linux` / `?os=windows`.
- **Fatto quando:** test del bridge per entrambi i mock; verifica standard verde.

### L6.2 — UI guidata dalla piattaforma
- **File:** `app/src/app/app.ts`, `app.html`, `config-validation.ts`, `app.spec.ts`.
- **Azioni:** `profiles` e `behaviorShells` da `platform_info` (rimuovere le costanti di `app.ts:19` e `app.ts:268-270`, mantenendo i testi Windows identici quando `os = windows`); controlli aspetto nascosti/disattivati secondo `capabilities` con spiegazione; nome del terminale nei testi al posto di «Windows Terminal»; esempio e regex dei percorsi per OS; font da `list_fonts`; colori anteprima dal catalogo temi (se esposto) o invariati; prompt di esempio della home (`app.html:47-48`) per shell; `npm.cmd` degli assistenti (`app.ts:440`, `app.ts:490`) solo su Windows.
- **Fatto quando:** test Angular: con mock Linux non compaiono «Windows PowerShell» né «Prompt dei comandi», compaiono bash/zsh/fish, l'assistente framework crea un comando con programma `npm`; con mock Windows l'output coincide con quello attuale (snapshot DOM o asserzioni esistenti invariate); percorso `/home/u/x` valido solo con OS Linux.

### L6.3 — Selettore del terminale e pagina tool
- **File:** `app.ts`, `app.html`, `models.ts`, `app.spec.ts`.
- **Azioni:** selettore «Terminali da configurare» (caselle) visibile solo con più terminali rilevati, salva `globalTerminal.terminals`; pagina tool con sorgente indicata e, per i gestori di sistema, comando copiabile + pulsante esplicito «Installa con password di amministratore».
- **Fatto quando:** test Angular del selettore e della pagina tool con mock Linux.

---

## Fase L7 — Packaging e release ❓Q3, Q7

### L7.1 — Bundle Linux
- **File:** `app/src-tauri/tauri.linux.conf.json`, `app/src-tauri/icons/*.png`, `tauri.conf.json` (solo se serve aggiungere icone PNG comuni).
- **Azioni:** `bundle.active: true` **solo** nella config Linux con `targets: ["appimage", "deb", "rpm"]`; icone PNG 32/128/256/512 dal logo esistente; dipendenze deb/rpm (`libwebkit2gtk-4.1-0`, `libgtk-3-0`); `csp` come `externalBin` (sidecar) installato in `/usr/bin` per deb/rpm; categoria `Development;Utility`.
- **Fatto quando:** `npm run tauri -- build` su Ubuntu 22.04 produce i tre pacchetti; `dpkg -I` mostra le dipendenze; l'AppImage parte su Ubuntu 24.04 (con `libfuse2` installata).

### L7.2 — Job di release Linux
- **File:** `.github/workflows/` (workflow di release, quando esisterà per Windows).
- **Azioni:** matrice `windows-latest` + `ubuntu-22.04` con `tauri-action`; artefatti AppImage/deb/rpm + `.sig` dell'updater + SHA-256.
- **Fatto quando:** una release di prova (tag `v0.x.0-linux-test` su fork o draft) contiene gli artefatti Linux.

### L7.3 — Updater e avvio
- **File:** `app/src-tauri/src/lib.rs`, `tauri.linux.conf.json`.
- **Azioni:** quando l'updater sarà integrato (PROGETTO §6), voce `linux-x86_64` in `latest.json` per l'AppImage; verificare il supporto deb/rpm della versione del plugin; su Linux impostare `WEBKIT_DISABLE_DMABUF_RENDERER=1` prima di creare la finestra se Wayland + NVIDIA proprietario (rilevamento da `/proc/driver/nvidia/version`) o documentarlo.
- **Fatto quando:** aggiornamento da una versione AppImage a una successiva funziona in una VM; limite dichiarato per deb/rpm se non supportato.

---

## Fase L8 — Verifica finale

### L8.1 — Smoke test manuale
- **File:** nuovo `docs/porting/linux/SMOKE.md` (checklist).
- **Azioni:** su VM pulite Ubuntu 24.04 GNOME (Ptyxis/GNOME Terminal) e Fedora KDE (Konsole): installa AppImage, configura aspetto per bash e zsh, comandi rapidi con parametri, workspace, prompt, autocomplete, pacchetto team misto Windows/Linux, installa `fzf` e `oh-my-posh` senza root, `csp port`, Cronologia → Ripristina, Disinstalla integrazione → `~/.bashrc`, `~/.zshrc`, dconf e `~/.local/share/konsole` tornano identici (confronto con copia iniziale).
- **Fatto quando:** checklist compilata con esito per ogni voce; difetti aperti come issue.

### L8.2 — Documentazione
- **File:** `README.md`, `PROGETTO.md`, `PRODUCT.md`, `docs/config-schema.md` (**solo** con approvazione dell'utente, perché cambiano il posizionamento «solo Windows»).
- **Azioni:** piattaforme supportate, prerequisiti Linux (WebKitGTK, libfuse2), percorsi XDG, tabella terminali/capacità.
- **Fatto quando:** l'utente approva il testo.

---

## Sessioni parallele possibili

Dopo L1: **L2**, **L4** e **L5** toccano file distinti (`generators/posix|fish`, `shells/*`; `tools.rs`, `packages/*`; `crates/csp/*`) e possono andare in sessioni separate con worktree propri. L3 dipende da L2.6 (le shell forniscono i comandi dei profili) e da L3.1. L6 parte quando `platform_info` (L6.1) è definito; il mock permette di iniziare la UI in parallelo a L3.
