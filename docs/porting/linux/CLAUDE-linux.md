# Regole aggiuntive per lo sviluppo Linux

Valgono **in aggiunta** a `CLAUDE.md` durante l'esecuzione di [`PIANO.md`](PIANO.md). In caso di dubbio prevale `CLAUDE.md`.

## Architettura

- Struttura condivisa con il porting macOS: `platform.rs` (`Os`, `PlatformPaths`, `Probe`), `shells/` (`ShellIntegration`), `terminals/` (`TerminalIntegration`), `packages/` (`PackageManager`), `color_schemes.rs`. Prima di creare un modulo, controllare se il porting macOS lo ha già creato e riusarlo.
- Le integrazioni attive per l'OS si registrano con `cfg(target_os = …)` **in un solo punto** (`platform.rs`); `cfg` altrove solo nel codice che chiama API di sistema. Il codice usato da Linux e macOS va marcato «condiviso con macOS» nel commento del modulo.
- **I generatori si compilano e si testano su tutti gli OS**, compresi quelli Windows, Konsole, GNOME, bash/zsh/fish. Mai mettere un generatore dietro `cfg`.
- Ogni accesso al sistema in fase di rilevamento passa dal trait `Probe`: nei test si usa `FakeProbe`. Nessun test legge o scrive la vera home, il vero dconf o il vero `~/.bashrc`.
- Le chiavi dconf si toccano solo tramite i percorsi virtuali `dconf-dir:` / `dconf-key:` dell'applicatore, mai con `Command::new("dconf")` sparso nel codice.

## File dell'utente

- File utente toccati su Linux: `~/.bashrc`, `${ZDOTDIR:-$HOME}/.zshrc`, `$PROFILE` di pwsh, `~/.config/konsolerc` (solo `DefaultProfile`), chiavi dconf `list`/`default` dei profili GNOME/Ptyxis. Nient'altro. In particolare: mai `~/.profile`, `~/.bash_profile`, `~/.zshenv`, `/etc/*`, mai `chsh`.
- Per fish non si modifica alcun file utente: solo `conf.d/cli-superpower.fish`.
- Blocchi sempre con `shells::rc_block`; newline `\n`; percorso dello script generato **assoluto e quotato** con `sh_literal`.
- Gli script generati iniziano con la guardia interattiva e non producono output in shell non interattive (rompono `scp`, `rsync`, `git` via ssh).
- Symlink: si seguono solo per i file utente condivisi e solo verso file regolari scrivibili sotto la home; mai scrivere in `/nix/store` o fuori dalla home.
- Conservare i permessi dei file sostituiti.

## Script di shell generati

- bash: sintassi compatibile **bash 3.2** (macOS) salvo blocchi protetti da controllo di versione. Niente `eval` dei valori dell'utente: i parametri arrivano come `"$1"`, i letterali con `sh_literal`.
- Usare `command <programma>` per evitare che un alias o una funzione con lo stesso nome intercetti il comando rapido.
- Ogni integrazione esterna (zsh-autosuggestions, ble.sh, fzf, oh-my-posh) controlla presenza e versione prima di essere attivata; se manca, la shell parte comunque senza errori.
- Verifiche minime in test: `bash -n`, `zsh -n`, `fish --no-execute` sugli snapshot, più almeno un test comportamentale per generatore (saltato con messaggio se la shell non è installata).

## Strumenti e rete

- Nessun `curl | sh`. Download solo da URL HTTPS del catalogo, con SHA-256 fissato nel codice e verificato **prima** di estrarre.
- Installazioni solo sotto la home (`~/.local/bin`, `~/.local/share/cli-superpower/tools/`, `~/.local/share/fonts/`). Root solo tramite `pkexec`, solo da un'azione esplicita dell'utente nella UI, mai in blocco.
- Si disinstalla solo ciò che è registrato in `installed-tools.json` e il cui hash coincide.

## Percorsi

- Mai scrivere nel codice `/home/…`, `~/.config` o `~/.local/share`: risolverli da `PlatformPaths`, variabili XDG (`XDG_CONFIG_HOME`, `XDG_DATA_HOME`) con i default della specifica, o chiedendo alla shell (`ZDOTDIR`, `$__fish_config_dir`, `$PROFILE`).
- I percorsi di Windows devono restare **identici** a quelli attuali (`%APPDATA%\CliSuperpower`, `%LOCALAPPDATA%\CliSuperpower`): gli utenti Windows esistenti non devono perdere configurazione o manifest.

## Test e CI

- Snapshot con `insta`; rivedere le differenze con `cargo insta review` e committare i file `.snap`.
- Test che richiedono strumenti di sistema: `#[cfg(target_os = "linux")]` + controllo di presenza a runtime con messaggio di salto. dconf solo sotto `dbus-run-session` con `XDG_CONFIG_HOME` temporanea.
- Prima di dichiarare un task concluso: verifica standard di `PIANO.md` su Windows (locale) **e** job `linux` della CI verde. Se non si è potuto provare su un desktop Linux reale (KDE/GNOME), scriverlo nei limiti.

## UI

- Stessa UX: nessuna sezione nuova o rimossa per Linux. Le differenze arrivano da `platform_info` (shell, terminale, capacità) e si mostrano come opzioni disattivate con spiegazione, non come errori.
- I testi Windows esistenti non cambiano quando `os = windows`.
- Testi in italiano, stesso tono di `PRODUCT.md`; nomi propri dei terminali come li scrive il progetto upstream (Konsole, GNOME Terminal, Ptyxis, Kitty).
