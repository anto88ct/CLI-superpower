# Regole aggiuntive per lo sviluppo macOS

Valgono **in aggiunta** a `CLAUDE.md` (che resta prioritario) per ogni task di `docs/porting/macos/PIANO.md`. Motivazioni e dettagli in `ANALISI.md`.

## Architettura

- I generatori restano puri e **senza `cfg`**: zsh, bash, fish, iTerm2, Terminal.app e AppleScript si compilano e si testano su ogni OS, così i loro snapshot girano anche nella CI Windows. Percorsi, versioni e capacità arrivano come parametri (struct di fatti prodotte dalla scoperta), mai da `std::env`, dal file system o da `Command`.
- `cfg` solo nella scoperta e nell'esecuzione: `#[cfg(unix)]` per ciò che vale su macOS e Linux (permessi, symlink, shell POSIX, `kill`), `#[cfg(target_os = "macos")]` per ciò che è solo Apple (iTerm2, Terminal.app, `defaults`, `osascript`, `libproc`). La scelta delle integrazioni da registrare sta in un solo punto.
- Codice marcato **[condivisa con Linux]** nel piano: prima di modificarlo leggere `docs/porting/linux/` e mantenere un'unica implementazione.
- Non duplicare la logica di `config.rs` per OS: la validazione è comune (valori di tutte le piattaforme ammessi), è il planner a ignorare ciò che non si applica e a dirlo in anteprima.

## File dell'utente su macOS

- Gli unici file utente che si modificano sono: il file rc della shell (`${ZDOTDIR:-$HOME}/.zshrc`; per bash il file di login effettivo e `.bashrc`) e `$PROFILE` di pwsh, sempre e solo nel blocco tra `# >>> CLI Superpower >>>` e `# <<< CLI Superpower <<<`. Per fish si usa un file nostro in `conf.d/`, mai `config.fish`.
- Newline `\n`, UTF-8 senza BOM per tutti i file POSIX. Non riusare il default CRLF di `update_profile_text`.
- Preservare il modo del file (`0600` resta `0600`). Un file rc symlink va risolto con la funzione dedicata dell'applicatore (decisione D5), mai sostituito con un file normale.
- Il blocco rc contiene il percorso assoluto dello script, quotato (il percorso contiene `Application Support`). Mai cablare percorsi: home, `ZDOTDIR`, `__fish_config_dir` e `$PROFILE` si chiedono alla shell o all'API.
- Per trovare i percorsi o le collisioni non caricare i file dell'utente: `zsh -c` / `zsh -f`, `bash --noprofile --norc`, `fish --no-config`, `pwsh -NoProfile`. Ogni processo figlio ha un timeout.

## Preferenze di altre app

- Mai leggere o scrivere direttamente i file in `~/Library/Preferences/*.plist`: passare da `/usr/bin/defaults` (o CFPreferences), perché cfprefsd tiene una cache.
- Solo chiavi gestite e dichiarate nel piano (es. `Default Bookmark Guid` di iTerm2), registrate nel manifest con il valore precedente, ripristinabili, con controllo di conflitto.
- I profili iTerm2 si gestiscono **solo** con file nostri in `DynamicProfiles`, con GUID deterministici. Immagini di sfondo sempre copiate nella cartella dell'app (`…/CliSuperpower/backgrounds`).

## Processi, permessi, privacy

- Mai `sudo`, mai richieste di password, mai installare Homebrew. I cask che chiedono la password diventano "installazione manuale".
- Un'app avviata dal Dock non ha il PATH della shell: usare sempre il PATH ricostruito di `platform.rs` per trovare `brew`, shell e tool; passarlo esplicitamente ai processi figli.
- Non verificare dall'app l'esistenza di percorsi scelti dall'utente in Scrivania, Documenti, Download o iCloud Drive (scatena richieste TCC); la shell li verifica quando esegue.
- Gli unici permessi di sistema ammessi sono Automazione (per i workspace, con `NSAppleEventsUsageDescription` ed entitlement `com.apple.security.automation.apple-events`). Niente Accessibilità, niente Full Disk Access, niente App Sandbox.
- Ogni errore di permesso diventa un messaggio in italiano con il percorso esatto in Impostazioni di Sistema.

## Quoting e script generati

- zsh/bash: letterali tra apici singoli (`'` → `'\''`); fish: apici singoli con `\'` e `\\`; AppleScript: `\` e `"`. Un NUL è un errore del generatore. Le funzioni di quoting stanno in `generators/quote.rs` e hanno test propri.
- bash generato compatibile con `/bin/bash` 3.2 (niente array associativi, `${x,,}`, `mapfile`, `declare -n`).
- Gli script devono essere idempotenti se ricaricati (`source ~/.zshrc` due volte): guardie sulle variabili globali, `add-zsh-hook` invece di accodare a mano, binding precedente memorizzato una volta (come `$global:__CspTabFallback` in PowerShell).
- Nessun output all'avvio fuori dal banner, e il banner solo se interattivo, con terminale e larghezza sufficiente (compatibilità con l'instant prompt di powerlevel10k).

## Test e verifiche

- Ogni nuovo generatore ha snapshot `insta` + casi limite (apici, spazi, Unicode, `{{ }}`, parametri facoltativi, interi).
- I test che toccano il file system usano una `PlatformPaths` interamente temporanea (HOME, ZDOTDIR, XDG finti). Nessun test scrive nella HOME reale o nelle preferenze reali: i test sulle preferenze usano un dominio `it.clisuperpower.test.<pid>` e lo rimuovono.
- Sul job `macos-latest`: sintassi (`zsh -n`, `/bin/bash -n`, `fish --no-execute`, `osacompile -o /dev/null`) e comportamento (`zsh -i -c` con ZDOTDIR temporanea) degli script generati.
- Prima di dichiarare concluso un task macOS: verifiche V-base di `PIANO.md`, più V-mac quando il task tocca l'app Tauri. Se il task è stato sviluppato su Windows, la verifica macOS è il job della CI: dichiararlo esplicitamente se non è ancora stato eseguito.
- Ogni ipotesi marcata "(da verificare)" in `ANALISI.md` si conferma con uno spike in `docs/spikes/macos.md` prima di diventare codice.
