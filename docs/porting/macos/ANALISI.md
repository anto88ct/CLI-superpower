# Porting di CLI Superpower su macOS — Analisi tecnica

> **Stato:** bozza v0.1 · **Data:** 6 novembre 2026 · **Base:** codice nella cartella di lavoro (nessun commit), `schemaVersion` 6
> **Obiettivo:** stessa UX e stesse funzionalità della versione Windows, su macOS (Apple Silicon e Intel).
> **Documenti collegati:** [`PIANO.md`](PIANO.md) (piano eseguibile), [`CLAUDE-macos.md`](CLAUDE-macos.md) (regole aggiuntive). Un'analisi parallela per Linux vive in `docs/porting/linux/`: i punti marcati **[condivisa con Linux]** vanno decisi una volta sola per entrambi.

Le affermazioni sul codice citano file e funzioni reali. Le affermazioni sul comportamento di macOS, dei terminali e delle shell che non sono state verificate su una macchina reale sono marcate **(da verificare in S*n*)**: gli spike S1–S11 sono definiti in `PIANO.md`, fase M1, sul modello di `docs/spikes/fase-0.md`.

---

## 1. Sintesi delle decisioni raccomandate

| Tema | Raccomandazione per l'MVP macOS | Motivo |
|---|---|---|
| Terminale principale | **iTerm2** con *Dynamic Profiles* | È l'equivalente più vicino ai fragment di Windows Terminal: file JSON di proprietà dell'app in una cartella nota, ricaricati automaticamente, rimozione = cancellare il file. Supporta per profilo colori, font, trasparenza, sfocatura, immagine di sfondo, cursore; workspace con tab e split via AppleScript. |
| Terminale secondario | **Terminal.app** con supporto ridotto | È il terminale predefinito: va coperto, ma i profili sono in un dominio di preferenze binario (`com.apple.Terminal`) con colori e font archiviati, niente immagini né split. Aspetto di base e workspace a finestre separate, dopo lo spike S3. |
| Altri terminali | WezTerm, Ghostty, Kitty, Alacritty **dopo l'MVP**, come adapter **[condivisa con Linux]** | Hanno configurazione testuale con `include`/`config-file`/`import` (facile e non distruttiva) e girano anche su Linux: un adapter serve due piattaforme. Non sono il default di nessun utente macOS. |
| Shell | **zsh** (primaria, default da macOS 10.15), **bash**, **fish**, **pwsh** opzionale | zsh copre quasi tutti; bash per chi l'ha scelta; fish è popolare e ha `conf.d` (zero modifiche ai file utente); pwsh riusa quasi tutto il generatore esistente. `powershell` (5.1) e `cmd` non esistono su macOS. |
| Iniezione nelle shell | Blocco con marcatori in `${ZDOTDIR:-~}/.zshrc`, nel file di login di bash (+ `.bashrc` con guardia), file nostro in `fish/conf.d/`, blocco in `$PROFILE` di pwsh | Stesso modello di `profile_block`/`update_profile_text` in `crates/core/src/desktop.rs`, generalizzato. |
| File dell'app | `~/Library/Application Support/CliSuperpower/` (config, team, generated, manifest, backups) | È quello che restituiscono già `data_dir()` e `local_data_dir()` di Tauri usati in `app/src-tauri/src/lib.rs::run`. |
| Autocomplete/cronologia (equivalente di PSReadLine/Clink) | zsh: **zsh-autosuggestions** + hook `zshaddhistory`; fish: nativo; bash: `HISTIGNORE` + **ble.sh** opzionale; pwsh: PSReadLine (codice esistente) | Riusare tool maturi (principio 5). |
| Prompt | **Oh My Posh** come su Windows (`oh-my-posh init <shell>` per zsh, bash, fish, pwsh) | Stesso generatore `oh_my_posh_config`; un solo motore da mantenere. |
| Installazione tool | **Homebrew** (`brew install` senza sudo); mai installare Homebrew in automatico | Equivalente di winget; Homebrew richiede amministratore e Xcode CLT. |
| `csp port` | API `libproc` (crate `libproc`) + `kill(SIGTERM)` | Equivalente diretto di `GetExtendedTcpTable` + `TerminateProcess`. |
| Pacchetto | `.app` + `.dmg` **universal** (arm64 + x86_64), firmato **Developer ID**, **notarizzato**, hardened runtime, **senza** App Sandbox | Gatekeeper blocca le app non notarizzate in modo molto più duro di SmartScreen; la sandbox è incompatibile con la scrittura di `~/.zshrc` e delle preferenze di altre app. |
| Aggiornamenti | `tauri-plugin-updater` con artefatti `.app.tar.gz` firmati, chiavi `darwin-aarch64`/`darwin-x86_64` in `latest.json` **[condivisa con Linux]** | Stesso meccanismo previsto in `PROGETTO.md` §6 (oggi non ancora configurato su nessuna piattaforma). |
| Schema | Un solo salto **v6 → v7 "cross-platform"** **[condivisa con Linux]** | Nuovi valori di shell, percorsi POSIX, scelta dei terminali, nuovi tool. |

---

## 2. Inventario: cosa fa oggi l'app e dove

| Funzione | Stato nel codice | Punti di codice principali |
|---|---|---|
| F1 Aspetto per profilo | Implementato (fragment WT, immagini, padding, conflitti con `settings.json`) | `generators::terminal_fragment`, `generators::terminal_images`, `Desktop::plan`, `Desktop::upload_background`, `TerminalSettings::{parse, add_inherited_profiles, conflicts}` in `desktop.rs` |
| Profilo predefinito | Implementato (modifica CST di `settings.json`) | `Desktop::plan` (ramo `defaultProfile`, `jsonc_parser::cst`) |
| F2 Comandi rapidi | Implementato per pwsh/powershell (funzioni) e cmd (Clink, argomenti fissi) | `generators/powershell.rs::{powershell_init, append_command, declaration, literal}`, `generators/invoke-native.ps1`, `generators/clink.rs::clink_script_with_theme` |
| F3 Strati team/personale | Implementato, indipendente dall'OS | `config.rs::{resolve, merge_items, migrate}`, `Desktop::{preview_team, install_team, team_candidate}` |
| F4 Workspace | Implementato (argomenti `wt`), avvio dalla UI | `generators::{workspace_args, append_pane, wt_value}`, `Desktop::launch` (`wt.exe`) |
| F5 Prompt | Implementato con Oh My Posh in pwsh/powershell e cmd | `generators/prompt.rs::{oh_my_posh_config, posh_segment}`, `desktop.rs::OH_MY_POSH_INIT`, `generators/clink.rs::prompt_script` |
| Autocomplete/cronologia (`shellBehavior`, anticipo di F9) | Implementato (PSReadLine, Clink) | `generators/behavior.rs::{behavior, SENSITIVE}`, `powershell.rs::{append_behavior, TAB_ACCEPT, HISTORY_FILTER}`, `clink.rs::behavior_script` |
| Banner | Implementato | `generators/banner.rs::{powershell, lua, banner_enabled}` |
| F6 `csp port` | Implementato solo Windows (`#[cfg(windows)] mod port`) | `crates/csp/src/port.rs::{listeners, tcp_table, run, terminate, verify_listener}`, `crates/csp/src/main.rs::run_port` |
| F7 Tool esterni | Implementato con winget; su non-Windows uno stub che non rileva nulla | `tools.rs::{CATALOG, statuses, install, uninstall, install_recommended}`, `tools.rs::platform` (`#[cfg(windows)]` e `#[cfg(not(windows))]`) |
| F8 Backup/ripristino/disinstallazione | Implementato, quasi indipendente dall'OS | `apply.rs::{apply, commit, recover, history, restore_latest, removals, lock, atomic_write, read}` |
| Clink incluso nell'app | Implementato (copia in `tools/clink`, AutoRun del registro) | `clink.rs::{enable, disable, profile_directory, autorun_values, injection}`, `app/scripts/fetch-clink.ps1`, `tauri.conf.json` (`resources`) |
| Aggiornamento app | **Non configurato** (`bundle.active: false`, nessun plugin updater) | `app/src-tauri/tauri.conf.json`, `app/src-tauri/Cargo.toml` |
| Distribuzione di `csp` | **Non configurata** (nessun `externalBin`) | `tauri.conf.json` |

Osservazioni trasversali utili al porting:

- La configurazione è gestita come `serde_json::Value` validato a mano (`config/validate.rs`), non con struct Rust esposte alla UI; i tipi TypeScript in `app/src/app/models.ts` sono scritti a mano (non c'è ancora tauri-specta/ts-rs). "Rigenerare i tipi" oggi significa aggiornare `models.ts` a mano.
- I test dei generatori sono asserzioni `contains`/`assert_eq` dentro i moduli, non snapshot `insta` come previsto da `PROGETTO.md` §7. Per i nuovi generatori macOS si raccomanda di introdurre snapshot veri (vedi §9).
- Il core compila già su non-Windows grazie agli stub (`tools.rs::platform` non-windows, `csp::run_port` non-windows, `cfg(windows)` in `clink.rs::hide_window` e `desktop.rs::shell_profile`). È probabile che `cargo test --workspace` passi già su macOS, ma **non è verificato**: è il primo task del piano.

---

## 3. Mappa dei punti dipendenti da Windows

Legenda difficoltà: **B** bassa, **M** media, **A** alta.

| # | Punto | Dove | Cosa fa oggi | Equivalente macOS | Diff. |
|---|---|---|---|---|---|
| W1 | Cartelle dell'app | `app/src-tauri/src/lib.rs::run` (`data_dir`, `local_data_dir`, `resource_dir`) → `Desktop::new(data, local, system_local)` | `%APPDATA%\CliSuperpower`, `%LOCALAPPDATA%\CliSuperpower`, `%LOCALAPPDATA%` | `data_dir()` e `local_data_dir()` valgono entrambi `~/Library/Application Support`: `data` e `local` coincidono in `.../CliSuperpower` (nessuna collisione di nomi file: verificato sull'elenco dei file scritti). `system_local` perde significato: serve una struct `PlatformPaths` con `home` esplicita. | B |
| W2 | Apertura link | `lib.rs::open_link` | `rundll32 url.dll,FileProtocolHandler` | `tauri-plugin-opener` (multipiattaforma) oppure `/usr/bin/open`. | B |
| W3 | Rilevamento terminale | `Desktop::profiles` | Legge `settings.json` di WT (tre percorsi `Packages/...`), GUID dei profili | Rilevare iTerm2 (`/Applications/iTerm.app`, `~/Applications`, bundle id `com.googlecode.iterm2`) e Terminal.app; i "profili" diventano le shell installate. | M |
| W4 | Aspetto | `generators::terminal_fragment`, `terminal_images`, `Desktop::plan` | Fragment con `updates` su GUID esistenti, immagini copiate accanto | iTerm2: Dynamic Profile **nuovo** per ogni shell (non può aggiornare un profilo esistente), immagine referenziata con percorso assoluto. Terminal.app: profili gestiti nel dominio `com.apple.Terminal` (S3). | A |
| W5 | Profilo predefinito | `Desktop::plan` (CST di `settings.json`) | Chiave `defaultProfile` | iTerm2: chiave `Default Bookmark Guid` nel dominio `com.googlecode.iterm2`; Terminal.app: `Default Window Settings` e `Startup Window Settings`. Scrittura tramite `defaults`/CFPreferences, mai sul file `.plist` (cfprefsd tiene una cache). Richiede un nuovo tipo di modifica nell'applicatore (§4.10). | A |
| W6 | Conflitti con impostazioni manuali | `TerminalSettings::conflicts` | Avvisa se `settings.json` prevale sul fragment | iTerm2: i profili dinamici sono nostri, nessuna precedenza; l'avviso diventa "le modifiche fatte a mano a un profilo dinamico vengono perse al ricaricamento" (da verificare in S2). | B |
| W7 | Workspace | `generators::workspace_args`, `append_pane`, `Desktop::launch` | Argomenti `wt`, `pwsh.exe -EncodedCommand`, `cmd.exe /d /s /k` | Generatore puro di AppleScript per iTerm2 (`create window/tab with profile`, `split vertically/horizontally`, `write text`) avviato con `/usr/bin/osascript`; Terminal.app: una finestra per tab, niente split. | M |
| W8 | Profilo della shell | `desktop.rs::shell_profile` | Avvia `{shell}.exe -NoProfile -Command` per `$PROFILE`, policy, collisioni | zsh: `zsh -c 'print -r -- ${ZDOTDIR:-$HOME}'`; bash: regola dei file di login; fish: `fish -c 'echo $__fish_config_dir'`; pwsh: stesso script ma eseguibile `pwsh` (non `pwsh.exe`), policy irrilevante su Unix. Collisioni: `whence -w` / `type -t` / `type -q`. | M |
| W9 | Blocco nel profilo | `desktop.rs::{profile_block, update_profile_text, START, END}` | Marcatori, BOM/UTF-16, newline CRLF di default, percorso `$env:LOCALAPPDATA\CliSuperpower\generated\…` **cablato** | Stessi marcatori `# >>> CLI Superpower >>>` (commento valido in zsh/bash/fish/pwsh); newline LF; percorso assoluto quotato passato come parametro; niente BOM. | B |
| W10 | Script di avvio | `generators/powershell.rs::powershell_init`, `invoke-native.ps1` | Funzioni PowerShell, PSReadLine, banner | Nuovi generatori puri `zsh_init`, `bash_init`, `fish_init`. Per pwsh su macOS l'helper nativo va rivisto (gestione `.cmd/.bat`, `[Environment]::SystemDirectory`, quoting CRT): usare `ProcessStartInfo.ArgumentList`. | M |
| W11 | Prompt | `generators/prompt.rs::posh_segment` | Stile `full` sostituisce `/` con `\` | Su macOS lo stile `full` deve lasciare `/`: il generatore riceve il "sapore" dei percorsi come parametro. `desktop.rs::OH_MY_POSH_INIT` cerca `oh-my-posh.exe`. | B |
| W12 | cmd / Clink | `clink.rs` (registro `HKCU/HKLM\...\Command Processor\AutoRun`, `clink_*.exe`), `generators/clink.rs`, `fetch-clink.ps1`, `tauri.conf.json` `resources` | Clink incluso e attivato per cmd | **Nessun equivalente**: cmd non esiste. Le funzioni che Clink fornisce a cmd sono fornite nativamente da zsh/fish o da plugin (§4.3). Tutto il ramo Clink va escluso dalla build macOS. | B |
| W13 | Tool | `tools.rs::CATALOG` (`winget_id`, eseguibili `.exe`), `platform::{winget, detect, search_paths, file_metadata}` (FileVersionInfo, font dal registro) | winget, rilevamento in PATH/ProgramFiles | Homebrew (`brew`), eseguibili senza estensione, PATH dell'app GUI da ricostruire, versione da `--version` o `brew list --versions`, Nerd Font da `~/Library/Fonts` e `/Library/Fonts`. | M |
| W14 | `csp port` | `crates/csp/src/port.rs`, `Cargo.toml` (`windows-sys`) | `GetExtendedTcpTable`, `OpenProcess`, `QueryFullProcessImageNameW`, `GetProcessTimes`, `IsProcessCritical`, `TerminateProcess` | `libproc`: elenco PID, descrittori socket TCP in `LISTEN`, `proc_pidpath`, ora di avvio da `proc_bsdinfo`; `kill(pid, SIGTERM)`; nessun "critical process": rifiutare PID 1 e processi di altri utenti con messaggio chiaro. | M |
| W15 | Validazione percorsi | `config/validate.rs::{windows_path, drive_path, unc_path, program}`; `app/src/app/config-validation.ts` (`absolutePath`, `pathHelp`) | Solo percorsi Windows assoluti | Accettare anche percorsi POSIX assoluti (§6). | B |
| W16 | Valori di shell/profilo | `validate.rs::PROFILES`, `config.rs::TOOLS`, `models.ts` (`Profile`), `app.ts` (`profiles`, `behaviorShells`) | `pwsh`, `powershell`, `cmd` | Aggiungere `zsh`, `bash`, `fish` (§6). | M |
| W17 | Codifiche | `desktop.rs::text_file(…, bom)` (BOM per `init-powershell.ps1`), `profile_block` (UTF-16) | Necessari per Windows PowerShell 5.1 | Su macOS UTF-8 senza BOM, LF. Il codice esistente resta per Windows. | B |
| W18 | Build e CI | `.github/workflows/ci.yml` (solo `windows-latest`), `app/package.json` (`fetch-clink` con `powershell`), `tauri.conf.json` (`beforeDevCommand`/`beforeBuildCommand` chiamano `fetch-clink`), icona solo `.ico` | Solo Windows | Job `macos-latest`; `fetch-clink` solo su Windows (`tauri.windows.conf.json`); icone `.icns`; bundle `app`/`dmg`. | B |
| W19 | UI: testi e anteprime | `app.html` (righe ~47–53, 75, 128, 156, 167–168, 205–215, 261–262), `app.ts` (`profiles` r.19, `pathStyles`, `behaviorShells` r.267, `terminalFonts` r.352, `appearanceThemes` r.342, `npm.cmd` r.440 e r.490), `styles.css` (Segoe UI, Consolas), `config-bridge.ts` (mock con `C:\\CLI-Superpower\\`) | Tutto pensato per Windows | Testi e opzioni guidati dalla piattaforma (§8). | M |
| W20 | Distribuzione | `PROGETTO.md` §7 e §9 (NSIS per utente, SmartScreen) | Installer NSIS non ancora attivo | `.dmg` universal, firma Developer ID, notarizzazione, updater (§4.12). | A |

---

## 4. Lo stesso prodotto sui tre strati di macOS

### 4.1 Strato 1 — Terminali

| Terminale | Dove si configura | Iniezione non distruttiva | Aspetto per profilo | Workspace (tab + split) | Profilo predefinito | Verdetto |
|---|---|---|---|---|---|---|
| **iTerm2** | Preferenze in `com.googlecode.iterm2` + **Dynamic Profiles** in `~/Library/Application Support/iTerm2/DynamicProfiles/*.json` | File nostro nella cartella dei profili dinamici, ricaricato a caldo; rimozione = cancellare il file | Sì: colori ANSI, font, `Transparency`, `Blur`, immagine (`Background Image Location`, `Blend`), cursore, margini (2 valori) | Sì: AppleScript nativo con split | Chiave `Default Bookmark Guid` | **MVP, completo** |
| **Terminal.app** | Dominio `com.apple.Terminal`, chiave `Window Settings` (dizionario di profili); colori e font come `NSData` archiviati con `NSKeyedArchiver` | Profili con nome nostro dentro una chiave condivisa; scrittura solo via `defaults`/CFPreferences; rischio che Terminal aperto riscriva le sue preferenze (S3) | Parziale: colori, font, opacità del fondo, sfocatura (chiave da verificare in S3), cursore. **Niente immagine, niente padding** | Finestre separate con `do script`; le tab richiedono System Events (permesso Accessibilità) → no; **niente split** | `Default Window Settings`, `Startup Window Settings` | **MVP ridotto** |
| WezTerm | `~/.wezterm.lua` o `$XDG_CONFIG_HOME/wezterm/wezterm.lua` (Lua) | Blocco che fa `dofile` di un nostro modulo e modifica la tabella di config: fragile se l'utente non usa la forma `config_builder` | Sì (per dominio/launch menu) | Ottimo: `wezterm cli spawn/split-pane` | Sì (Lua) | Post-MVP **[condivisa con Linux]** |
| Ghostty | `~/.config/ghostty/config` (anche `~/Library/Application Support/com.mitchellh.ghostty/config`) | Una riga `config-file = "…"` tra marcatori | **No profili**: un solo aspetto globale | Scripting limitato | — | Post-MVP **[condivisa con Linux]** |
| Kitty | `~/.config/kitty/kitty.conf` | `include` di un nostro file tra marcatori | Globale; per finestra con `--override` | Sì: file di sessione (`kitty --session`) | — | Post-MVP **[condivisa con Linux]** |
| Alacritty | `~/.config/alacritty/alacritty.toml` | `[general] import = [...]` (formato del campo cambiato tra versioni) | Globale | **No tab né split** | — | Escluso |

**Perché iTerm2 + Terminal.app nell'MVP.** La UX di Windows è "scegli l'aspetto per shell, premi Applica, apri il terminale": iTerm2 la riproduce quasi 1:1 (i profili dinamici sono l'equivalente dei fragment, l'AppleScript dell'equivalente `wt new-tab ; split-pane`). Terminal.app non si può ignorare perché è quello che l'utente ha già, ma il suo formato rende costoso e rischioso andare oltre l'aspetto di base. I terminali "di configurazione testuale" sono più facili da gestire e utili anche su Linux, quindi conviene progettarli come adapter comuni in una fase successiva.

**Differenza di UX da gestire.** Su Windows il fragment *aggiorna* il profilo PowerShell che l'utente apre già. In iTerm2 un profilo dinamico è un profilo *nuovo* ("CLI Superpower · zsh", con `Dynamic Profile Parent Name` = il profilo predefinito dell'utente per ereditarne il resto — da verificare in S2). Perché l'utente lo veda senza fare nulla, l'app propone in anteprima di impostarlo come profilo predefinito (riuso di `globalTerminal.defaultProfile`, oggi già presente nello schema). Con il ripristino il GUID predefinito originale torna al suo posto.

**GUID stabili.** I GUID dei profili dinamici vanno derivati in modo deterministico dall'ID della shell (UUID v5 con namespace fisso dell'app): stessa configurazione → stesso file → idempotenza, e le scorciatoie dell'utente che puntano al profilo sopravvivono a un nuovo Applica.

**Immagini.** Oggi `terminal_images` copia l'immagine accanto al fragment perché WT la risolve relativamente alla cartella del fragment. iTerm2 accetta un percorso assoluto: si può puntare direttamente al file già caricato in `…/CliSuperpower/backgrounds/<sha256>.<ext>` da `Desktop::upload_background`, senza copia.

**Mappatura dei campi di `appearance` (iTerm2).**

| Campo | iTerm2 | Note |
|---|---|---|
| `colorScheme` | Chiavi `Ansi 0..15 Color`, `Foreground Color`, `Background Color`, `Cursor Color`… come dizionari di componenti | Oggi è il **nome** di uno schema WT. Serve un catalogo di schemi con i valori nel core (§7). |
| `font.face`, `font.size` | `Normal Font` = `"<PostScriptName> <size>"` | Il nome PostScript differisce dal nome famiglia mostrato all'utente (es. `MesloLGS-NF-Regular`): risoluzione da fare nella scoperta (CoreText), non nel generatore. |
| `opacity` (0–100) | `Transparency` = `1 - opacity/100` | |
| `useAcrylic` | `Blur` = true, `Blur Radius` | Approssimazione: la UI lo chiama "Sfocatura" su macOS. |
| `backgroundImage` | `Background Image Location` (assoluto) | |
| `backgroundImageOpacity` | `Blend` | Semantica diversa (miscela con il colore di fondo): approssimazione. |
| `backgroundImageAlignment` | `Background Image Mode` (stretch/tile/fill/fit) | Le 9 posizioni WT non esistono: mappare `center`→fill, gli altri→fill con avviso in anteprima. |
| `cursorShape` | `Cursor Type` (sottolineato, barra, blocco) | `bar`→barra, `underscore`/`doubleUnderscore`→sottolineato, `filledBox`/`emptyBox`/`vintage`→blocco, con avviso per le approssimazioni. |
| `padding` | `Side Margins`, `Top/Bottom Margins` | Due valori: max(left,right), max(top,bottom), con avviso se diversi. |

I codici numerici di `Cursor Type` e `Background Image Mode` e il nome esatto delle chiavi vanno confermati in S2 leggendo un profilo esportato da iTerm2.

### 4.2 Strato 2 — Shell e punti di iniezione

macOS apre **shell di login** in ogni nuova finestra di Terminal.app e iTerm2. Questo cambia quale file viene letto.

| Shell | Disponibilità | File letto dalle finestre del terminale | Dove iniettare | Come trovarlo a runtime |
|---|---|---|---|---|
| **zsh** | `/bin/zsh` di sistema (5.9 sulle versioni recenti) | `.zshenv` → `.zprofile` → `.zshrc` → `.zlogin` sotto `${ZDOTDIR:-$HOME}` | **`.zshrc`** (letto da ogni shell interattiva, login e non) | `zsh -c 'print -r -- ${ZDOTDIR:-$HOME}'` (legge solo `.zshenv`, dove di solito si imposta `ZDOTDIR`): equivalente di "chiedere `$PROFILE` alla shell" |
| **bash** | `/bin/bash` **3.2.57** (vecchia, GPLv2); bash 5 via Homebrew | Login: il **primo esistente** tra `.bash_profile`, `.bash_login`, `.profile`. Non-login: `.bashrc` | Il file di login effettivo **e** `.bashrc`, entrambi con lo stesso blocco protetto da una guardia (`__CSP_INIT_LOADED`) | Regola applicata dal planner; se nessuno esiste si crea `.bash_profile` **solo** se manca anche `.profile` (crearlo oscurerebbe `.profile`) |
| **fish** | Homebrew (`/opt/homebrew/bin/fish` o `/usr/local/bin/fish`) | `conf.d/*.fish` poi `config.fish` sotto `$__fish_config_dir` | **File nostro** `conf.d/cli-superpower.fish`: nessun file utente toccato | `fish -c 'echo $__fish_config_dir'` (rispetta `XDG_CONFIG_HOME`) |
| **pwsh** | Homebrew cask `powershell` o pkg Microsoft (`/usr/local/bin/pwsh`) | `$PROFILE.CurrentUserCurrentHost` = `~/.config/powershell/Microsoft.PowerShell_profile.ps1` | Blocco in `$PROFILE`, come oggi | `shell_profile` esistente, ma con eseguibile `pwsh` e senza `Get-ExecutionPolicy` (su Unix è sempre `Unrestricted`) |

Blocco proposto per zsh e bash (stessi marcatori di `desktop.rs::START`/`END`, LF, percorso assoluto calcolato dall'applicatore e passato al generatore):

```sh
# >>> CLI Superpower >>> (non modificare: gestito automaticamente)
if [ -r '/Users/nome/Library/Application Support/CliSuperpower/generated/init.zsh' ]; then
    . '/Users/nome/Library/Application Support/CliSuperpower/generated/init.zsh'
fi
# <<< CLI Superpower <<<
```

Il percorso contiene uno spazio (`Application Support`): va sempre quotato. In alternativa si possono collocare gli script generati sotto `~/.config/cli-superpower/` (XDG, senza spazi, uguale a Linux); la raccomandazione è di restare nella cartella dell'app e quotare, perché tutto ciò che appartiene all'app resta in un solo posto e la disinstallazione è più chiara. Decisione da confermare con la sessione Linux **[condivisa con Linux]**.

**Posizione del blocco.** `update_profile_text` oggi lo accoda in fondo: su macOS è la scelta giusta, perché il nostro prompt e i nostri binding devono venire *dopo* oh-my-zsh, prezto o powerlevel10k. Conflitti noti da segnalare in anteprima (rilevando il contenuto del file, senza modificarlo): powerlevel10k *instant prompt* (vieta output durante l'avvio → il banner va stampato al primo `precmd`, S6), un altro motore di prompt attivo (starship, p10k) quando l'utente attiva il prompt di CLI Superpower, `zsh-autosuggestions` già caricato da un plugin manager.

### 4.3 Autocomplete e cronologia: l'equivalente di PSReadLine e Clink

Il generatore comune `generators/behavior.rs::behavior` produce già una struttura neutra (`predictions`, `view`, `accept_with_tab`, `patterns`): basta un consumatore per ogni shell.

| Campo `shellBehavior` | zsh | fish | bash | pwsh (macOS) |
|---|---|---|---|---|
| `predictions: history` | Carica **zsh-autosuggestions** (se non è già caricato: `(( ${+functions[_zsh_autosuggest_start]} ))`), `ZSH_AUTOSUGGEST_STRATEGY=(history)` | Nativo (autosuggestions attive di default) | Solo con **ble.sh** (pesante, opzionale); altrimenti non disponibile | PSReadLine, codice esistente |
| `predictions: off` | Non carica il plugin / lo disattiva se caricato (S8) | `set -g fish_autosuggestion_enabled 0` (fish ≥ 3.4, S7) | Nulla da fare | Esistente |
| `predictionView: inline` | Comportamento del plugin | Nativo | ble.sh | Esistente |
| `predictionView: list` | **Non disponibile** nell'MVP (zsh-autocomplete è in conflitto con zsh-autosuggestions) | **Non disponibile** (il pager di fish elenca completamenti, non cronologia) | Non disponibile | Esistente |
| `acceptKey: right` | Default del plugin (`forward-char` accetta) | Default | ble.sh | Esistente |
| `acceptKey: tab` | Widget `__csp_tab_accept`: se `$POSTDISPLAY` non è vuoto `zle autosuggest-accept`, altrimenti il widget che Tab aveva prima (memorizzato una volta sola, come `$global:__CspTabFallback` in `TAB_ACCEPT`) | `bind \t` con `commandline --showing-suggestion` (versione minima da verificare, S7) | ble.sh | Esistente |
| `hideSensitive` / `historyExclude` | Hook **`zshaddhistory`** (via `add-zsh-hook`, idempotente): ritorno **2** = riga tenuta in memoria ma non scritta su file, equivalente esatto di `MemoryOnly` (S7) | Funzione `fish_should_add_to_history` (versione minima da verificare, S7) | `HISTIGNORE` con pattern resi insensibili alle maiuscole (`*[Pp][Aa][Ss][Ss]…*`): la riga **non** entra nemmeno nella cronologia di sessione (differenza da documentare) | Esistente (`HISTORY_FILTER`) |

Le opzioni non disponibili vengono esposte alla UI come **capacità** per shell (§8), così l'editor non le offre invece di fallire. **atuin** (cronologia in SQLite con ricerca) e **fzf** (Ctrl+R) restano tool di F9/F11, come su Windows.

### 4.4 Prompt (F5)

- Generatore invariato: `oh_my_posh_config` produce lo stesso JSON. Unica correzione: lo stile `full` di `posh_segment` usa `replace "/" "\\"`, che su macOS darebbe percorsi con backslash. Il generatore deve ricevere la piattaforma (o un `PathFlavor`) come parametro, non leggerla dall'ambiente.
- Inizializzazione per shell, generata nello script di avvio e protetta da `command -v oh-my-posh`:
  - zsh: `eval "$(oh-my-posh init zsh --config '<tema>')"`
  - bash: `eval "$(oh-my-posh init bash --config '<tema>')"`
  - fish: `oh-my-posh init fish --config '<tema>' | source`
  - pwsh: `OH_MY_POSH_INIT` esistente con `oh-my-posh` al posto di `oh-my-posh.exe`.
- Il file del tema resta `generated/prompt.omp.json` (`Desktop::prompt_theme`).
- Con bash 3.2 Oh My Posh funziona ma la durata dei comandi richiede hook (`PROMPT_COMMAND`/`DEBUG` trap) più fragili: da verificare in S6.

### 4.5 Comandi rapidi (F2)

Il contratto dello schema (template ad argomenti, parametri posizionali, `{{`/`}}`, nessuna reinterpretazione dei valori) si traduce bene nelle shell POSIX, meglio che in cmd: a differenza di Clink, **i parametri sono supportati**.

Schema della funzione zsh generata (bash analogo, fish con `function … ; end` e `$argv`):

```zsh
function mig {
    emulate -L zsh
    if (( $# < 1 )); then print -u2 -- 'Uso: mig <nome>'; return 2; fi
    local -a __csp_args
    __csp_args=(ef migrations add "$1")
    command -- dotnet "${__csp_args[@]}"
}
```

- **Programma**: `command --` salta funzioni e alias, come `Get-Command -CommandType Application` in `invoke-native.ps1`; un percorso assoluto si esegue direttamente.
- **`workingDirectory`**: subshell `( cd -- '<dir>' && command -- … )`, così la cartella torna com'era anche in caso di errore (requisito dello schema).
- **`changeDirectory`**: funzione con `cd -- '<dir>'` nella shell corrente (equivalente di `Set-Location`).
- **Parametri facoltativi senza default**: argomento aggiunto solo se presente (`(( $# >= 2 )) && __csp_args+=("$2")`), come `$PSBoundParameters.ContainsKey`.
- **`integer`**: verifica con pattern (`[[ $1 == (|-)<-> ]]` in zsh, `=~ ^-?[0-9]+$` in bash) e intervallo i32.
- **Quoting**: letterali tra apici singoli con `'` → `'\''` (zsh/bash); fish con `\'` e `\\`. Un NUL è un errore del generatore (come oggi).
- **Help**: PowerShell offre `Get-Help mig`. Nelle shell POSIX non c'è un equivalente nativo: l'MVP stampa l'uso quando mancano argomenti; l'help completo arriva con `csp help`/`csp list` (F10), che legge la configurazione effettiva.
- **Nomi riservati**: oltre alle parole chiave, evitare nomi che coincidono con builtin critici (`cd`, `exec`, `command`, `builtin`, `eval`, `source`, `.`); l'elenco va nel generatore come oggi `KEYWORDS`.
- **Prefisso `/`**: comportamento da verificare in S1. Attesa: in bash una funzione con `/` nel nome è definibile fuori dalla modalità POSIX ed è invocabile; in zsh va provato (la ricerca dei comandi con `/` nel nome può saltare le funzioni); fish rifiuta probabilmente `/` nei nomi di funzione. Se il prefisso non è supportato da una shell, il core lo segnala come fa già ("verifica che il prefisso sia supportato in ogni shell di destinazione" in `docs/config-schema.md`) e l'utente usa `x-` o nessun prefisso.
- **Collisioni**: `whence -w <nome>` (zsh), `type -t` (bash), `type -q` (fish), lanciati senza file utente (`zsh -f`, `bash --noprofile --norc`, `fish --no-config`): stesso limite di oggi con `-NoProfile`.

### 4.6 Workspace (F4)

Generatore puro `iterm2_workspace_script(config, id, profiles) -> String` che produce AppleScript; l'applicatore non è coinvolto (non scrive file), `Desktop::launch` esegue `/usr/bin/osascript` passando lo script su stdin.

```applescript
tell application "iTerm2"
    set w to (create window with profile "CLI Superpower · zsh")
    tell current session of w
        set name to "Sviluppo"
        write text "cd -- '/Users/nome/src/efm/frontend' && ng serve"
        set p2 to (split vertically with profile "CLI Superpower · zsh")
    end tell
    tell p2 to write text "cd -- '/Users/nome/src/efm/backend' && dotnet watch"
    tell w to set t2 to (create tab with profile "CLI Superpower · zsh")
end tell
```

- `right` → `split vertically`, `down` → `split horizontally` (stessa semantica di `--vertical`/`--horizontal` in `append_pane`).
- `startupCommand` è già "sintassi della shell scelta" nello schema: con `write text` gira nella shell interattiva, quindi può usare anche i comandi rapidi. Sostituisce la codifica `-EncodedCommand` di Windows.
- Escaping AppleScript: `\` e `"` dentro le stringhe; il comando shell viene prima quotato per la shell e poi per AppleScript (due livelli, entrambi nel generatore, con test dedicati).
- Terminal.app: `tell application "Terminal" to do script "…"` apre una finestra per tab; gli split vengono ignorati con un avviso in anteprima.
- Permesso **Automation** (TCC): al primo avvio macOS chiede "CLI Superpower vuole controllare iTerm". Servono `NSAppleEventsUsageDescription` in `Info.plist` e l'entitlement `com.apple.security.automation.apple-events` con hardened runtime (S4). Se l'utente nega, `osascript` fallisce con errore -1743: messaggio chiaro con il percorso Impostazioni di Sistema → Privacy e sicurezza → Automazione.

### 4.7 Strato 3 — Tool e Homebrew (F7)

| Aspetto | Windows oggi | macOS |
|---|---|---|
| Gestore | `winget.exe` in `%LOCALAPPDATA%\Microsoft\WindowsApps` (`platform::winget`) | `brew` in `/opt/homebrew/bin` (arm64) o `/usr/local/bin` (Intel); variabile `HOMEBREW_PREFIX` se presente |
| Assente | Link manuale | Link a https://brew.sh. **Mai** installare Homebrew dall'app (richiede amministratore, Xcode CLT, script remoto) |
| Installazione | `winget install --id … --scope user --disable-interactivity` | `brew install <formula>` con `HOMEBREW_NO_AUTO_UPDATE=1`, `HOMEBREW_NO_ENV_HINTS=1`, `NONINTERACTIVE=1`, stdin nullo; cask solo se non richiedono password |
| Disinstallazione | Solo tool installati dall'app (`installed-tools.json`) | Identico: stesso `STATE_FILE`, `brew uninstall <formula>` |
| Rilevamento | PATH + ProgramFiles + `FileVersionInfo` via PowerShell | PATH **ricostruito** (vedi sotto) + prefissi Homebrew; versione con `<tool> --version` (timeout) o `brew list --versions` |
| Nerd Font | Registro `...\Fonts` | Scansione di `~/Library/Fonts` e `/Library/Fonts` (nome con "Nerd Font"/"NF"), oppure CoreText |

Catalogo macOS proposto (nomi da confermare in S5, come fatto per winget-pkgs):

| ID | Pacchetto Homebrew | Note |
|---|---|---|
| `oh-my-posh` | formula `oh-my-posh` | Consigliato (`RECOMMENDED`), come su Windows |
| `starship`, `zoxide`, `fzf`, `tealdeer`, `eza`, `bat`, `jq`, `lazygit` | formule omonime | |
| `nerd-font` | cask `font-meslo-lg-nerd-font` o `font-caskaydia-cove-nerd-font` | Installa in `~/Library/Fonts`, senza sudo |
| `pwsh` | cask `powershell` | Installer `.pkg` che chiede la password: **solo manuale** dall'app |
| `clink` | — | Non applicabile: nascosto su macOS |
| nuovi: `zsh-autosuggestions`, `fish`, `bash`, `ble-sh` (opz.), `atuin` (v2) | formule omonime (`ble.sh` non è in homebrew-core: installazione manuale) | Richiedono nuovi ID nello schema (§6) |

**PATH dell'app GUI.** Un'app avviata dal Dock eredita un PATH minimo (`/usr/bin:/bin:/usr/sbin:/sbin`), non quello della shell. Senza correzione l'app non trova `brew`, `fish`, `pwsh`, `oh-my-posh`. Strategia (S5): `/usr/libexec/path_helper -s` + prefissi Homebrew noti + `~/.local/bin`, `~/.cargo/bin`; in aggiunta, best-effort con timeout, `zsh -lc 'print -r -- $PATH'` (legge `.zprofile`, dove sta di solito `brew shellenv`). Il PATH risolto si passa esplicitamente ai processi figli; non si modifica l'ambiente globale del processo.

### 4.8 `csp port` (F6)

| Passo | Windows (`port.rs`) | macOS |
|---|---|---|
| Elenco listener | `GetExtendedTcpTable(TCP_TABLE_OWNER_PID_LISTENER)` | `proc_listpids` → per ogni PID `proc_pidinfo(PROC_PIDLISTFDS)` → `proc_pidfdinfo(PROC_PIDFDSOCKETINFO)` → socket TCP in stato `LISTEN` sulla porta (crate `libproc`, API esatte da confermare in S9) |
| Visibilità | Tutti i PID; dettagli limitati per altri utenti | Solo i processi dell'utente sono ispezionabili senza root: i listener di altri utenti non compaiono → messaggio "potrebbero esserci processi di sistema non visibili senza privilegi". Alternativa più completa: `lsof -nP -iTCP:<porta> -sTCP:LISTEN -F pcn`, con gli stessi limiti senza `sudo` |
| Percorso | `QueryFullProcessImageNameW` | `proc_pidpath` |
| Ora di avvio | `GetProcessTimes` | `proc_bsdinfo.pbi_start_tvsec` |
| Processo critico | `IsProcessCritical` | Rifiutare PID 1 (`launchd`) e i processi di altri utenti; `EPERM` → messaggio chiaro, mai elevare |
| Terminazione | `TerminateProcess` + `WaitForSingleObject` 5 s, verifica dell'ora di avvio contro il riuso del PID | `kill(pid, SIGTERM)`, attesa fino a 5 s con controllo periodico, stessa verifica dell'ora di avvio; nessun `SIGKILL` automatico |
| Interazione | `termina <PID>` solo con stdin/stdout terminale (`run`, `confirmed_pid`) | Identica (codice comune) |

Il codice non specifico (`parse_port`, `confirmed_pid`, formattazione dell'età, flusso di `run`) va spostato in un modulo comune; restano per-OS solo `listeners`, `process_info` e `terminate`. Linux avrà la sua implementazione (`/proc/net/tcp*`) con la stessa interfaccia **[condivisa con Linux]**.

### 4.9 Percorsi a runtime

| Cosa | Windows | macOS | Come si ottiene |
|---|---|---|---|
| Dati dell'app (config, team) | `%APPDATA%\CliSuperpower` | `~/Library/Application Support/CliSuperpower` | `app.path().data_dir()` (già usato) |
| Stato (manifest, history, generated, backgrounds, tools) | `%LOCALAPPDATA%\CliSuperpower` | stessa cartella | `app.path().local_data_dir()` (già usato; coincide con `data_dir` su macOS) |
| Home | — | `$HOME` / `getpwuid` | Nuovo campo di `PlatformPaths` (oggi non esiste) |
| Profili dinamici iTerm2 | — | `~/Library/Application Support/iTerm2/DynamicProfiles` | Derivato da home; verificare che iTerm2 non ne permetta lo spostamento (S2) |
| Rc zsh | `$PROFILE` | `${ZDOTDIR:-$HOME}/.zshrc` | Chiesto a zsh |
| Config fish | — | `$__fish_config_dir` | Chiesto a fish |
| `$PROFILE` pwsh | chiesto a pwsh | chiesto a pwsh | `shell_profile` |
| Risorse dell'app | `resource_dir()/clink` | `CliSuperpower.app/Contents/Resources` | `resource_dir()` (Clink assente su macOS) |

Regola già in `CLAUDE.md`: nessun percorso cablato. In particolare **non** si deve riprodurre su macOS il percorso cablato `$env:LOCALAPPDATA\CliSuperpower\generated\…` di `update_profile_text`: il blocco riceve il percorso assoluto dallo strato che lo conosce.

### 4.10 Applicatore su POSIX

`apply.rs` è in gran parte portabile (`File::lock` della libreria standard, `fs::rename` atomico sullo stesso volume, SHA-256, journal `pending.json`). Servono però correzioni **[condivisa con Linux]**:

1. **Permessi.** `atomic_write` crea il file temporaneo con i permessi di default (umask) e poi `rename`: il file finale perde il modo originale (`.zshrc` a `0600` diventerebbe `0644`), e i metadati estesi. Su Unix copiare il modo del file esistente sul temporaneo prima del `rename`.
2. **Symlink.** `read` rifiuta un percorso che non è un file regolare ("Il percorso non è un file regolare"). Su macOS è comune che `~/.zshrc` sia un link gestito da stow, chezmoi, yadm o un repository di dotfile. Oggi l'app fallirebbe; scrivere con `rename` sul link lo sostituirebbe con un file, rompendo la gestione dei dotfile. Proposta: risolvere il link (`canonicalize`) e gestire il **file di destinazione**, mostrando in anteprima entrambi i percorsi; il manifest registra il percorso risolto. Decisione da confermare (domanda D5). Se poi il gestore dei dotfile riscrive il file, il controllo d'hash esistente segnala correttamente il conflitto.
3. **Newline.** `update_profile_text` sceglie `\r\n` quando il file non contiene `\n` (file vuoto o nuovo): per gli rc POSIX deve essere sempre `\n`.
4. **Preferenze macOS (`defaults`).** Il modello attuale gestisce **file a byte**. Le chiavi di `com.googlecode.iterm2` e `com.apple.Terminal` non vanno scritte sul `.plist` (cfprefsd ha una cache e l'app proprietaria può riscriverle). Serve un nuovo tipo di modifica pianificata, accanto a `PlannedFile`: "chiave di preferenza" `{dominio, chiave, valore}` applicata con `/usr/bin/defaults` (o CFPreferences), con valore precedente registrato nel journal e nel manifest, controllo "il valore attuale è ancora quello che abbiamo scritto" prima del ripristino (stessa logica di `vanished`/hash). È l'equivalente della modifica mirata di `settings.json` (`shared_input`). Va progettato dopo S2/S3 e coperto da test con un dominio di prova (`defaults` su un dominio temporaneo `it.clisuperpower.test.*`).
5. **Case-insensitivity.** APFS di default non distingue maiuscole e minuscole: due percorsi diversi nel manifest (`BTreeMap<PathBuf, …>`) possono indicare lo stesso file. Rischio basso (tutti i percorsi vengono dalla scoperta), ma conviene normalizzare con `canonicalize` sui percorsi utente.

### 4.11 Keychain

Oggi nessuna funzione gestisce segreti: **il Keychain non serve per l'MVP**. Servirà per eventuali sorgenti team private con token o per l'assistente AI (F19). Raccomandazione per allora: crate `keyring` (Keychain su macOS, Secret Service su Linux, Credential Manager su Windows) **[condivisa con Linux]**. La chiave privata dell'updater resta solo nei segreti di GitHub, mai sulla macchina dell'utente.

### 4.12 Pacchetto, firma, notarizzazione, aggiornamenti

| Tema | Raccomandazione |
|---|---|
| Formato | `.app` dentro `.dmg` (`bundle.targets: ["app", "dmg"]` in `tauri.macos.conf.json`). Opzionale in futuro: cask in un tap Homebrew del progetto (equivalente della pubblicazione winget). |
| Architetture | Binario **universal** (`tauri build --target universal-apple-darwin`, rustup con `aarch64-apple-darwin` e `x86_64-apple-darwin`). Raddoppia tempi di build e peso, ma un solo download. |
| Versione minima | Proposta macOS **13 Ventura** (`bundle.macOS.minimumSystemVersion`): WebKit recente per Angular 22, zsh 5.9; da confermare (domanda D3). |
| Firma | Certificato **Developer ID Application** (Apple Developer Program, 99 USD/anno). Hardened runtime attivo (default di Tauri). Entitlement: `com.apple.security.automation.apple-events` per i workspace. |
| Notarizzazione | `notarytool` tramite Tauri (`APPLE_ID` + `APPLE_PASSWORD` app-specific + `APPLE_TEAM_ID`, oppure chiave API `APPLE_API_KEY`/`APPLE_API_ISSUER`), staple sul `.dmg`. Senza notarizzazione, da macOS 15 l'utente deve passare da Impostazioni → Privacy e sicurezza → "Apri comunque": accettabile per test interni, non per distribuzione. Molto più restrittivo dell'avviso SmartScreen di `PROGETTO.md` §9. |
| `csp` | Sidecar `bundle.externalBin` (`binaries/csp-<target-triple>`; per universal `csp-universal-apple-darwin` ottenuto con `lipo`). L'app lo copia (tramite applicatore e manifest) in `…/CliSuperpower/bin/csp` e gli script di avvio aggiungono quella cartella al `PATH`: niente scritture in `/usr/local/bin` (richiederebbe amministratore). Su Windows la distribuzione di `csp` non è ancora definita: conviene decidere lo stesso modello **[condivisa con Linux]**. |
| Updater | `tauri-plugin-updater`, `bundle.createUpdaterArtifacts: true` (artefatto `.app.tar.gz` + `.sig`), `latest.json` con `darwin-aarch64` e `darwin-x86_64` che puntano allo stesso archivio universal. Stessa chiave dell'updater delle altre piattaforme. |
| Posizione di installazione | L'updater sostituisce il bundle sul posto: se l'app è in `/Applications` e l'utente non è amministratore, l'aggiornamento fallisce. Documentare l'installazione in `~/Applications` per utenti standard e gestire l'errore con un messaggio chiaro. |
| Menu | Su macOS i comandi Copia/Incolla dei campi passano dal menu dell'app: verificare che la build abbia il menu "Modifica" predefinito, altrimenti Cmd+C/Cmd+V non funzionano nei campi dell'editor. |

### 4.13 Permessi, sandbox, privacy (TCC)

- **App Sandbox: no.** L'app deve scrivere `~/.zshrc`, file in `~/Library/Application Support/iTerm2`, preferenze di altre app e lanciare `brew`/`osascript`: incompatibile con la sandbox e quindi con il Mac App Store. Distribuzione fuori store.
- **Automation:** necessaria solo per i workspace (§4.6).
- **Accessibilità:** **non** richiesta, per scelta (niente System Events: Terminal.app senza tab nei workspace).
- **Cartelle protette** (Scrivania, Documenti, Download, iCloud Drive): qualsiasi `stat`/lettura dell'app su percorsi lì dentro fa comparire una richiesta di permesso. Oggi `Desktop::plan` legge l'immagine di sfondo dal percorso indicato (`apply::read(source)`): con le immagini caricate tramite `upload_background` il file è già nella cartella dell'app, ma un percorso scritto a mano in `~/Desktop` scatenerebbe la richiesta. Regola: non verificare l'esistenza di `workingDirectory`/`changeDirectory` dall'app (lo fa la shell quando esegue), e copiare sempre le immagini nella cartella dell'app.
- **Full Disk Access:** non serve.

---

## 5. Funzionalità senza equivalente diretto

| Funzione Windows | Problema su macOS | Soluzione proposta |
|---|---|---|
| Profilo `cmd` e tutto Clink (`clink.rs`, `generators/clink.rs`, AutoRun) | cmd non esiste | Nascosto su macOS; elementi team con `shells: ["cmd"]` ignorati in generazione con nota in anteprima. Le capacità di Clink sono coperte da zsh/fish nativamente. |
| Profilo `powershell` (Windows PowerShell 5.1) | Non esiste | Nascosto; elementi con solo `powershell` ignorati con nota. |
| Fragment WT con `updates` di profili esistenti | iTerm2 e Terminal.app non permettono di "sovrapporre" un profilo dell'utente | Profili nuovi "CLI Superpower · <shell>" che ereditano dal predefinito + proposta di renderli predefiniti (§4.1). |
| `useAcrylic` | Nessun acrilico | Sfocatura (`Blur`) in iTerm2; chiave di sfocatura in Terminal.app da verificare (S3). Etichetta UI "Sfocatura". |
| `backgroundImageAlignment` a 9 posizioni | iTerm2 ha solo modalità di riempimento | Mappatura con avviso; capacità esposta alla UI. |
| `cursorShape` `vintage`, `doubleUnderscore`, `emptyBox` | Non esistono | Approssimazione con avviso (§4.1). |
| `padding` a 4 lati | 2 valori in iTerm2, nessuno in Terminal.app | Massimo per asse, avviso. |
| Immagine di sfondo in Terminal.app | Non supportata | Capacità `backgroundImage: false` per Terminal.app; l'editor la disabilita con spiegazione. |
| Split nei workspace con Terminal.app | Non supportati | Avviso in anteprima: "con Terminal i pannelli si aprono come finestre separate"; suggerimento di usare iTerm2. |
| `predictionView: list` | Nessun equivalente stabile in zsh/fish/bash | Capacità per shell; opzione nascosta o disabilitata con spiegazione. |
| Help integrato `Get-Help mig` | Nessun meccanismo nativo di help per funzioni in zsh/bash | Messaggio d'uso nella funzione + `csp help`/`csp list` (F10). |
| Avviso `ExecutionPolicy` (`Restricted`/`AllSigned`) | Non esiste | Sostituito da avvisi utili su macOS: shell di login dell'utente diversa da quella configurata (`dscl . -read /Users/$USER UserShell` o `$SHELL`), conflitti con p10k/starship/zsh-autosuggestions già caricato. |
| Parametri nei comandi CMD (vietati oggi) | — | Su zsh/bash/fish i parametri sono supportati: nessuna limitazione analoga. |
| Quake mode (F12, v2) | Terminal.app non lo ha | iTerm2 *Hotkey Window* configurabile nel profilo dinamico (chiavi da verificare): v2. |
| Notifiche (F14, v2) | API diverse | `csp notify` con `UNUserNotificationCenter` (richiede app firmata) o `osascript -e 'display notification …'` come fallback: v2. |

---

## 6. Impatto sullo schema di configurazione (v6 → v7) **[condivisa con Linux]**

Un solo salto, coordinato con il porting Linux, per non avere due "v7" divergenti.

| Modifica | Dove | Note |
|---|---|---|
| Nuovi valori di profilo/shell: `zsh`, `bash`, `fish` | `validate.rs::PROFILES` (usato da `appearance`, `shells`, `pane.profile`, `shellBehavior`, `globalTerminal.defaultProfile`), `models.ts` `Profile`/`Shell` | Il validatore accetta **tutti** i valori su ogni OS: un pacchetto team può contenere comandi per Windows e per macOS. È il planner di ogni OS che genera solo per le shell disponibili e lo dice in anteprima. |
| Percorsi assoluti POSIX | `validate.rs::windows_path` → `absolute_path` (Windows **o** POSIX `/…`, senza `~` né variabili, come oggi) | Il planner rifiuta in anteprima un percorso della famiglia sbagliata ("percorso Windows su macOS: il comando X non verrà generato"). Aggiornare `config-validation.ts` (`absolutePath`, `pathHelp`). |
| Scelta dei terminali | Nuovo campo facoltativo `globalTerminal.terminals`: array di ID tra `windows-terminal`, `iterm2`, `apple-terminal` (+ ID Linux) | Omesso = tutti i terminali supportati e rilevati sull'OS corrente. È una preferenza della macchina: solo nel file personale (come `globalTerminal` oggi). |
| Nuovi tool | `config.rs::TOOLS` | `zsh-autosuggestions`, `fish`, `bash`, eventualmente `ble-sh`, `atuin` (coordinare con Linux). |
| Programma nei template | Nessuna modifica di schema | Ma l'UI oggi scrive `npm.cmd` (`app.ts` r.440, r.490): su macOS deve scrivere `npm`. Domanda D7: usare `npm` ovunque (su Windows `invoke-native.ps1` lo risolve comunque come Application). |

Migrazione 6 → 7: solo incremento di `schemaVersion` (ogni documento v6 è valido in v7), come i salti precedenti in `config.rs::migrate`. Da aggiornare insieme: `SCHEMA_VERSION`, `validate.rs::version_field` (oggi ha `6` **cablato**: conviene usare `SCHEMA_VERSION`), i test che confrontano `"schemaVersion":6` (`desktop.rs` test, `config/tests.rs`), `models.ts` (`schemaVersion: 1|…|6`), il mock di `config-bridge.ts::previewTeam` (`[1..6]`), `docs/config-schema.md`.

---

## 7. Architettura cross-platform proposta

### 7.1 Principio

Separare in ogni area tre cose che oggi in `desktop.rs` sono mescolate:

1. **Scoperta** (I/O, per OS): cosa c'è sulla macchina → una struct di *fatti* (percorsi, versioni, GUID, capacità). Esempi esistenti: `TerminalSettings`, `ShellProfile`.
2. **Generazione** (pura, compilata su **tutti** gli OS): `(config effettiva, fatti) → contenuto`. Nessun `cfg` nei generatori, così i loro snapshot test girano anche sulla CI Windows.
3. **Pianificazione** (pura): fatti + contenuti → `Vec<PlannedChange>` + anteprime. **Applicazione**: solo `apply.rs`.

### 7.2 Moduli e astrazioni

```
crates/core/src/
├─ platform.rs            PlatformPaths { home, data, state, generated }, enum Os, rilevamento PATH (GUI)
├─ shells/
│  ├─ mod.rs              trait ShellIntegration; struct ShellFacts; rc_block(text, block, newline)
│  ├─ powershell.rs       ex desktop.rs::{shell_profile, profile_block, update_profile_text, OH_MY_POSH_INIT}
│  ├─ zsh.rs, bash.rs, fish.rs        [condivisa con Linux]
├─ terminals/
│  ├─ mod.rs              trait TerminalIntegration; struct TerminalFacts; Capabilities
│  ├─ windows_terminal.rs ex desktop.rs::{TerminalSettings, profile_list, profile_shell, ramo fragment/defaultProfile di plan}
│  ├─ iterm2.rs           (macOS)
│  └─ apple_terminal.rs   (macOS)
├─ packages/
│  ├─ mod.rs              trait PackageManager; catalogo con ID per gestore   [condivisa con Linux]
│  ├─ winget.rs           ex tools.rs::platform (windows)
│  └─ brew.rs             (macOS; Homebrew esiste anche su Linux)
├─ color_schemes.rs       catalogo schemi (nome → colori), dati puri         [condivisa con Linux]
├─ generators/            puri: esistenti + zsh.rs, bash.rs, fish.rs, iterm2.rs, apple_terminal.rs, applescript.rs
├─ apply.rs               + PlannedChange::Preference (macOS), permessi/symlink POSIX
└─ desktop.rs             orchestratore: itera su shell e terminali disponibili
```

Firme indicative (il `Value` resta il formato della configurazione effettiva, come oggi):

```rust
pub trait ShellIntegration {
    fn id(&self) -> &'static str;                         // "zsh", "pwsh", ...
    fn discover(&self, paths: &PlatformPaths) -> Result<Option<ShellFacts>, String>;   // I/O
    fn plan(&self, effective: &Value, facts: &ShellFacts, paths: &PlatformPaths)
        -> Result<(Vec<PlannedChange>, Vec<PreviewFile>), String>;                    // puro
}

pub trait TerminalIntegration {
    fn id(&self) -> &'static str;                         // "windows-terminal", "iterm2", ...
    fn discover(&self, paths: &PlatformPaths) -> Result<Option<TerminalFacts>, String>;
    fn capabilities(&self) -> Capabilities;               // per la UI
    fn plan(&self, effective: &Value, facts: &TerminalFacts) -> Result<(Vec<PlannedChange>, Vec<PreviewFile>), String>;
    fn launch_workspace(&self, effective: &Value, id: &str, facts: &TerminalFacts) -> Result<(), String>;
}

pub trait PackageManager {
    fn id(&self) -> &'static str;                         // "winget", "brew"
    fn locate(&self, path: &SearchPath) -> Option<PathBuf>;
    fn install(&self, exe: &Path, package: &PackageRef) -> Result<(), String>;
    fn uninstall(&self, exe: &Path, package: &PackageRef) -> Result<(), String>;
}
```

Quali implementazioni registrare si decide in un solo punto (`desktop.rs` o `platform.rs`) con `cfg(target_os)`; tutto il resto è codice comune.

### 7.3 Cosa resta comune, cosa va dietro `cfg`

| Comune (tutti gli OS) | `#[cfg(unix)]` (macOS + Linux) | `#[cfg(target_os = "macos")]` | `#[cfg(windows)]` |
|---|---|---|---|
| `config` (parse, validate, migrate, resolve); tutti i **generatori** (compresi zsh/bash/fish/iTerm2/AppleScript); `apply.rs` (logica); `PlatformPaths` (struct); catalogo tool e schemi; `csp` parsing e flusso interattivo | Permessi e symlink in `atomic_write`; scoperta zsh/bash/fish; ricostruzione PATH; `kill` in `csp` | Scoperta iTerm2/Terminal.app; `defaults`/CFPreferences; `osascript`; `libproc` in `csp port`; Homebrew se si decide di non offrirlo su Linux | Scoperta WT; Clink e registro; winget; `windows-sys` in `csp port`; `CREATE_NO_WINDOW` |

### 7.4 Comandi Tauri

Il contratto IPC (`app/src-tauri/src/lib.rs`) resta invariato per non toccare la UX. Si aggiunge un solo comando:

- `platform_info` → `{ os, arch, shells: [{id, label, installed, isLoginShell}], terminals: [{id, label, installed, capabilities}], pathExample }`.

`terminal_profiles` resta e su macOS restituisce le shell utilizzabili (quelle installate e gestite da almeno un terminale rilevato).

---

## 8. Impatto sulla UI Angular (stessa UX, opzioni per OS)

La struttura (Home con le shell, editor Aspetto/Comandi/Workspace/Prompt/Autocomplete/Team, Tool, Cronologia, barra Applica) **non cambia**. Cambia ciò che è guidato dalla piattaforma, che oggi è cablato.

| Elemento | Oggi | Su macOS | Dove |
|---|---|---|---|
| Elenco shell | Costante `profiles` (pwsh, powershell, cmd) | Da `platform_info`: zsh, bash, fish, pwsh (solo installate o tutte con badge "non installata") | `app.ts` r.19, `models.ts` |
| Etichette e icone | "PowerShell 7", "Prompt dei comandi", icona PS/cmd | "Zsh", "Bash", "Fish", "PowerShell 7"; icona generica terminale | `app.html` r.47–68 |
| Anteprima del terminale | `PS C:\Progetti>`, "Microsoft Windows", "Copyright (C) Microsoft Corporation." | `nome@mac ~/Progetti %` (zsh), `$` (bash), `>` (fish); intestazione senza riferimenti Windows | `app.html` r.48, r.167 |
| Stato del profilo | "Pronto in Windows Terminal" / "Non rilevato in Windows Terminal" | "Pronto in iTerm2" / "Pronto in Terminale" / "Shell non installata" | `app.html` r.53 |
| Aspetto | "Usa uno schema di Windows Terminal", "Effetto acrilico", allineamento a 9 posizioni, 6 cursori, 4 margini | Opzioni filtrate dalle **capacità** del terminale scelto; "Sfocatura"; campi non supportati disabilitati con spiegazione | `app.html` r.75, r.128 |
| Font | Cascadia, Consolas, Lucida Console… | SF Mono, Menlo, Monaco + Nerd Font rilevati | `app.ts` r.352 |
| Esempi di percorso | `~\..\Desktop`, `C:\Users\nome\…` | `~/…/Desktop`, `/Users/nome/…` | `app.ts` `pathStyles`, `segmentOptions` |
| Validazione percorsi | Regex Windows, messaggio "percorso Windows assoluto" | Regex per OS da `platform_info.pathExample` | `config-validation.ts` |
| Comandi: scelta shell | Checkbox PS 7 / PS / CMD | Checkbox per le shell della piattaforma | `app.html` r.205–215 |
| Framework predefiniti | `program: 'npm.cmd'` | `program: 'npm'` | `app.ts` r.440, r.490 |
| Autocomplete | Note su PSReadLine e Clink, vista a elenco sempre disponibile | Note su zsh-autosuggestions/fish/ble.sh; "vista a elenco" disabilitata dove non c'è | `app.ts` `behaviorShells` r.267 |
| Banner | "nel Prompt dei comandi compare se Clink è già attivo" | Testo senza cmd/Clink | `app.html` r.156 |
| Prompt | "in PowerShell e nel Prompt dei comandi" | "in tutte le shell configurate" | `app.html` r.261–262 |
| Workspace | Avvio con Windows Terminal | Avvio con iTerm2 (o Terminale, con avviso sugli split); primo avvio: spiegazione del permesso Automazione | `app.html` sezione workspace |
| Tipografia | `"Segoe UI Variable","Segoe UI"`, `Consolas` (`styles.css`) | Pila di fallback: `-apple-system, BlinkMacSystemFont, "SF Pro Text"` e `ui-monospace, "SF Mono", Menlo`. **`DESIGN.md` è normativo** e cita Segoe UI Variable / Cascadia Mono: va aggiornato con la pila per macOS (decisione D12). | `styles.css`, `DESIGN.md` |
| Mock nel browser | Percorsi `C:\CLI-Superpower\`, profili Windows | Mock per piattaforma selezionabile (es. query `?os=macos`) per sviluppare la UI macOS anche da Windows | `config-bridge.ts` |
| Link esterni | `open_link` → `rundll32` | `tauri-plugin-opener` | `lib.rs` |

---

## 9. Strategia di test

| Livello | Cosa | Dove gira |
|---|---|---|
| Snapshot dei generatori | Ogni nuovo generatore (zsh, bash, fish, iTerm2, Terminal.app, AppleScript, blocco rc con LF) con snapshot **insta** (dev-dependency da aggiungere, come previsto in `PROGETTO.md` §7) e casi limite: apici, spazi, Unicode, `{{ }}`, parametri facoltativi, interi | Tutte le CI (generatori senza `cfg`) |
| Unità di dominio | Mappatura `appearance` → iTerm2 con approssimazioni; risoluzione file di login bash; validazione percorsi POSIX; migrazione 6→7 | Tutte le CI |
| Sintassi degli script generati | `zsh -n`, `bash -n` (anche con `/bin/bash` 3.2), `fish --no-execute`, `osacompile -o /dev/null` per l'AppleScript, `python3 -m json.tool` / parsing per i profili dinamici | `macos-latest` |
| Comportamento delle shell | `ZDOTDIR=$(mktemp -d) zsh -i -c '<comando rapido>'` con rc che carica lo script generato; idem bash con `--rcfile`, fish con `XDG_CONFIG_HOME` temporanea; asserzioni su output, codice di uscita, cartella corrente, cronologia non scritta per righe sensibili | `macos-latest` |
| Integrazione applicatore | `Desktop` con `PlatformPaths` interamente sotto una cartella temporanea (HOME finta): apply, idempotenza (secondo apply = nessuna modifica), conflitto dopo modifica manuale, remove che riporta i file byte per byte, permessi `0600` preservati, symlink | `macos-latest` (e `ubuntu-latest` per la parte POSIX) |
| Preferenze | `PlannedChange::Preference` su un dominio di test temporaneo, ripristino del valore precedente, conflitto se il valore cambia | `macos-latest` |
| `csp port` | Come `crates/csp/src/tests.rs` (`mod windows`): `TcpListener` temporaneo IPv4/IPv6, PID corrente, `verify_listener` | `macos-latest` |
| UI | Vitest con bridge simulato per `os = macos` (elenco shell, testi, validazione percorsi, capacità) | Tutte le CI |
| Build | `tauri build --no-bundle` su `macos-latest`; nella pipeline di release `--target universal-apple-darwin`, firma e notarizzazione | `macos-latest` |
| Smoke manuale | Checklist su un Mac pulito (Apple Silicon + uno Intel o VM): iTerm2 e Terminal.app, zsh/bash/fish, primo avvio Gatekeeper, permesso Automazione, aggiornamento | Manuale prima di ogni release |

CI: aggiungere il job `macos` a `.github/workflows/ci.yml` con gli stessi passi del job `windows` meno `fetch-clink` **[condivisa con Linux: matrice unica]**.

---

## 10. Rischi

| Rischio | Impatto | Probabilità | Mitigazione |
|---|---|---|---|
| Terminal.app riscrive le proprie preferenze mentre è aperto e cancella i profili gestiti; formato `NSKeyedArchiver` per colori e font | Aspetto perso o corrotto | Alta | Spike S3 prima di ogni sviluppo; applicare solo con Terminal chiuso o tramite import di file `.terminal`; se lo spike fallisce, Terminal.app resta "solo shell" (nessun aspetto) e l'UI lo dice. |
| Scrittura diretta di file `.plist` invece di `defaults`/CFPreferences | Modifiche ignorate o perse per la cache di cfprefsd | Media | Regola in `CLAUDE-macos.md`; test su dominio temporaneo. |
| Dotfile gestiti come symlink (stow/chezmoi/yadm) | Errore "non è un file regolare" o link rotto | Alta tra gli sviluppatori | §4.10 punto 2; domanda D5. |
| Permessi del file rc persi alla riscrittura | Regressione di sicurezza (rc leggibile da altri) | Certa senza correzione | §4.10 punto 1, test con `0600`. |
| Shell di login: bash legge `.bash_profile`, non `.bashrc` | Integrazione bash che "non funziona" | Alta | Regola di §4.2 con test. |
| PATH minimo dell'app GUI | Tool installati risultano mancanti; `brew` non trovato | Certa senza correzione | S5; PATH ricostruito e passato ai processi figli. |
| Conflitti con oh-my-zsh, prezto, powerlevel10k, plugin manager | Prompt doppio, avvisi di instant prompt, suggerimenti doppi | Alta | Blocco in coda; rilevamento e avvisi in anteprima; banner al primo `precmd`; guardie "già caricato". |
| Prefisso `/` non supportato da zsh o fish | Comandi `/ndev` impossibili in quelle shell | Media | S1; il core lo segnala; fallback `x-`/nessun prefisso già previsti dallo schema. |
| Permesso Automation negato | Workspace non avviabili | Media | Messaggio con percorso nelle Impostazioni; anteprima dello script comunque disponibile. |
| Costo e gestione di firma/notarizzazione (account, certificato, segreti CI) | Distribuzione bloccata da Gatekeeper | Certa senza account | Decisione D2 prima della fase di packaging; build interne non notarizzate solo per test. |
| Updater fallisce se l'app è in `/Applications` e l'utente non è amministratore | Aggiornamenti bloccati | Media in aziende | Documentare `~/Applications`; errore chiaro. |
| Homebrew assente, in prefisso non standard, o x86 sotto Rosetta su Apple Silicon | Installazione tool impossibile o nel prefisso sbagliato | Media | Rilevare `brew` reale e la sua architettura (`brew config`); link manuali come oggi. |
| Cask che chiedono la password (es. `powershell`) | Installazione bloccata in modalità non interattiva | Certa | Solo link manuale per quei tool. |
| bash 3.2 di sistema | Funzioni bash 4+ non disponibili; ble.sh lento | Alta | Generatore bash compatibile 3.2 (niente array associativi, niente `${var,,}`), test con `/bin/bash`. |
| Due schemi "v7" divergenti tra porting macOS e Linux | Documenti incompatibili | Media | Un solo task "schema v7 cross-platform" condiviso (PIANO M3). |
| Build universal lenta, sidecar `csp` con nome per target | CI lunga, bundle che non trova il sidecar | Media | Cache Rust per target; script `lipo` testato in S10. |
| Codice generato da AI non rivisto che tocca file di sistema | Bug sui file utente | Media | Generatori puri + snapshot + test d'integrazione con HOME finta (nessun test tocca la HOME reale: lezione già presente nel test `preview_apply_and_remove_integrate_without_touching_live_terminal`). |

---

## 11. Domande aperte per l'utente

| # | Domanda | Raccomandazione |
|---|---|---|
| D1 | Terminali nell'MVP: iTerm2 completo + Terminal.app ridotto va bene? Oppure serve anche WezTerm/Ghostty/Kitty subito (utili anche per Linux)? | iTerm2 + Terminal.app; gli altri dopo, come adapter condivisi con Linux. |
| D2 | C'è un account Apple Developer Program (99 USD/anno) per firma e notarizzazione? Chi custodisce certificato e chiavi API? | Sì, prima della fase di packaging. |
| D3 | Versione minima di macOS? | macOS 13 Ventura. |
| D4 | Livello di supporto bash: bash 3.2 di sistema (senza suggerimenti in linea) o richiedere bash 5 + ble.sh? | 3.2 supportata senza predizioni; ble.sh opzionale. |
| D5 | Dotfile symlink: seguire il link e modificare il file reale, o rifiutare con messaggio? | Seguire il link, mostrando il percorso reale in anteprima. |
| D6 | Un pacchetto team può contenere elementi per Windows e per macOS insieme (schema v7 cross-platform)? | Sì: validazione comune, generazione per OS. |
| D7 | `npm.cmd` nei template: passare a `npm` anche su Windows? | Sì, se un test su Windows conferma che `invoke-native.ps1` e Clink lo risolvono correttamente. |
| D8 | Homebrew assente: solo link a brew.sh o anche una procedura guidata? | Solo link. |
| D9 | fish e pwsh nell'MVP macOS o in un secondo momento? | fish sì (costo basso grazie a `conf.d`); pwsh sì (riusa il generatore). |
| D10 | Gli script generati: cartella dell'app (con spazio nel percorso) o `~/.config/cli-superpower` come su Linux? | Cartella dell'app, da allineare con la sessione Linux. |
| D11 | Distribuire anche un cask Homebrew (tap del progetto)? | In seguito, come la pubblicazione winget. |
| D12 | Approvare l'aggiornamento di `DESIGN.md` con la pila tipografica macOS (SF Pro / SF Mono)? | Sì: stessa identità, font di sistema della piattaforma. |
| D13 | Come si distribuisce `csp` (oggi non incluso in nessun pacchetto)? Copia gestita in `…/CliSuperpower/bin` + PATH dagli script di avvio, su tutte le piattaforme? | Sì, stesso modello ovunque. |
