# Porting di CLI Superpower su Linux — Analisi tecnica

> **Stato:** bozza v0.1 · **Data:** 6 novembre 2026 · **Ambito:** stessa UX e stesse funzionalità della versione Windows, su Linux desktop (x86_64, poi aarch64).
> **Fonti lette:** `PROGETTO.md`, `PRODUCT.md`, `DESIGN.md`, `CLAUDE.md`, `README.md`, `docs/config-schema.md`, `docs/spikes/fase-0.md`, `crates/core/src/**`, `crates/csp/src/**`, `app/src-tauri/**`, `app/src/app/**`, `.github/workflows/ci.yml`.
> **Documenti collegati:** [`PIANO.md`](PIANO.md) (piano eseguibile), [`CLAUDE-linux.md`](CLAUDE-linux.md) (regole aggiuntive). L'analisi macOS è in `docs/porting/macos/` (sessione parallela): i punti marcati **«condiviso con macOS»** vanno decisi una volta per entrambe.

---

## 0. Sintesi e decisioni raccomandate

| # | Decisione | Raccomandazione | Motivo in una riga |
|---|---|---|---|
| D1 | Terminali nell'MVP Linux | **Konsole** e **famiglia GNOME** (GNOME Terminal + Ptyxis, un solo backend dconf). Kitty/WezTerm nella fase successiva, insieme a macOS. Ghostty e Alacritty solo aspetto, più avanti. | Sono i terminali già installati su KDE e GNOME: stesso posizionamento di Windows Terminal («non è un nuovo terminale»). Hanno profili per shell, come WT. |
| D2 | Shell | **bash, zsh, fish** dalla prima release Linux; **pwsh** opzionale riusando `generators::powershell_init`. Niente `powershell` (5.1) né `cmd`. | Coprono quasi tutti gli utenti; pwsh esiste su Linux e il generatore attuale è quasi portabile. |
| D3 | Iniezione nei file utente | Blocco delimitato in `~/.bashrc` e `${ZDOTDIR:-$HOME}/.zshrc`; file **nostro** in `~/.config/fish/conf.d/cli-superpower.fish` (nessun file utente toccato per fish). Script generati in `~/.local/share/cli-superpower/generated/`. | Stesso schema di `$PROFILE` (`desktop.rs`, `profile_block`/`update_profile_text`). |
| D4 | Equivalente di Clink | Nessun Clink: **funzionalità native della shell** + plugin piccoli: zsh-autosuggestions (zsh), autosuggestion native (fish), **ble.sh opzionale** (bash), **fzf** per la ricerca nella cronologia (tutte). | Clink è specifico di cmd.exe; su Linux le shell hanno già hook equivalenti. |
| D5 | Motore del prompt | **Oh My Posh** anche su Linux (stesso `generators::oh_my_posh_config`). | Nessun cambio di modello; supporta bash/zsh/fish/pwsh. |
| D6 | Installazione tool | **Senza root**: binari di release ufficiali, versioni e SHA-256 fissati nel catalogo, in `~/.local/bin` (o `~/.local/share/cli-superpower/tools/`). Fallback: Homebrew se presente (condiviso con macOS), poi comando `apt`/`dnf`/`pacman`/`zypper` **mostrato** all'utente o eseguito via `pkexec` solo su conferma esplicita. | Equivalente di `winget --scope user`; nessuna elevazione automatica. |
| D7 | Configurazione di dconf (GNOME) | Trattare le chiavi dconf come **file virtuali** nell'applicatore (lettura = `dconf dump`/`dconf read`, scrittura = `dconf load`/`dconf write`/`dconf reset`), con lo stesso journal, manifest e controllo hash. | Riusa `apply.rs` invece di introdurre un secondo meccanismo di rollback. |
| D8 | Packaging | **AppImage** (installazione per utente, unico formato aggiornabile in modo affidabile dall'updater Tauri) + **.deb** e **.rpm**. **Niente Flatpak** in v1. | Flatpak richiederebbe `--filesystem=home` e `flatpak-spawn --host`: la sandbox andrebbe di fatto disattivata. |
| D9 | Schema configurazione | `schemaVersion` **7**, **un solo salto condiviso con macOS**: nuovi valori `bash`, `zsh`, `fish` per shell/profili, percorsi assoluti POSIX accettati, `globalTerminal.terminals` facoltativo (array di ID terminale). Migrazione 6→7 = solo incremento (i documenti v6 restano validi). | Regola di `CLAUDE.md`; continuità con le migrazioni 1→6 di `config::migrate`. |
| D10 | Architettura | Estrarre da `Desktop::plan` le integrazioni con i **nomi già proposti nell'analisi macOS**: `platform.rs` (`Os`, `PlatformPaths`), `shells/` (`trait ShellIntegration`), `terminals/` (`trait TerminalIntegration`), `packages/` (`trait PackageManager`), `color_schemes.rs`; in più un `trait Probe` per rendere testabile la scoperta. Generatori sempre compilati su tutti gli OS (puri); solo scoperta e I/O dietro `cfg(target_os)`. | Oggi `desktop.rs` mescola WT, Clink e PowerShell in un'unica funzione `plan`. |
| D11 | `csp port` | Implementazione Linux su `/proc/net/tcp{,6}` + `/proc/<pid>/fd` + `pidfd` per la terminazione sicura. | Stessa UX (`termina <PID>`), stessa difesa dal riuso del PID. |
| D12 | CI | Nuovo job `ubuntu-22.04` (glibc più vecchia → binari compatibili) con build Tauri, test Rust/Angular, smoke test delle shell reali (bash/zsh/fish) e di dconf in `dbus-run-session`. | `PROGETTO.md` §8.3 prevede già test su Linux per i generatori. |

---

## 1. Inventario: cosa fa oggi il codice e dove dipende da Windows

L'inventario parte dal **codice reale**, non dalla roadmap. Stato: fase 0 avanzata, molte funzionalità v1 già presenti.

### 1.1 Backend (`crates/core`)

| Area | File / funzione | Cosa fa | Dipendenza da Windows | Portabilità |
|---|---|---|---|---|
| Config | `config.rs`: `SCHEMA_VERSION = 6`, `migrate`, `resolve`, `overlay`, `merge_items` | Strati default → team → personale, migrazione 1→6 | Nessuna nel motore | **Comune** |
| Validazione | `config/validate.rs`: `PROFILES = ["pwsh","powershell","cmd"]`, `windows_path`, `drive_path`, `unc_path`, `appearance` | Valida i documenti | `PROFILES` e `windows_path` (solo `C:\…` e UNC) | Da estendere (§9) |
| Parsing | `config/parse.rs`: `parse_json` | JSON stretto | Nessuna | Comune |
| Applicatore | `apply.rs`: `apply`, `commit`, `recover`, `rollback`, `history`, `restore_latest`, `removals`, `atomic_write`, `lock`, `read` | Journal `pending.json`, manifest con SHA-256, cronologia (10 voci), lock `apply.lock` | Nessuna diretta (usa `std::fs`, `File::lock`, `rename`) | Comune, con 3 aggiustamenti Linux (§4.4) |
| Orchestrazione | `desktop.rs`: `Desktop { data, local, system_local, clink_bundle }`, `plan`, `apply`, `remove`, `launch`, `profiles`, `shell_profile`, `profile_block`, `update_profile_text`, `TerminalSettings`, `profile_shell` | Pianifica e applica tutto | **Forte**: percorsi WT in `profiles()`, cartella `Fragments`, `wt.exe` in `launch`, `"{shell}.exe"` in `shell_profile`, `$env:LOCALAPPDATA` nel blocco di `update_profile_text`, CRLF di default, `OH_MY_POSH_INIT` con `oh-my-posh.exe`, `sync_clink` | Da scomporre (§8) |
| Clink | `clink.rs`: `profile_directory`, `enable`, `disable`, `autorun_values` (registro via `powershell.exe`), `injection` | Copia Clink e imposta AutoRun di cmd | **Solo Windows** | Nessun equivalente; non compilarlo fuori da Windows o lasciarlo inerte |
| Tool | `tools.rs`: `CATALOG` (12 tool con `winget_id`), `statuses`, `install`, `uninstall`, `install_recommended`, `mod platform` `#[cfg(windows)]` (winget, `FileVersionInfo`, font nel registro) e stub `#[cfg(not(windows))]` | Rilevamento e installazione | **Forte**: winget, `.exe`, registro dei font | Nuovo backend (§6) |
| Generatori | `generators.rs`: `terminal_fragment`, `terminal_images`, `workspace_args`, `wt_value` | Fragment WT e argomenti `wt` | Specifici di WT (ma puri) | Restano; se ne aggiungono per Linux |
| Generatori | `generators/powershell.rs`: `powershell_init` + `invoke-native.ps1` | Funzioni per i comandi rapidi, PSReadLine, filtro cronologia | Quasi nessuna (vedi §5.6) | Riutilizzabile per pwsh su Linux |
| Generatori | `generators/clink.rs`: `clink_script_with_theme` | Script Lua per cmd | Solo cmd | Non usato su Linux |
| Generatori | `generators/prompt.rs`: `oh_my_posh_config` | Config Oh My Posh | Lo stile `full` sostituisce `/` con `\` | Comune, con un ritocco (§5.7) |
| Generatori | `generators/banner.rs` (`banner_enabled`, `powershell`, `lua`), `banner.txt` | Banner di avvio | Nessuna | Comune; servono varianti sh/fish |
| Generatori | `generators/behavior.rs`: `SENSITIVE`, `behavior`, `has_behavior` | Modello autocomplete/cronologia | Nessuna | **Comune** |
| Sfondi | `desktop.rs`: `upload_background`, `background_preview`, `background_format` | Salva immagini per hash | Nessuna | Comune |

### 1.2 `csp` (`crates/csp`)

| File | Stato | Dipendenza |
|---|---|---|
| `main.rs`: `parse_port`, `run_port` (stub `#[cfg(not(windows))]` che restituisce errore) | Parsing comune | — |
| `port.rs` (`#[cfg(windows)]`): `listeners` (`GetExtendedTcpTable`), `Process::open/created_at/is_critical`, `terminate` (`TerminateProcess`), `confirmed_pid`, `process_age` | Solo Windows | `windows-sys` |
| `tests.rs`: `accepts_only_valid_tcp_port_numbers` (comune), modulo `windows` con listener reali | Misto | — |

### 1.3 Shell Tauri (`app/src-tauri`)

| Punto | Codice | Dipendenza |
|---|---|---|
| Cartelle | `lib.rs` `run()`: `data_dir()/CliSuperpower`, `local_data_dir()/CliSuperpower`, `system_local = local_data_dir()` | Su Linux `data_dir()` e `local_data_dir()` coincidono (`$XDG_DATA_HOME`): config, stato e «system_local» finirebbero nella stessa cartella |
| Clink incluso | `resource_dir()/clink`, `tauri.conf.json` `bundle.resources` e `beforeBuildCommand: npm run fetch-clink` (script **PowerShell** `app/scripts/fetch-clink.ps1`) | La build Linux fallisce o scarica un binario inutile |
| Link esterni | `open_link` usa `rundll32 url.dll,FileProtocolHandler` | Solo Windows |
| Icone | `bundle.icon = ["icons/terminal-spezzato.ico"]`, `bundle.active = false` | Servono PNG per Linux |
| Comandi IPC | 18 comandi (`load_config` … `terminal_profiles`, `open_link`) | Firma neutra; `terminal_profiles` restituisce ID WT |

### 1.4 UI Angular (`app/src/app`)

| Punto | Codice | Dipendenza |
|---|---|---|
| Tipi | `models.ts`: `Profile` limitato a `pwsh`, `powershell`, `cmd`, `PersonalConfig.schemaVersion: 1..6` (scritti a mano, non generati: **tauri-specta/ts-rs non sono ancora in uso**) | Da estendere |
| Profili | `app.ts:19` `profiles` fisso (PowerShell 7, Windows PowerShell, Prompt dei comandi); `app.ts:268-270` `behaviorShells` con note specifiche (PSReadLine, Clink) | Da rendere guidato dal backend |
| Temi | `app.ts:342` `appearanceThemes` (8 schemi WT con 4 colori per l'anteprima) | Su Linux servono le **palette complete** generate dal core |
| Font | `app.ts:352` `terminalFonts` fisso (Cascadia, Consolas, …) | Su Linux elenco da `fc-list` |
| Validazione | `config-validation.ts:9-10` `pathHelp` («percorso Windows assoluto») e regex `absolutePath` solo Windows | Da rendere dipendente dall'OS |
| Testi | `app.html` cita «Windows Terminal», «Windows PowerShell» (righe 47, 53, 75, 167-168, 206) | Da parametrizzare col nome del terminale |
| Mock | `config-bridge.ts` simula profili Windows e percorsi `C:\CLI-Superpower\…` | Serve una variante Linux per lo sviluppo nel browser |
| Programmi | `app.ts:440` e `app.ts:490` scrivono `npm.cmd` nei comandi creati dagli assistenti | Su Linux deve essere `npm` (stesso problema segnalato dall'analisi macOS) |
| Anteprima home | `app.html:47-48` disegna `Microsoft Windows`, `PS C:\Progetti>`, `C:\Progetti>` | Prompt Unix di esempio (`~/progetti $`) per bash/zsh/fish |

### 1.5 Build e CI

`.github/workflows/ci.yml` ha un solo job `windows-latest`. Non c'è ancora updater né bundle attivo (`bundle.active: false`), quindi packaging e auto-update sono da progettare per **entrambe** le piattaforme; su Linux non si «porta» niente di esistente.

> **Nota sui test.** `CLAUDE.md` chiede snapshot test per ogni generatore, ma oggi i test dei generatori sono asserzioni puntuali in `#[cfg(test)]` e `insta` non è tra le dipendenze. Per i nuovi generatori Linux il piano introduce `insta` come dev-dependency (vedi `PIANO.md`, task L1.1).

---

## 2. Terminali Linux

### 2.1 Il problema

Su Windows c'è un terminale di riferimento (Windows Terminal) con un'API di estensione documentata (fragment). Su Linux non esiste un terminale universale né un meccanismo comune: ogni terminale ha formato e capacità propri. Il modello «profilo per shell» di WT (`appearance.pwsh`, `appearance.cmd`…) esiste nativamente solo in alcuni.

### 2.2 Matrice delle capacità

Legenda: ✅ supportato · ⚠️ parziale o da verificare nello spike · ❌ assente.

| Capacità (campo config) | Konsole | GNOME Terminal | Ptyxis | Kitty | WezTerm | Ghostty | Alacritty |
|---|---|---|---|---|---|---|---|
| Formato config | file INI `~/.local/share/konsole/*.profile` + `*.colorscheme`; `~/.config/konsolerc` | dconf `/org/gnome/terminal/legacy/profiles:/` | GSettings `org.gnome.Ptyxis` (dconf) + palette in `~/.local/share/org.gnome.Ptyxis/palettes/` | `~/.config/kitty/kitty.conf` con `include` | Lua `~/.config/wezterm/wezterm.lua` | `~/.config/ghostty/config` con `config-file` | TOML con `general.import` |
| Inclusione non distruttiva | ✅ file nostri (profilo e schema) | ⚠️ profili nostri, ma la lista `list` è condivisa | ⚠️ come GNOME (`profile-uuids`) | ✅ una riga `include` in blocco marcato | ⚠️ blocco marcato con `require` | ✅ una riga `config-file` | ✅ una voce `import` |
| Profili per shell (= WT) | ✅ | ✅ | ✅ | ❌ (solo sessioni/launcher) | ⚠️ `launch_menu`, non profili veri | ❌ | ❌ |
| `colorScheme` | ✅ `.colorscheme` | ✅ `palette` + fg/bg | ✅ file `.palette` | ✅ | ✅ | ✅ | ✅ |
| `font.face/size` | ✅ per profilo | ✅ per profilo | ⚠️ solo globale (`font-name`) | ✅ globale | ✅ | ✅ | ✅ |
| `backgroundImage` (+ opacità) | ✅ `Wallpaper=` nello schema colori (allineamento ⚠️) | ❌ | ❌ | ✅ `background_image` | ✅ | ✅ (versioni recenti ⚠️) | ❌ |
| `opacity` | ✅ `Opacity=` (con compositor) | ✅ `background-transparency-percent` | ✅ | ✅ | ✅ | ✅ | ✅ |
| `useAcrylic` | ⚠️ `Blur=true` (solo KDE) | ❌ | ❌ | ⚠️ `background_blur` | ⚠️ | ⚠️ | ❌ |
| `cursorShape` | ✅ 3 forme | ✅ block/ibeam/underline | ⚠️ globale | ✅ | ✅ | ✅ | ✅ |
| `padding` | ⚠️ margine unico (`TerminalMargin`, da verificare) | ❌ | ❌ | ✅ | ✅ | ✅ | ✅ |
| `defaultProfile` | ✅ `konsolerc` `DefaultProfile=` | ✅ chiave `default` | ✅ `default-profile-uuid` | n/a | n/a | n/a | n/a |
| Workspace: tab | ✅ `--tabs-from-file` / D-Bus | ✅ `--tab` | ✅ `--tab` | ✅ file di sessione | ✅ `wezterm cli spawn` | ❌ da CLI su Linux | ❌ |
| Workspace: split | ⚠️ `--layout` (versioni recenti, da verificare) o D-Bus | ❌ → tab | ❌ → tab | ✅ `layout splits` | ✅ `wezterm cli split-pane` | ❌ | ❌ |
| Disponibile su macOS | ❌ | ❌ | ❌ | ✅ | ✅ | ✅ | ✅ |
| Preinstallato | KDE (Kubuntu, Fedora KDE, openSUSE, Kinoite) | Ubuntu ≤ 25.04, Debian GNOME | Fedora Workstation, Ubuntu ≥ 25.10 | no | no | no | no |

### 2.3 Raccomandazione per l'MVP

1. **Konsole** — il più vicino a Windows Terminal: profili per comando, schemi colori con sfondo e opacità, tab e split, configurazione **solo su file** (si integra senza modifiche nell'applicatore). Generiamo file nostri: `CliSuperpower-<shell>.profile` e `CliSuperpower-<shell>.colorscheme`. L'unica chiave condivisa è `DefaultProfile` in `konsolerc`, modificata solo se l'utente imposta `globalTerminal.defaultProfile`.
2. **GNOME Terminal + Ptyxis** (un backend «dconf», due adattatori) — il terminale di chi usa GNOME, cioè la maggioranza degli utenti Ubuntu/Fedora. Creiamo **profili nostri** con UUID stabili (derivati in modo deterministico dall'ID shell, per l'idempotenza) e aggiungiamo il loro UUID alla lista condivisa. Limiti mostrati nell'anteprima: niente immagine di sfondo, niente padding, split resi come tab; su Ptyxis font e cursore sono globali.
3. **Dopo l'MVP, insieme a macOS:** Kitty e/o WezTerm (stesso generatore su Linux e macOS, **condiviso con macOS**). Kitty ha la miglior resa dei workspace (file di sessione) ma non ha profili: l'aspetto «per shell» si ottiene con file `--config` aggiuntivi e launcher dedicati, oppure si applica un aspetto unico. Ghostty e Alacritty: solo aspetto globale, nessun workspace.

Motivo dell'ordine: KDE e GNOME coprono la quasi totalità dei desktop Linux; i loro terminali sono già installati (coerente con PROGETTO §1, «usa Windows Terminal, che esiste già»), hanno profili per shell (stessa UX dell'editor aspetto) e non richiedono di installare nulla. Kitty/WezTerm conviene farli una volta sola per Linux e macOS.

### 2.4 Rilevamento

- Terminali installati: ricerca nel `PATH` **e** nei percorsi noti (l'app GUI non eredita il `PATH` della shell interattiva): `/usr/bin`, `/usr/local/bin`, `~/.local/bin`, `~/.local/kitty.app/bin`, `/var/lib/flatpak/exports/bin`, `~/.local/share/flatpak/exports/bin`, `/snap/bin`.
- Terminale del desktop: `$XDG_CURRENT_DESKTOP` (KDE → Konsole, GNOME → Ptyxis se presente, altrimenti GNOME Terminal); `xdg-terminal-exec` se presente.
- **Terminali Flatpak** (es. Ptyxis da Flathub): leggono le impostazioni da `~/.var/app/<id>/config/glib-2.0/settings/keyfile`, non dal dconf dell'utente. In v1: rilevarli e mostrare «versione Flatpak non supportata», non scrivere.

### 2.5 dconf come «file virtuale»

L'applicatore (`apply.rs`) lavora su `PathBuf` e byte. Proposta: introdurre un piccolo strato `Store` usato da `read`/`replace`, che riconosce lo schema del percorso:

| Percorso virtuale | Lettura | Scrittura | Rimozione |
|---|---|---|---|
| `dconf-dir:/org/gnome/terminal/legacy/profiles:/:<uuid>/` | `dconf dump <dir>` (testo keyfile) | `dconf reset -f <dir>` + `dconf load <dir>` | `dconf reset -f <dir>` |
| `dconf-key:/org/gnome/terminal/legacy/profiles:/list` | `dconf read <key>` (testo GVariant) | `dconf write <key> <valore>` | `dconf reset <key>` |

Così journal, manifest con hash, conflitti («modificato esternamente») e ripristino funzionano senza cambiare la logica. La chiave `list` è **condivisa** (come `settings.json` su Windows): si usa `shared_input` di `PlannedFile` per la modifica mirata. Attenzione: se `list` non è mai stato scritto, il profilo predefinito esiste solo implicitamente; aggiungere il nostro UUID richiede di includere anche quello predefinito (da verificare nello spike S3).

---

## 3. Shell

### 3.1 Dove e come iniettare

| Shell | File utente | Cosa scriviamo | Rilevamento a runtime (equivalente di `shell_profile`) |
|---|---|---|---|
| bash | `~/.bashrc` | Blocco delimitato che carica `init.bash` | `bash --version`; percorso rc fisso `~/.bashrc` (le shell di GNOME Terminal/Konsole non sono di login) |
| zsh | `${ZDOTDIR:-$HOME}/.zshrc` | Blocco delimitato che carica `init.zsh` | `zsh -c 'print -r -- ${ZDOTDIR:-$HOME}'` (legge `.zshenv`, dove di solito si imposta `ZDOTDIR`) |
| fish | nessuno | **File nostro** `$__fish_config_dir/conf.d/cli-superpower.fish` | `fish -c 'echo $__fish_config_dir'` (rispetta `XDG_CONFIG_HOME`) |
| pwsh | `$PROFILE.CurrentUserCurrentHost` (di solito `~/.config/powershell/Microsoft.PowerShell_profile.ps1`) | Blocco esistente, con percorso Linux | `shell_profile` attuale con binario `pwsh` invece di `pwsh.exe` |

Blocco per bash/zsh (identico nello stile a PROGETTO §5.2):

```bash
# >>> CLI Superpower >>> (non modificare: gestito automaticamente)
if [ -r '/home/anna/.local/share/cli-superpower/generated/init.bash' ]; then
    . '/home/anna/.local/share/cli-superpower/generated/init.bash'
fi
# <<< CLI Superpower <<<
```

- Il percorso è **assoluto e quotato**, risolto dall'app al momento di Applica (su Windows si usa `$env:LOCALAPPDATA`; su Linux `XDG_DATA_HOME` può differire tra sessione grafica e shell, quindi il letterale è più affidabile).
- `update_profile_text` va generalizzata in una funzione pura `rc_block(text, block, comment, default_newline)` (**condivisa con macOS**). Su Linux il terminatore di default per un file vuoto deve essere `\n` (oggi è `\r\n`).
- Il file `init.*` controlla di essere in una shell interattiva (`[[ $- == *i* ]] || return` in bash; `[[ -o interactive ]] || return` in zsh; `status is-interactive; or exit` in fish), perché `.zshrc`/conf.d possono essere letti anche da script.
- **Ordine:** il blocco si accoda alla fine del file (come su Windows). Se l'utente usa oh-my-zsh, Powerlevel10k o Starship e attiva il nostro prompt, l'anteprima avvisa del conflitto (equivalente dei messaggi di `conflicts` in `TerminalSettings`).
- Se `~/.bashrc` manca (raro, alcune distro minimali) lo si crea: il manifest registra `original: None`, come per un `$PROFILE` assente.

### 3.2 Comandi rapidi (F2)

Ogni comando diventa una **funzione** nel file `init` della shell, con gli argomenti passati come dati (mai `eval` dei valori), in linea con `docs/config-schema.md` («il generatore passa ciascun valore come dato»):

```bash
mig() {
    case "${1-}" in -h|--help) printf '%s\n' 'Crea una migrazione EF' 'Uso: mig <nome>'; return 0;; esac
    [ "$#" -ge 1 ] || { printf 'Parametro obbligatorio: nome\n' >&2; return 2; }
    command dotnet ef migrations add "$1"
}
```

- Parametri opzionali senza default: l'argomento si salta (array `set --` costruito in modo incrementale, compatibile bash 3.2 per poterlo **condividere con macOS**).
- `workingDirectory`: subshell `( cd -- "$dir" && exec … )`, che non cambia la cartella dell'utente anche in caso di errore.
- `changeDirectory`: `cd -- '/percorso'` dentro la funzione (cambia la shell corrente, come `Set-Location`).
- Help: `--help` generato da descrizione e parametri (equivalente del comment-based help di `append_help`).
- fish: `function mig --description '…'` con `argparse h/help`, `$argv[1]`.
- Collisioni: il rilevamento chiede alla shell `type -t nome` (bash), `whence -w nome` (zsh), `type -q nome` (fish), come `shell_profile` fa con `Get-Command`.

**Prefisso `/`.** In bash un nome con `/` viene trattato come percorso e non cerca le funzioni; fish non ammette `/` nei nomi di funzione. Quindi `/ndev` **non funziona nativamente** su bash e fish (da confermare nello spike S1). Soluzioni:

| Shell | Soluzione per `/nome` | Note |
|---|---|---|
| zsh | widget ZLE su `accept-line` che riscrive `/nome args` in `__csp_nome args` | Analogo a `clink.onfilterinput` dello script Clink |
| fish | `abbr --regex '^/nome$' --function …` (fish ≥ 3.6) | Espansione visibile alla pressione di spazio/Invio |
| bash | nessuna soluzione robusta senza ble.sh | `commandPrefix: "/"` + scope bash → errore di validazione con suggerimento `x-` o nessun prefisso |

`x-` e prefisso vuoto funzionano in tutte e tre le shell.

### 3.3 Autocomplete e cronologia (F9, `shellBehavior`)

| Opzione | bash | zsh | fish |
|---|---|---|---|
| `predictions: history` (in linea) | **ble.sh** (opzionale, pesante) | **zsh-autosuggestions** (`ZSH_AUTOSUGGEST_STRATEGY=(history)`) | nativa (`fish_autosuggestion_enabled 1`, fish ≥ 3.4) |
| `predictions: off` | non caricare ble.sh | non caricare il plugin | `set -g fish_autosuggestion_enabled 0` |
| `predictionView: list` | fzf su Ctrl+R (`eval "$(fzf --bash)"`, fzf ≥ 0.48) | fzf (`fzf --zsh`) | fzf (`fzf --fish`) |
| `acceptKey: tab` | binding ble.sh | `bindkey '^I'` su un widget che accetta il suggerimento o richiama il completamento precedente | `bind \t` con `commandline --showing-suggestion` (versione da verificare) |
| `hideSensitive` / `historyExclude` | `HISTIGNORE` con glob resi insensibili alle maiuscole (`*[pP][aA][sS][sS]*`) | funzione `zshaddhistory` che restituisce 2 (resta in memoria, non va su file: **identico a `MemoryOnly`** di PSReadLine) | `fish_should_add_to_history` (versione da verificare) |

Differenza da dichiarare in UI: in bash `HISTIGNORE` esclude la riga anche dalla cronologia **in memoria**, mentre PowerShell e zsh la tengono in memoria. Il modello dei pattern resta quello comune di `generators/behavior.rs` (`SENSITIVE`, `behavior`).

La UI mostra per ogni shell una nota come quelle di `behaviorShells` (`app.ts:268-270`): per bash «Le predizioni in linea richiedono ble.sh, installabile dall'app»; per fish «Le predizioni sono integrate nella shell».

### 3.4 Banner (`banner`)

Varianti sh/fish del frammento di `banner.rs`: solo shell interattive, `[ -t 1 ]` (output non reindirizzato), larghezza da `COLUMNS` o `tput cols` ≥ larghezza del disegno. Il testo resta `banner.txt` (comune).

### 3.5 Prompt (F5)

Oh My Posh su Linux: `eval "$(oh-my-posh init bash --config '<file>')"`, idem `zsh`, `oh-my-posh init fish --config … | source`. Il file `prompt.omp.json` è lo stesso prodotto da `oh_my_posh_config`. Unico ritocco: lo stile `full` (`prompt.rs`) sostituisce `/` con `\`; su Linux va lasciato il separatore nativo (template `{{ if eq .OS "windows" }}…{{ else }}{{ .Location }}{{ end }}`, oppure parametro di piattaforma al generatore: da scegliere nel task L1.6).

### 3.6 pwsh su Linux

Il generatore `powershell_init` è quasi portabile. Va adattato:

| Punto | Oggi | Linux |
|---|---|---|
| Binario in `shell_profile` | `format!("{shell}.exe")` | `pwsh` |
| Blocco in `$PROFILE` (`update_profile_text`) | `$env:LOCALAPPDATA\CliSuperpower\generated\…` | percorso assoluto `/home/…/.local/share/cli-superpower/generated/init-pwsh.ps1` |
| `OH_MY_POSH_INIT` | `Get-Command -Name oh-my-posh.exe` | `oh-my-posh` |
| Policy di esecuzione | `Get-ExecutionPolicy` (Restricted/AllSigned) | Su Linux è sempre `Unrestricted`: l'avviso non serve |
| `invoke-native.ps1` | ramo `.cmd/.bat` | innocuo su Linux |
| Shell `powershell` (5.1) | supportata | **non esiste**: nascosta dalla UI |
| Workspace | `pwsh.exe -NoExit -EncodedCommand` | `pwsh -NoExit -EncodedCommand` |

### 3.7 cmd e Clink

Nessun equivalente. Su Linux il profilo `cmd` non compare; comandi con `shells: ["cmd"]` in un pacchetto team vengono ignorati con avviso nell'anteprima. Le funzioni che su Windows richiedono Clink (scorciatoie, autosuggerimenti, banner, prompt in cmd) su Linux sono coperte dalle shell native (§3.2-3.5).

---

## 4. Percorsi, file e applicatore

### 4.1 Percorsi a runtime (`PlatformPaths`, **condiviso con macOS**)

| Ruolo | Windows (oggi) | Linux (proposto) | Come si risolve |
|---|---|---|---|
| Config personale e pacchetti team (`Desktop.data`) | `%APPDATA%\CliSuperpower\` | `$XDG_CONFIG_HOME/cli-superpower/` (default `~/.config/cli-superpower/`) | Tauri `app.path().config_dir()` |
| Generati, sfondi, tool, manifest, cronologia, lock (`Desktop.local`) | `%LOCALAPPDATA%\CliSuperpower\` | `$XDG_DATA_HOME/cli-superpower/` (default `~/.local/share/cli-superpower/`) | Tauri `app.path().data_dir()` |
| Base per integrazioni di sistema (`Desktop.system_local`) | `%LOCALAPPDATA%` (Fragments, Clink) | non serve: ogni terminale risolve i propri percorsi (`XDG_CONFIG_HOME`, `XDG_DATA_HOME`) | backend terminale |
| Binari dei tool | winget | `~/.local/bin` (link) + `~/.local/share/cli-superpower/tools/<id>/<versione>/` | `PlatformPaths::user_bin` |
| Font | registro | `$XDG_DATA_HOME/fonts/` + `fc-cache` | — |

Nota: oggi `lib.rs` usa `data_dir()` e `local_data_dir()`, che su Linux coincidono. La separazione config/dati va fatta esplicitamente. Nome della cartella in minuscolo con trattino (`cli-superpower`), convenzione XDG. `manifest.json`/`history.json` potrebbero stare in `$XDG_STATE_HOME`, ma tenerli accanto ai generati (come su Windows) evita di cambiare `apply.rs`: si raccomanda di non spostarli in v1.

### 4.2 Elenco dei file toccati su Linux

| File / chiave | Proprietario | Contenuto |
|---|---|---|
| `~/.config/cli-superpower/config.personal.json`, `team/*.json` | App | Come su Windows |
| `~/.local/share/cli-superpower/generated/init.bash`, `init.zsh`, `init-pwsh.ps1`, `prompt.omp.json` | App | Script generati |
| `~/.config/fish/conf.d/cli-superpower.fish` | App | Script fish generato |
| `~/.local/share/konsole/CliSuperpower-*.profile`, `CliSuperpower-*.colorscheme` | App | Profili e schemi Konsole |
| `dconf /org/gnome/terminal/legacy/profiles:/:<uuid>/` (e Ptyxis) | App | Profili GNOME nostri |
| `~/.bashrc`, `~/.zshrc`, `$PROFILE` di pwsh | **Utente** | Solo il blocco delimitato |
| `dconf …/profiles:/list`, `…/default`; `~/.config/konsolerc` `DefaultProfile` | **Utente** | Solo le chiavi gestite, con valore precedente nel manifest |
| `~/.local/bin/<tool>`, `~/.local/share/fonts/…` | App | Solo i tool installati dall'app (`installed-tools.json`) |

### 4.3 Riavvio dei terminali

Konsole rilegge i profili all'apertura di una nuova scheda/finestra; GNOME Terminal e Ptyxis applicano subito le modifiche dconf. Le shell vedono le modifiche solo in nuove sessioni: messaggio identico a quello di oggi («verrà caricato al prossimo avvio della shell»).

### 4.4 Aggiustamenti Linux all'applicatore (`apply.rs`)

1. **Dotfile come symlink.** `apply::read` rifiuta i file non regolari (`symlink_metadata` → «Il percorso non è un file regolare»). Su Linux è comune gestire i dotfile con GNU Stow, chezmoi o home-manager (symlink). Proposta (**condivisa con macOS**): per i soli file utente condivisi (`shared_input`), risolvere il symlink se il target è un file regolare scrivibile nella home e scrivere sul target, registrando nel manifest il percorso risolto. Se il target è in sola lettura (es. `/nix/store`), rifiutare con un messaggio e mostrare il blocco da incollare a mano.
2. **Permessi.** `atomic_write` crea un file nuovo e lo rinomina: il file finale prende i permessi di default (umask), non quelli dell'originale. Copiare la modalità del file sostituito (es. un `.zshrc` 0600 deve restare 0600).
3. **Durabilità.** Dopo `rename`, `fsync` della cartella padre (su ext4/btrfs la rinomina non è garantita su disco senza). Non cambia la semantica, migliora il recupero dopo un crash.

`File::lock` (flock) e `rename` atomico funzionano su Linux senza modifiche.

---

## 5. Strumenti esterni (F7)

### 5.1 Strategia di installazione

Ordine delle sorgenti per ogni tool (trait `PackageManager`):

1. **Già presente** → nessuna azione, non è «nostro» (stessa regola di `tools::install`).
2. **Release ufficiale** (binario statico, preferibilmente musl) scaricata via HTTPS, **versione e SHA-256 fissati nel catalogo** compilato nell'app, estratta in `~/.local/share/cli-superpower/tools/<id>/<versione>/` con link in `~/.local/bin`. Nessun root. Disinstallazione = rimozione dei nostri file (registrati in `installed-tools.json`).
3. **Homebrew** se `brew` è già installato (**condiviso con macOS**).
4. **Gestore di sistema** (`apt`, `dnf`, `pacman`, `zypper`): richiede root. Si mostra il comando da copiare; esecuzione solo su richiesta esplicita tramite `pkexec` (finestra di autenticazione polkit). Mai `sudo` silenzioso, mai elevazione automatica (PROGETTO §F6/§11).
5. **Link manuale** (`manual_url`, già nel catalogo).

`cargo install` non è raccomandato come sorgente predefinita (compilazione lunga, richiede toolchain).

### 5.2 Catalogo Linux

| ID catalogo | Linux | Sorgente preferita senza root | Note |
|---|---|---|---|
| `pwsh` | sì | tarball ufficiale in `~/.local/share/…/powershell/` + link `pwsh` | ~70 MB; opzionale |
| `clink` | **no** | — | Nascosto su Linux |
| `oh-my-posh` | sì | binario ufficiale `posh-linux-amd64/arm64` | Consigliato (come oggi `RECOMMENDED`) |
| `starship` | sì | binario musl | — |
| `nerd-font` | sì | zip in `~/.local/share/fonts/NerdFonts/` + `fc-cache -f` | Rilevamento: `fc-list` contiene «Nerd Font» |
| `zoxide`, `fzf`, `eza`, `bat`, `lazygit`, `tealdeer` | sì | binari musl/statici | `bat` è `batcat` su Debian/Ubuntu: rilevare entrambi i nomi |
| `jq` | sì | binario `jq-linux-amd64` | — |
| **nuovi** `zsh-autosuggestions`, `ble-sh` | sì | tarball in `~/.local/share/cli-superpower/plugins/` | Non sono eseguibili: «installato» = file presenti |
| **nuovi** `kitty`, `wezterm`… | fase successiva | installer ufficiale in `~/.local/kitty.app` | Solo se si supporta il terminale |

Il catalogo diventa **per OS**: `executables` con nomi per piattaforma (`pwsh.exe` vs `pwsh`), sorgenti per piattaforma (`winget_id`, `release: {url, sha256, archive, binary}` per `linux-x86_64`/`linux-aarch64`). La versione si rileva eseguendo `<tool> --version` con timeout (al posto di `FileVersionInfo`).

### 5.3 PATH

`~/.local/bin` è nel `PATH` su Ubuntu/Debian/Fedora solo tramite `~/.profile` o `.bashrc` della distro, e non sempre per zsh/fish. Gli script `init.*` generati aggiungono `~/.local/bin` al `PATH` **solo se manca** (dentro file nostri, quindi senza toccare altro). Il rilevamento dall'app GUI cerca anche in `~/.local/bin`, `~/.cargo/bin`, `/home/linuxbrew/.linuxbrew/bin`, `~/.nix-profile/bin`, `/snap/bin` (equivalente di `search_paths` nel modulo `platform` di `tools.rs`).

### 5.4 Dipendenze nuove nel core

Download e verifica richiedono un client HTTP (`ureq` con rustls) ed estrazione (`flate2`, `tar`, `zip`). Restano fuori dai generatori (I/O) e vanno nel modulo `packages`. Alternativa: eseguire il download nel processo Tauri con il plugin HTTP; si sconsiglia perché `csp` (che non dipende da Tauri) in futuro potrebbe voler installare tool (`csp apply`).

---

## 6. `csp` su Linux (F6, poi F14/F15)

| Funzione | Windows (`port.rs`) | Linux |
|---|---|---|
| Elenco listener | `GetExtendedTcpTable` IPv4/IPv6 | Parsing di `/proc/net/tcp` e `/proc/net/tcp6` (stato `0A` = LISTEN) → inode; mappa inode → PID scandendo `/proc/<pid>/fd/*` (`socket:[inode]`). Parser **puro** testabile con fixture |
| Nome e percorso | `QueryFullProcessImageNameW` | `/proc/<pid>/exe` (readlink), `/proc/<pid>/comm` |
| Avviato da | `GetProcessTimes` | `starttime` (campo 22 di `/proc/<pid>/stat`, in tick) + `btime` di `/proc/stat` |
| Processo critico | `IsProcessCritical` | PID 1, thread del kernel (`PF_KTHREAD` o `exe` assente), UID diverso dal nostro → rifiuto |
| Identità prima di terminare | handle + ora di creazione | **`pidfd_open`** (Linux ≥ 5.3) + confronto `starttime` + `pidfd_send_signal(SIGTERM)`; attesa fino a 5 s con `poll` sul pidfd |
| Processi di altri utenti | visibili con privilegi | `/proc/<pid>/fd` non leggibile: elencare la porta come «occupata da un processo di un altro utente», suggerire `sudo ss -ltnp 'sport = :4200'`, mai elevare |

Crate consigliato: `rustix` (API sicure per `pidfd`, `kill`, `poll`) oppure `libc`. Va aggiunto in `[target.'cfg(target_os = "linux")'.dependencies]` di `crates/csp/Cargo.toml`. Il parser di `/proc/net/tcp*` va scritto in un modulo comune e testato su ogni OS; macOS userà `libproc`/`lsof`, quindi l'interfaccia `listeners(port) -> Vec<Listener>` è **condivisa con macOS**.

v2: `csp notify` → notifiche D-Bus (`notify-rust`, funziona anche su macOS); `csp table --copy` con `arboard`: su Linux gli appunti appartengono al processo e si perdono alla sua uscita (X11 e Wayland). Usare `arboard` con l'estensione Linux `wait()` (processo figlio che resta finché un altro programma prende gli appunti) o delegare a `wl-copy`/`xclip` se presenti. Wayland richiede la feature `wayland-data-control`.

Distribuzione di `csp`: oggi non viene installato nel `PATH` nemmeno su Windows. Su Linux: in `.deb`/`.rpm` come `/usr/bin/csp` (sidecar Tauri `externalBin`); con AppImage l'app lo copia in `~/.local/bin/csp` come file gestito dal manifest.

---

## 7. Workspace (F4)

`Desktop::launch` oggi chiama `wt.exe` con `generators::workspace_args`. Su Linux ogni terminale ha un generatore puro e un comando di lancio:

| Terminale | Generazione | Split |
|---|---|---|
| Konsole | file `--tabs-from-file` (`title: …;; workdir: …;; profile: …;; command: …`) | `--layout` JSON o D-Bus `org.kde.konsole` (spike S4); in mancanza → tab con avviso |
| GNOME Terminal | `gnome-terminal --window --tab --profile=… --working-directory=… -- <shell> …` per ogni tab | → tab con avviso |
| Ptyxis | `ptyxis --new-window -d DIR -x CMD`, `--tab` | → tab con avviso |
| Kitty (dopo MVP) | file di sessione (`new_tab`, `layout splits`, `cd`, `launch --location=vsplit`) | ✅ |

`startupCommand` (codice nella sintassi della shell scelta, come da schema) va eseguito lasciando la shell interattiva aperta (equivalente di `-NoExit`):

| Shell | Argomenti |
|---|---|
| bash | `bash -i -c 'eval "$1"; exec bash -i' csp "<comando>"` (il comando viaggia come argomento, non interpolato) |
| zsh | `zsh -i -c 'eval "$1"; exec zsh -i' csp "<comando>"` |
| fish | `fish -i -C "<comando>"` (`--init-command`) |
| pwsh | `pwsh -NoExit -EncodedCommand <base64 UTF-16LE>` (come oggi) |

`-i` carica `.bashrc`/`.zshrc`, quindi i comandi rapidi sono disponibili nel workspace.

---

## 8. Architettura cross-platform

### 8.1 Problema attuale

`Desktop::plan` (`desktop.rs`) è una funzione unica che: genera il prompt, individua `settings.json` di WT (`profiles`), produce fragment e immagini, modifica `defaultProfile` con il CST JSONC, prepara lo script Clink (`clink::profile_directory`), interroga `pwsh.exe`/`powershell.exe` (`shell_profile`) e aggiorna `$PROFILE` (`profile_block`). `Desktop::apply` chiama anche `sync_clink`, `Desktop::launch` chiama `wt.exe`. Aggiungere Linux qui dentro con `cfg` renderebbe il file ingestibile.

### 8.2 Astrazioni proposte (**condivise con macOS**)

I nomi seguono la struttura già proposta in `docs/porting/macos/ANALISI.md` §7.2, per avere un solo set di astrazioni. Le aggiunte richieste da Linux sono marcate `// Linux`.

```
crates/core/src/
├─ platform.rs            enum Os; PlatformPaths; trait Probe (+ SystemProbe; FakeProbe nei test)   // Probe: Linux
├─ shells/
│  ├─ mod.rs              trait ShellIntegration; struct ShellFacts; rc_block(...)
│  ├─ powershell.rs       ex desktop.rs::{shell_profile, profile_block, update_profile_text, OH_MY_POSH_INIT}
│  ├─ cmd.rs              ex desktop.rs (script Clink, sync_clink) — solo Windows a runtime
│  └─ bash.rs, zsh.rs, fish.rs
├─ terminals/
│  ├─ mod.rs              trait TerminalIntegration; struct TerminalFacts; Capabilities
│  ├─ windows_terminal.rs ex desktop.rs::{TerminalSettings, profile_list, profile_shell, fragment, defaultProfile, wt.exe}
│  ├─ konsole.rs          // Linux
│  └─ gnome.rs            // Linux: GNOME Terminal + Ptyxis (backend dconf comune)
├─ packages/
│  ├─ mod.rs              trait PackageManager; catalogo con sorgenti per OS
│  ├─ winget.rs, brew.rs
│  ├─ release.rs          // Linux (utile anche a macOS): binari ufficiali con SHA-256
│  └─ system.rs           // Linux: apt/dnf/pacman/zypper, solo comando proposto o pkexec esplicito
├─ color_schemes.rs       catalogo schemi con palette complete
├─ generators/            puri: esistenti + bash/zsh/fish, konsole, gnome, ini, gvariant
├─ apply.rs               + PlannedChange per preferenze (macOS `defaults`, Linux dconf)
└─ desktop.rs             orchestratore indipendente dall'OS
```

```rust
pub enum Os { Windows, Linux, MacOs }

/// Percorsi risolti a runtime dal chiamante (Tauri path API, variabili XDG, known folders).
/// Campi come nell'analisi macOS (`home`, `data`, `state`, `generated`) più quelli richiesti da Linux.
pub struct PlatformPaths {
    pub home: PathBuf,
    pub data: PathBuf,      // config.personal.json, team/   (Linux: ~/.config/cli-superpower)
    pub state: PathBuf,     // manifest, history, lock, backgrounds, tools (Linux: ~/.local/share/cli-superpower)
    pub generated: PathBuf, // script generati (Linux: ~/.local/share/cli-superpower/generated)
    pub user_bin: PathBuf,  // Linux: ~/.local/bin
}

/// Linux: tutto l'I/O di scoperta passa da qui, così la scoperta si testa senza toccare il sistema.
pub trait Probe: Send + Sync {
    fn run(&self, program: &str, args: &[&str], env: &[(&str, &str)]) -> io::Result<Output>;
    fn read(&self, path: &Path) -> Result<Option<Vec<u8>>, String>;
    fn env(&self, key: &str) -> Option<OsString>;
    fn which(&self, program: &str) -> Option<PathBuf>;
}

pub struct Capabilities { pub per_profile: bool, pub background_image: bool, pub padding: bool,
    pub blur: bool, pub splits: bool, pub per_profile_font: bool, /* … */ }

pub trait TerminalIntegration {
    fn id(&self) -> &'static str;                 // "windows-terminal", "konsole", "gnome-terminal", "ptyxis", …
    fn discover(&self, paths: &PlatformPaths) -> Result<Option<TerminalFacts>, String>;   // I/O (via Probe)
    fn capabilities(&self) -> Capabilities;
    fn plan(&self, effective: &Value, facts: &TerminalFacts)
        -> Result<(Vec<PlannedChange>, Vec<PreviewFile>), String>;                        // puro
    fn launch_workspace(&self, effective: &Value, id: &str, facts: &TerminalFacts) -> Result<(), String>;
}

pub trait ShellIntegration {
    fn id(&self) -> &'static str;                 // "pwsh", "powershell", "cmd", "bash", "zsh", "fish"
    fn discover(&self, paths: &PlatformPaths) -> Result<Option<ShellFacts>, String>;      // rc, versione, collisioni
    fn plan(&self, effective: &Value, facts: &ShellFacts, paths: &PlatformPaths)
        -> Result<(Vec<PlannedChange>, Vec<PreviewFile>), String>;
    fn startup_args(&self, command: &str) -> Vec<String>;   // Linux: per i workspace (§7)
}

pub trait PackageManager {
    fn id(&self) -> &'static str;                 // "winget", "brew", "release", "system"
    // firme di locate/install/uninstall come nell'analisi macOS; "release" e "system" aggiunte da Linux
}
```

`Desktop` diventa un orchestratore indipendente dall'OS: risolve la config, chiede a terminali e shell disponibili di pianificare, passa tutto ad `apply::apply`. Quali implementazioni registrare si decide in un solo punto con `cfg(target_os)`.

**Preferenze non su file.** L'analisi macOS prevede `PlannedChange::Preference` per `defaults`/CFPreferences; su Linux lo stesso concetto copre le chiavi dconf (§2.5). Va definito **una volta**: un `PlannedChange` con lettura/scrittura delegate a uno «store» (file, `defaults`, dconf), mentre journal, manifest con hash e ripristino restano quelli di `apply.rs`.

### 8.3 Cosa resta comune e cosa va dietro `cfg`

| Sempre compilato, su ogni OS (puro, testabile in CI ovunque) | `cfg(unix)` (Linux + macOS) | `cfg(target_os = "linux")` | `cfg(windows)` |
|---|---|---|---|
| `config/*`, logica di `apply.rs`, **tutti** i generatori (WT, Clink, Konsole, GNOME, bash/zsh/fish, iTerm2…), `rc_block`, `color_schemes`, catalogo tool (dati), parser `/proc/net/tcp`, `csp` parsing e flusso | permessi e symlink in `atomic_write`; scoperta bash/zsh/fish; ricostruzione `PATH`; `kill` | scoperta Konsole/GNOME/Ptyxis, store dconf, `pidfd` e `/proc` in `csp port`, `fc-list`, gestori di sistema e `pkexec` | scoperta WT, Clink e registro, winget, `windows-sys`, `CREATE_NO_WINDOW` |

Regola: i generatori di **tutte** le piattaforme si compilano e si testano su tutte le piattaforme; solo la scelta delle integrazioni attive avviene con `cfg`. Così la CI Windows verifica anche gli snapshot Linux e viceversa.

### 8.4 Altre astrazioni condivise con macOS

- `rc_block` (in questo documento anche «`rc_block`»: usare **un solo nome**, quello di `shells/mod.rs`) con marcatori a sintassi di commento variabile e newline di default per OS.
- Generatori bash/zsh/fish (bash compatibile 3.2 per macOS).
- Catalogo **temi con palette completa** (16 colori + fg/bg/cursore) nel core: su Windows WT risolve i nomi da sé, su Linux/macOS bisogna scrivere i colori. La UI (`appearanceThemes`) dovrebbe leggere le anteprime dallo stesso catalogo.
- Elenco font di sistema (`fc-list` su Linux, Core Text su macOS) esposto alla UI.
- Sorgente «release» del catalogo tool e Homebrew.
- Apertura link con `tauri-plugin-opener` al posto di `rundll32` (`open_link`).
- Kitty/WezTerm come backend terminale.

---

## 9. Impatto sullo schema di configurazione (`schemaVersion` 7)

| Cambio | Dettaglio | Compatibilità |
|---|---|---|
| Nuovi valori di profilo/shell | `PROFILES` diventa `pwsh, powershell, cmd, bash, zsh, fish` per `appearance`, `commands[].shells`, `workspaces[].…pane.profile`, `shellBehavior`, `globalTerminal.defaultProfile` | v6 → v7: solo incremento di versione, come i salti esistenti in `config::migrate` |
| Percorsi | `windows_path` → `absolute_path`: accetta i percorsi Windows attuali **e** POSIX assoluti (`/…`, senza componenti `..`) | Allarga i valori ammessi |
| Terminali scelti | `globalTerminal.terminals`: facoltativo, array di ID fra `windows-terminal`, `iterm2`, `apple-terminal` (macOS), `konsole`, `gnome-terminal`, `ptyxis` (estendibile: `kitty`, `wezterm`, …); solo nel file personale. Se assente: tutti i terminali supportati e rilevati sull'OS corrente (definizione dell'analisi macOS §6) | Campo nuovo, facoltativo |
| Validazione per piattaforma | **non** nello schema: un pacchetto team con `C:\src` o `shells: ["cmd"]` resta valido ovunque. È la pianificazione (`Desktop::plan`) a saltare con avviso le voci non applicabili sull'OS corrente (percorso non nativo, shell assente) | Pacchetti team condivisi tra colleghi Windows e Linux |
| Prefisso | Regola nuova in `validate_effective`: `commandPrefix: "/"` + shell `bash` o `fish` (se lo spike S1 conferma) → errore esplicativo | — |

Aggiornamenti collegati: `docs/config-schema.md` (nuovi valori, nota sulla portabilità dei pacchetti), `models.ts` (`Profile`, `schemaVersion: 1..7`), mock di `config-bridge.ts` (accettare `schemaVersion` 7), validazione TS (`config-validation.ts`).

Domanda aperta collegata: percorsi relativi alla home (`~/src/efm`) per pacchetti team condivisi tra macchine diverse (§13, Q5).

---

## 10. Impatto sulla UI Angular (stessa UX)

Principio: **stesse sezioni, stessi flussi** (home con le shell, editor aspetto con anteprima, comandi, workspace, prompt, autocomplete, team, tool, cronologia). Cambiano solo le **opzioni disponibili**, che arrivano dal backend.

| Elemento | Oggi | Con Linux |
|---|---|---|
| Elenco shell/profili | costante `profiles` (`app.ts:19`) + `terminal_profiles` | nuovo comando IPC `platform_info` → `{os, terminal: {id, name, capabilities}, terminals: […], shells: [{id, label, installed, version, note}], pathExample}`; la UI costruisce `profiles` e `behaviorShells` da qui |
| Controlli aspetto | tutti visibili | nascosti o disattivati con spiegazione se la capacità manca (es. «GNOME Terminal non supporta immagini di sfondo»); `useAcrylic` diventa «Sfocatura» su Konsole |
| Temi | 8 nomi WT (`appearanceThemes`) | stesso elenco, colori dal catalogo del core |
| Font | lista fissa (`terminalFonts`) | lista da `list_fonts` (font monospazio di sistema) |
| Percorsi | placeholder e messaggio Windows (`config-validation.ts:9-10`) | esempio e regex secondo `platform_info.os` |
| Testi | «Windows Terminal» in `app.html` | nome del terminale attivo |
| Anteprima delle shell in home | prompt Windows cablati (`app.html:47-48`) | prompt di esempio per shell (`PS C:\Progetti>` su Windows, `~/progetti $` per bash/zsh, `~/progetti>` per fish) |
| Programma `npm` negli assistenti | `npm.cmd` (`app.ts:440`, `app.ts:490`) | `npm` fuori da Windows (o ovunque, se su Windows `invoke-native.ps1` lo risolve: domanda condivisa con macOS) |
| Scelta del terminale | — | selettore «Terminali da configurare» (solo se ce n'è più di uno), salva `globalTerminal.terminals`; «Apri workspace» usa il terminale del desktop |
| Tool | `canInstall` da winget | `canInstall` da qualsiasi `PackageManager`; per i gestori di sistema la UI mostra il comando e il pulsante «Installa con password di amministratore» (pkexec) |
| Simulazione browser | profili Windows | parametro `?os=linux` nel mock di `ConfigBridge` per sviluppare la UI Linux senza backend (approccio «prima il frontend», PROGETTO §8.1) |

Il layout, il design system (`DESIGN.md`) e il flusso Anteprima → Applica → Cronologia non cambiano. WebKitGTK rende Angular come Chromium per questo tipo di UI; da verificare font (il sistema potrebbe non avere i font citati in `DESIGN.md`: includerli nel bundle) e `backdrop-filter` (supportato in WebKitGTK recente).

---

## 11. Packaging, aggiornamenti, firma, WebKitGTK

### 11.1 Formati

| Formato | Pro | Contro | Raccomandazione |
|---|---|---|---|
| **AppImage** | Per utente, senza root (coerente col principio «installazione per utente» di PROGETTO §7); aggiornabile dall'updater Tauri | ~70-100 MB (include WebKitGTK); richiede **libfuse2**, assente di default su Ubuntu ≥ 22.04 (va spiegato, o usare `--appimage-extract-and-run`); nessuna voce di menu senza AppImageLauncher (l'app può crearla in `~/.local/share/applications/` come file gestito) | **Primario** |
| **.deb** / **.rpm** | Integrazione col sistema, dipendenze condivise (pacchetto piccolo) | Richiedono root per installare; l'aggiornamento automatico dipende dal supporto dell'updater (storicamente solo AppImage su Linux; versioni recenti del plugin aggiungono deb/rpm: **da verificare** sulla versione adottata) | Secondario |
| **Flatpak** | Distribuzione su Flathub | La sandbox impedisce di scrivere `~/.bashrc`, `~/.config/…` e di eseguire `zsh`, `dconf`, `konsole` dell'host senza `--filesystem=home` e `flatpak-spawn --host` (`--talk-name=org.freedesktop.Flatpak`): sandbox annullata, revisione Flathub difficile | **No in v1** |
| Snap | — | Confinamento simile; `classic` richiede approvazione | No |
| Tarball | Semplice | Nessuna integrazione | Facoltativo |

Build su **ubuntu-22.04**: glibc 2.35 → l'AppImage gira anche su distro più vecchie del runner più recente.

### 11.2 Updater Tauri

L'updater non è ancora integrato su Windows. Quando lo sarà (PROGETTO §6): `latest.json` con piattaforma `linux-x86_64` (e `linux-aarch64`) che punta all'AppImage firmata (stessa chiave minisign dell'updater, nessun certificato di sistema). Dopo l'aggiornamento, rilancio di Applica come su Windows.

### 11.3 Firma

Linux non ha SmartScreen né un obbligo di firma. Restano: firma dell'updater (obbligatoria per Tauri), checksum SHA-256 pubblicati nella release, facoltativamente firma GPG di `.deb`/`.rpm` e dell'AppImage.

### 11.4 WebKitGTK e dipendenze

- Tauri 2 richiede **webkit2gtk-4.1** (Ubuntu 22.04+, Debian 12+, Fedora 36+). Dipendenze di build in CI: `libwebkit2gtk-4.1-dev`, `build-essential`, `curl`, `wget`, `file`, `libxdo-dev`, `libssl-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev` (elenco dei prerequisiti ufficiali Tauri v2).
- Dipendenze runtime dichiarate nel `.deb`/`.rpm`: `libwebkit2gtk-4.1-0`, `libgtk-3-0`.
- Noti problemi: finestra bianca con driver NVIDIA proprietari su Wayland → variabile `WEBKIT_DISABLE_DMABUF_RENDERER=1` impostata dall'app prima di creare la webview se rileva quel caso (o documentata).
- Icone: servono PNG (32, 128, 256, 512) oltre all'`.ico`.
- Configurazione specifica per OS con `tauri.linux.conf.json` (Tauri 2 unisce i file per piattaforma): niente `fetch-clink` né risorsa `clink/`, bundle `appimage`, `deb`, `rpm`.

---

## 12. Distro, Wayland e X11

| Tema | Impatto | Gestione |
|---|---|---|
| Debian/Ubuntu vs Fedora vs Arch vs openSUSE | gestore pacchetti, nomi pacchetti (`bat`/`batcat`, `fd`/`fdfind`), versioni di fish/zsh | Rilevamento da `/etc/os-release`; sorgente «release» indipendente dalla distro |
| Distro immutabili (Fedora Silverblue/Kinoite, SteamOS, Bazzite) | `/usr` in sola lettura, `rpm-ostree` per i pacchetti | Solo installazioni in `~/.local`; nessun fallback di sistema proposto |
| NixOS / home-manager | dotfile symlink verso `/nix/store` (sola lettura); dconf può essere riscritto da home-manager | Rilevare e rifiutare con spiegazione (§4.4); mostrare il blocco da integrare nella config Nix |
| Versioni minime | bash ≥ 4.4 (Linux), fish ≥ 3.4 per alcune opzioni, zsh ≥ 5.8, fzf ≥ 0.48 per `--bash/--zsh/--fish` | Controlli di versione nel codice generato, come quelli per PSReadLine in `append_behavior` |
| Wayland | niente scorciatoie globali per app né posizionamento finestre: incide sul **quake mode (F12, v2)**; appunti legati al processo (F15) | F12 delegato a terminali dedicati (Yakuake su KDE, Guake su GNOME, `kitten quick-access-terminal`), con scorciatoia registrata nel desktop (kglobalaccel / tasti personalizzati GNOME via dconf) |
| X11 | come sopra, meno restrizioni | — |
| Compositor | trasparenza/sfocatura funzionano solo con compositor attivo | Avviso nell'anteprima se l'opacità è < 100 |
| Login shell vs shell non di login | `.bash_profile` vs `.bashrc` | Iniettiamo in `.bashrc` (i terminali avviano shell non di login); Konsole/GNOME con «login shell» attivo leggono `.bash_profile`, che sulle distro principali carica `.bashrc` (da avvisare se non lo fa) |

---

## 13. Funzionalità senza equivalente diretto

| Funzionalità Windows | Problema su Linux | Soluzione proposta |
|---|---|---|
| Fragment JSON di WT (estensione documentata) | Nessuno standard comune | Backend per terminale; Konsole con file nostri; GNOME con profili nostri in dconf (§2) |
| Aspetto per profilo su terminali senza profili (Kitty, Ghostty, Alacritty) | Un solo aspetto globale | Aspetto unico con avviso, o profili simulati con file `--config` e launcher `.desktop` dedicati (dopo MVP) |
| Immagine di sfondo / padding su GNOME Terminal e Ptyxis | Non supportati | Controlli disattivati con spiegazione; suggerimento di Konsole/Kitty |
| Acrilico | Concetto Windows | «Sfocatura» su Konsole (KDE), nascosto altrove |
| Clink per cmd | cmd non esiste | Funzionalità native delle shell (§3) |
| Windows PowerShell 5.1 | Non esiste | Shell nascosta; un pacchetto team che la usa produce un avviso |
| Policy di esecuzione | Non esiste | Avviso eliminato |
| winget | Nessun gestore universale senza root | Sorgente «release» con SHA-256 + Homebrew + comando di sistema via pkexec (§5) |
| Registro (AutoRun, font) | Non esiste | Nessun AutoRun da gestire; font via `fc-list` |
| Prefisso `/` in bash e fish | Nome trattato come percorso / non ammesso | zsh via ZLE, fish via `abbr --regex`, bash non supportato (§3.2) |
| `HISTIGNORE` di bash vs `MemoryOnly` | bash toglie la riga anche dalla memoria | Comportamento documentato nella nota della shell |
| Quake mode (v2) | Wayland blocca le scorciatoie globali | Terminali a scomparsa dedicati (§12) |
| `rundll32` per i link | — | `tauri-plugin-opener` / `xdg-open` |
| Installer NSIS per utente | deb/rpm richiedono root | AppImage per utente |
| Notifiche Windows (F14) | — | D-Bus (`notify-send`/`notify-rust`) |

---

## 14. Rischi

| Rischio | Impatto | Probabilità | Mitigazione |
|---|---|---|---|
| Refactoring di `desktop.rs` rompe il comportamento Windows | Regressioni sull'unica piattaforma oggi funzionante | Media | Fase L0 senza cambi funzionali; test esistenti (`preview_apply_and_remove_integrate_without_touching_live_terminal`, test di `apply.rs`) devono passare invariati su Windows prima di aggiungere Linux |
| Frammentazione dei terminali | Lavoro moltiplicato, UX diseguale | Alta | Due backend nell'MVP, matrice di capacità esposta alla UI, generatori puri per backend |
| dconf: chiave `list` implicita, scritture concorrenti di GNOME Terminal aperto | Profili persi o duplicati | Media | Spike S3; `shared_input` + controllo hash; UUID deterministici |
| Dotfile symlink (Stow, chezmoi, Nix) | Errore o sovrascrittura del link | Media | Risoluzione controllata e rifiuto per target in sola lettura (§4.4) |
| Framework di shell (oh-my-zsh, p10k, bash-it) in conflitto con prompt/plugin | Prompt doppio, avvio lento | Alta | Rilevamento e avviso; nostro blocco in coda; plugin caricati solo se l'opzione è attiva |
| ble.sh instabile o lento con alcune configurazioni | bash inutilizzabile | Media | Solo opzionale, mai di default; disattivazione con un clic |
| Download di binari dalla rete | Catena di fornitura | Media | Versioni fissate, SHA-256 nel catalogo compilato, solo HTTPS da domini ufficiali (GitHub Releases), nessun `curl | sh` |
| WebKitGTK diverso tra distro (versioni, bug grafici NVIDIA/Wayland) | Finestra bianca o glitch | Media | Build su 22.04, workaround DMABUF, smoke test su GNOME e KDE |
| `libfuse2` assente per AppImage | L'app non parte | Alta su Ubuntu recenti | Istruzioni nella pagina di download; `.deb` come alternativa |
| Updater limitato ad AppImage | Utenti deb/rpm senza aggiornamenti automatici | Media | Verificare il plugin; altrimenti notifica «nuova versione» con link |
| Prefisso `/` non disponibile in bash | UX diversa da Windows per chi usa bash | Certa | Validazione esplicita e suggerimento `x-` |
| Pacchetti team misti Windows/Linux | Comandi o percorsi non applicabili | Alta in team misti | Salto con avviso in pianificazione (§9); eventuale `~` (Q5) |
| Assenza di `insta` e di tipi TS generati | Divergenza tra Rust e TS aumentata da nuovi enum | Media | Introdurre `insta` (L1.1); valutare `ts-rs` (domanda Q8) |

---

## 15. Spike consigliati prima dell'implementazione

| ID | Domanda | Esito atteso |
|---|---|---|
| S1 | `/nome` in bash, zsh, fish: funzione, alias, `command_not_found_handle`, ZLE, `abbr --regex` | Tabella come in `docs/spikes/fase-0.md` |
| S2 | Formato profilo/colorscheme Konsole (chiavi `Wallpaper`, `Opacity`, `Blur`, `TerminalMargin`, `CursorShape`), rilettura a caldo, `--tabs-from-file`, `--layout` | Fixture reali per gli snapshot |
| S3 | dconf GNOME Terminal e Ptyxis: lista profili implicita, UUID, palette Ptyxis, effetto con terminale aperto; `dconf dump/load` in `dbus-run-session` | Fixture + procedura di test CI |
| S4 | Workspace: split in Konsole (layout o D-Bus); tab in GNOME Terminal/Ptyxis con profilo e cartella | Comandi di lancio verificati |
| S5 | `pidfd` + `/proc` per `csp port` con listener di test, anche di altro utente | Prototipo con test |
| S6 | Tauri su Linux: AppImage su Ubuntu 22.04/24.04, Fedora, Arch; updater su AppImage e deb; NVIDIA/Wayland | Matrice di compatibilità |

---

## 16. Domande aperte per l'utente

1. **Terminali MVP:** confermi Konsole + GNOME Terminal/Ptyxis, con Kitty/WezTerm nella fase condivisa con macOS? Oppure preferisci partire da un solo terminale cross-platform (es. Kitty) accettando l'assenza di profili per shell?
2. **Distro di riferimento** per test e supporto ufficiale: Ubuntu LTS 24.04 + Fedora Workstation? Arch «best effort»?
3. **Architetture:** solo x86_64 o anche aarch64 dalla prima release?
4. **bash:** accetti che il prefisso `/` non sia disponibile in bash (fallback `x-`), o vuoi investire in ble.sh per emularlo?
5. **Percorsi nei pacchetti team:** introdurre `~/…` (relativo alla home) nello schema per pacchetti condivisi tra macchine/OS diversi?
6. **Installazioni di sistema:** l'app può proporre `pkexec apt install …` (con password dell'utente) oppure deve solo mostrare il comando?
7. **Distribuzione:** AppImage + deb + rpm su GitHub Releases è sufficiente, o serve un repository APT/COPR/AUR?
8. **Tipi TypeScript:** con nuovi enum (`Profile`, terminali) conviene adottare ora `ts-rs`/tauri-specta (previsti da PROGETTO §7 ma non in uso) invece di aggiornare `models.ts` a mano?
9. **Nome cartelle:** `~/.config/cli-superpower` (convenzione Linux) o `CliSuperpower` come su Windows?
10. **pwsh su Linux:** va supportato già nella prima release Linux o dopo bash/zsh/fish?
