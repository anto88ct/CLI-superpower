# CLI Superpower — Documento di progetto

> **Stato:** bozza v0.1 · **Data:** 5 novembre 2026 · **Repo:** https://github.com/anto88ct/CLI-superpower
> **Autore:** Antonio · **Sviluppo:** codice scritto da Claude Code, revisione umana

---

## 1. Visione

CLI Superpower è un'app desktop per Windows che permette a uno sviluppatore di **configurare il proprio terminale da un pannello grafico**, senza conoscere file JSON, profili PowerShell o script di avvio.

L'utente sceglie aspetto, comandi rapidi, autocomplete e strumenti; preme **Applica**; l'app scrive la configurazione corretta in Windows Terminal e nelle shell. L'app si aggiorna da sola e può distribuire un **pacchetto di team** (comandi e impostazioni comuni) che ogni collega personalizza senza perdere le proprie modifiche.

### Problemi che risolve

| Problema oggi | Cosa offre CLI Superpower |
|---|---|
| Bisogna conoscere i comandi a memoria | Autocomplete predittivo, ricerca fuzzy nella cronologia, help con esempi |
| Comandi lunghi ripetuti ogni giorno | Comandi rapidi con parametri (`/ndev`, `/mig Nome`) |
| Terminale spoglio e tutto uguale | Temi, sfondi, font, prompt informativo, per singola shell |
| Output disordinato | Tool moderni preconfigurati + comando `csp` per tabelle e viste interattive |
| Ognuno configura a mano, in modo diverso | Pacchetto team condiviso + preferenze personali |
| Avviare un progetto richiede più finestre | Workspace: un comando apre tutte le tab/pannelli necessari |

### Cosa NON è

- **Non è un nuovo terminale**: usa Windows Terminal, che esiste già ed è ottimo.
- **Non è una nuova shell**: configura cmd e PowerShell, non li sostituisce.
- **Non riscrive l'output dei comandi di sistema** (es. `dir`): offre strumenti alternativi e un formattatore da usare in pipe.

---

## 2. Concetto chiave: i tre strati

"Il terminale" sono in realtà tre componenti distinti. Ogni funzionalità vive in uno strato preciso, e questo determina come si realizza.

| Strato | Cos'è | Cosa controlla | Come lo configuriamo |
|---|---|---|---|
| **1. Terminale** | Windows Terminal (la finestra) | Colori, sfondi, font, trasparenze, tab, pannelli, scorciatoie | JSON fragment (file nostri, non tocchiamo `settings.json` se non per le impostazioni globali) |
| **2. Shell** | cmd.exe, PowerShell 7, Windows PowerShell 5.1 | Comandi rapidi, autocomplete, cronologia, prompt | PowerShell: script `init.ps1` generato. cmd: Clink + script Lua e macro generate |
| **3. Programmi** | I comandi che l'utente lancia | Output, help | Installazione e configurazione di tool esterni + il nostro comando `csp` |

**Conseguenza pratica:** CLI Superpower è un **configuratore** che scrive file nei tre strati. Non intercetta né modifica nulla a runtime, tranne quello che fa il comando `csp` quando l'utente lo invoca.

---

## 3. Principi di progettazione

Queste regole valgono per tutto il progetto e vanno riportate nel `CLAUDE.md` del repository.

1. **Non distruttivo.** L'app scrive solo in file e cartelle di sua proprietà. Dove è inevitabile toccare un file dell'utente (`$PROFILE`, `settings.json`), aggiunge il minimo indispensabile, delimitato da marcatori, con backup preventivo.
2. **Dichiarativo.** Esiste un'unica fonte di verità: la configurazione salvata dall'app. Tutti i file di sistema sono **generati** da essa.
3. **Idempotente.** "Applica" eseguito due volte produce lo stesso risultato. Mai aggiungere righe in coda a un file esistente.
4. **Reversibile.** Ogni modifica è tracciata in un manifest; la disinstallazione ripristina lo stato originale.
5. **Riusa invece di reinventare.** Autocomplete, prompt, navigazione e formattazione esistono già come tool maturi: noi li installiamo, li configuriamo e li uniamo in un'esperienza coerente.
6. **Una sola configurazione, scope per shell.** Gli elementi non si duplicano tra cmd e PowerShell: ogni elemento dichiara a quali shell si applica.

---

## 4. Funzionalità

### 4.1 Riepilogo e priorità

| # | Funzionalità | Strato | Release |
|---|---|---|---|
| F1 | Personalizzazione aspetto per profilo (cmd / PowerShell) con copia tra profili | Terminale | **v1** |
| F2 | Comandi rapidi con parametri e scope per shell | Shell | **v1** |
| F3 | Pacchetto team + preferenze personali (configurazione a strati) | Tutti | **v1** |
| F4 | Workspace di progetto (tab e pannelli predefiniti) | Terminale | **v1** |
| F5 | Prompt informativo (git, versioni, durata, esito) | Shell | **v1** |
| F6 | Utility "chi occupa la porta" | Programmi (`csp`) | **v1** |
| F7 | Gestione tool esterni (rilevamento, installazione con winget) | Programmi | **v1** |
| F8 | Backup, ripristino, disinstallazione pulita | Tutti | **v1** |
| F9 | Autocomplete predittivo e cronologia | Shell | v2 |
| F10 | Help con esempi (tldr) + help automatico dei comandi rapidi | Programmi | v2 |
| F11 | Navigazione rapida (zoxide) e ricerca fuzzy (fzf) | Shell | v2 |
| F12 | Terminale a scomparsa (quake mode) | Terminale | v2 |
| F13 | Preset per stack (.NET, Angular, Node, Git, IIS) | Shell | v2 |
| F14 | Notifica a fine comando lungo | Shell + `csp` | v2 |
| F15 | Output strutturato: `csp table` (tabelle, copia negli appunti) | Programmi | v2 |
| F16 | Output strutturato: `csp view` (vista interattiva, grafici) | Programmi | v3 |
| F17 | Interfaccia git nel terminale (lazygit) | Programmi | v3 |
| F18 | Supporto cmd completo via Clink | Shell | v2/v3 (vedi §4.2) |
| F19 | Assistente AI nel terminale | Programmi | Futuro |
| F20 | Variabili d'ambiente per progetto | Shell | Futuro |
| F21 | Sincronizzazione configurazione tra PC | Tutti | Futuro |

### 4.2 Shell supportate

| Shell | Supporto | Note |
|---|---|---|
| PowerShell 7 (`pwsh`) | **v1, primaria** | Ecosistema più ricco (PSReadLine, oggetti) |
| Windows PowerShell 5.1 | v1 | Stesso `init.ps1`, ma percorso `$PROFILE` diverso; alcune opzioni PSReadLine possono differire per versione del modulo |
| cmd.exe | v2 | Richiede **Clink** per autocomplete, cronologia, macro persistenti e prompt |

> **Perché non tutte subito:** supportare cmd e PowerShell dalla v1 raddoppia generatori e test. La v1 punta su PowerShell; il modello di configurazione prevede già lo scope per shell, così aggiungere cmd non richiede di rifare nulla.

---

### F1 — Personalizzazione aspetto per profilo

**Cosa fa.** Ogni shell (profilo di Windows Terminal) ha il suo aspetto: schema colori, font e dimensione, immagine di sfondo con opacità e allineamento, trasparenza/acrilico, stile del cursore, padding. Pulsante **"Copia aspetto su…"** per duplicare i valori da un profilo all'altro.

**Come si realizza.** Tramite **JSON fragment** di Windows Terminal: file JSON che un'applicazione deposita in
`%LOCALAPPDATA%\Microsoft\Windows Terminal\Fragments\CliSuperpower\`.
Un fragment può creare nuovi profili, modificare profili esistenti (identificati dal GUID nel campo `updates`) e aggiungere schemi colori. Da Windows Terminal 1.24 i fragment possono distribuire anche immagini (es. `backgroundImage`) dalla stessa cartella.

**Perché i fragment e non `settings.json`.**
- Non tocchiamo il file dell'utente: niente rischio di corromperlo o di sovrascrivere sue modifiche.
- Il percorso è lo stesso per tutte le varianti di Windows Terminal (Store, Preview, non pacchettizzato), mentre `settings.json` cambia posizione.
- Disinstallare = cancellare la nostra cartella.

**Limiti / punti di attenzione.**
- Le impostazioni **globali** (profilo predefinito, tema dell'app, scorciatoie) non sono coperte dai fragment: per quelle si modifica `settings.json` con backup e solo per le chiavi gestite (vedi §5.4).
- Se l'utente ha impostato a mano un valore nel suo `settings.json`, quel valore prevale sul fragment. L'app deve rilevarlo e avvisare ("questa impostazione è sovrascritta dalla tua configurazione manuale").
- I GUID dei profili predefiniti (cmd, PowerShell) vanno letti dalla configurazione esistente, non hardcodati.

**UI.** Anteprima live del terminale nel pannello (riquadro che simula colori, font e sfondo), selettore immagini, galleria di schemi colori.

---

### F2 — Comandi rapidi con parametri

**Cosa fa.** L'utente definisce scorciatoie per comandi frequenti:

| Scorciatoia | Comando | Parametri | Shell |
|---|---|---|---|
| `ndev` | `npm run dev` | — | Tutte |
| `mig` | `dotnet ef migrations add {nome}` | `nome` (obbligatorio) | Tutte |
| `gco` | `git checkout {branch}` | `branch` | Tutte |
| `iisr` | `iisreset` | — | Solo PowerShell |

Ogni comando ha: nome, comando con segnaposto `{param}`, descrizione, parametri (nome, descrizione, obbligatorio/opzionale, valore predefinito), shell di destinazione, cartella in cui eseguirlo (opzionale).

**Come si realizza.**
- **PowerShell:** una funzione per comando dentro `init.ps1`, con parametri tipizzati e help integrato (comment-based help), così `Get-Help mig` e `mig -?` funzionano.
- **cmd (con Clink):** macro `doskey` caricate da un file generato (`$1`, `$2`, `$*` per i parametri).

**Prefisso `/`.** Il desiderio iniziale è usare `/ndev`. Il prefisso deve essere **configurabile** e la sua fattibilità va **verificata in uno spike** per ciascuna shell (caratteri ammessi nei nomi di funzione e di macro). Fallback: nessun prefisso o prefisso alfanumerico (es. `,ndev` o `x-ndev`).

**Punti di attenzione.**
- Conflitti di nome con comandi esistenti (es. un comando rapido chiamato `ls`): l'app deve controllare e avvisare.
- Escaping: i segnaposto vanno sostituiti in modo sicuro per evitare che un valore con spazi o caratteri speciali rompa il comando.

---

### F3 — Pacchetto team + preferenze personali

**Cosa fa.** La configurazione è composta da **strati sovrapposti**:

```
Valori predefiniti dell'app
        ↓ sovrascritti da
Pacchetto team (pubblicato da un responsabile, versionato)
        ↓ sovrascritti da
Preferenze personali dell'utente
        ↓
Configurazione effettiva → "Applica"
```

- Il **pacchetto team** contiene comandi rapidi comuni, tema aziendale, workspace dei progetti, tool consigliati.
- L'utente può **disattivare** singoli elementi del team o **sovrascriverli**, ma non li cancella dal pacchetto.
- Quando esce una nuova versione del pacchetto team, l'utente riceve i nuovi elementi e le sue personalizzazioni restano.

**Come si distribuisce il pacchetto.** v1: file JSON importato a mano o da URL (es. raw di un repo git interno). v2: controllo automatico degli aggiornamenti dall'URL configurato.

**UI.** Ogni elemento mostra la sua origine (badge "Team" / "Personale") e se è stato modificato rispetto al team (con pulsante "Ripristina valore del team").

**Punti di attenzione.** Il pacchetto team è codice che finisce nella shell dei colleghi: va scaricato solo da fonti configurate esplicitamente e mostrato in anteprima prima di applicarlo.

---

### F4 — Workspace di progetto

**Cosa fa.** Un workspace descrive come aprire un progetto: tab, pannelli divisi, cartella e comando di ciascun pannello.

Esempio "eFM":
- Pannello sinistro: `ng serve` nella cartella del frontend
- Pannello destro: `dotnet watch` nella cartella del backend
- Seconda tab: shell libera nella root del repo, per git

Si avvia dal pannello (pulsante) o dal terminale (`ws efm`).

**Come si realizza.** Windows Terminal accetta da riga di comando la creazione di tab e pannelli (`wt new-tab ... ; split-pane ...`). L'app genera la riga di comando corretta e la espone come comando rapido e/o come profilo dedicato nel fragment.

**Punti di attenzione.** Il separatore `;` di `wt` va escapato quando il comando è lanciato da PowerShell; il generatore deve gestirlo per ogni shell.

---

### F5 — Prompt informativo

**Cosa fa.** Il prompt mostra: cartella corrente (abbreviata), branch git e stato (file modificati, commit da pushare), versione di Node/.NET se la cartella è un progetto, durata dell'ultimo comando se lunga, indicatore rosso se l'ultimo comando è fallito.

**Come si realizza.** Integrazione con **Oh My Posh** o **Starship** (da scegliere nello spike: entrambi supportano PowerShell e cmd/Clink). L'app genera il loro file di configurazione a partire da un editor semplificato (segmenti attivabili, colori, ordine) e installa un **Nerd Font** se l'utente sceglie un tema con icone.

---

### F6 — "Chi occupa la porta?"

**Cosa fa.** `csp port 4200` mostra quale processo ascolta sulla porta (nome, PID, percorso, avviato da quanto) e chiede se terminarlo.

**Come si realizza.** Nel comando `csp` (Rust), tramite le API di Windows per la tabella delle connessioni TCP. Funziona identico in ogni shell.

**Punti di attenzione.** Terminare un processo di sistema o di un altro utente richiede privilegi: messaggio chiaro, mai tentare l'elevazione automatica.

---

### F7 — Gestione tool esterni

**Cosa fa.** Elenco dei tool supportati con stato (installato / mancante / versione), installazione con un click, attivazione/disattivazione dell'integrazione.

| Tool | A cosa serve | Funzionalità |
|---|---|---|
| PowerShell 7 | Shell moderna | Base |
| Clink | Autocomplete, cronologia e macro per cmd | F2, F9, F18 |
| Oh My Posh / Starship | Prompt | F5 |
| Nerd Font | Icone nel terminale | F1, F5 |
| zoxide | Salto rapido tra cartelle | F11 |
| fzf (+ modulo PSFzf) | Ricerca fuzzy | F11 |
| tealdeer | Help con esempi (pagine tldr) | F10 |
| eza | Elenco file colorato e a colonne | F15 |
| bat | Visualizzazione file con evidenziazione | F15 |
| jq | Interrogazione JSON | F15 |
| lazygit | Interfaccia git testuale | F17 |

**Come si realizza.** Installazione tramite **winget** (presente su Windows 10/11 moderni). Gli identificativi winget di ogni tool vanno verificati e tenuti in un catalogo aggiornabile insieme all'app.

**Punti di attenzione.** winget può mancare o essere bloccato da policy aziendali: l'app deve rilevarlo e mostrare il link di installazione manuale.

---

### F8 — Backup, ripristino, disinstallazione

**Cosa fa.**
- Prima di ogni "Applica" salva una copia dei file utente che verranno toccati.
- Pagina "Cronologia" con le applicazioni precedenti e pulsante "Ripristina".
- "Disinstalla integrazione": rimuove fragment, script generati e righe aggiunte, riportando il sistema allo stato iniziale.

**Come si realizza.** Un **manifest** (§5.4) registra ogni file scritto o modificato con hash; i backup sono in una cartella dell'app con rotazione (es. ultimi 10).

---

### F9 — Autocomplete predittivo e cronologia (v2)

**Cosa fa.** Suggerimenti in grigio mentre si scrive, basati sulla cronologia (impara dai comandi più usati), vista a elenco dei suggerimenti, ricerca nella cronologia, cronologia condivisa tra sessioni, esclusione dalla cronologia dei comandi con dati sensibili.

**Come si realizza.** Non si costruisce un motore: si configura **PSReadLine** (PowerShell: sorgente delle predizioni, stile di visualizzazione, scorciatoie) e **Clink** (cmd: autosuggerimenti). Il pannello espone le opzioni con nomi comprensibili.

**Punto di attenzione.** La cronologia è salvata in chiaro su disco: l'opzione "non salvare comandi che contengono password/token" va attivata di default con un filtro configurabile.

---

### F10 — Help comprensibile (v2)

**Cosa fa.**
- `csp help <comando>`: mostra prima la pagina tldr (esempi pratici), con fallback all'help ufficiale.
- I comandi rapidi hanno help generato automaticamente da descrizione e parametri.
- `csp list`: elenco colorato di tutti i comandi rapidi disponibili, filtrabile.

---

### F11 — Navigazione e ricerca (v2)

- **zoxide**: `z efm` porta nella cartella più usata che contiene "efm".
- **fzf**: Ctrl+R per cercare nella cronologia scrivendo frammenti sparsi; Ctrl+T per cercare file.

L'app installa e aggancia i tool nell'`init.ps1` / negli script Clink.

---

### F12 — Terminale a scomparsa (v2)

Attiva la modalità "quake" di Windows Terminal (finestra che scende dall'alto con una scorciatoia globale) e permette di scegliere scorciatoia, profilo e dimensione. Richiede modifica controllata di `settings.json` (impostazione globale).

---

### F13 — Preset per stack (v2)

Pacchetti pronti di comandi rapidi selezionabili: **.NET** (build, test, migrazioni EF, watch), **Angular** (serve, build, generate), **Node/npm**, **Git** (alias comuni), **IIS** (reset, riavvio app pool). Sono pacchetti nello stesso formato del pacchetto team.

---

### F14 — Notifica a fine comando lungo (v2)

**Cosa fa.** Se un comando dura più di N secondi (configurabile), alla fine arriva una notifica di Windows con nome del comando, durata ed esito.

**Come si realizza.** In PowerShell il prompt generato legge durata ed esito dell'ultimo comando dalla cronologia della sessione e, oltre la soglia, chiama `csp notify`. Su cmd tramite gli eventi di Clink (v3).

---

### F15 — Output strutturato: `csp table` (v2)

**Cosa fa.** Formattatore da usare in pipe:

```powershell
Get-Process | Select-Object Name, Id, CPU | ConvertTo-Json | csp table
Get-Content dati.csv | csp table --sort CPU --desc
csp table dati.json --copy markdown
```

- Input: JSON, CSV, testo a colonne (rilevamento automatico o `--format`).
- Output: tabella allineata con colori (intestazioni, numeri, valori nulli), larghezza adattata al terminale.
- `--copy markdown|csv|tsv`: copia negli appunti in un formato che si incolla bene in Excel, Jira, Teams.
- Opzioni base: ordinamento, filtro colonne, limite righe.

**Aiuto per PowerShell.** `init.ps1` può definire una funzione `ct` che fa automaticamente `ConvertTo-Json | csp table`, così basta `Get-Process | ct`.

### F16 — Output strutturato: `csp view` (v3)

Vista interattiva a tutto terminale (TUI): scorrimento, filtro mentre si scrive, ordinamento per colonna, selezione e copia di righe, grafici semplici (barre, linee) disegnati con caratteri Unicode sulle colonne numeriche.

---

### F19–F21 — Futuro

- **Assistente AI** ("come faccio a…", "spiegami questo errore"): richiede gestione chiavi API, costi e attenzione a non inviare dati sensibili. Da valutare dopo la v2.
- **Variabili d'ambiente per progetto**: caricate entrando in una cartella; tocca segreti, va progettata con cura.
- **Sincronizzazione tra PC**: in gran parte coperta da export/import e dal pacchetto team.

---

## 5. Architettura

### 5.1 Flusso

```
┌──────────────────────────────┐
│  Pannello (Angular in Tauri) │  l'utente modifica le opzioni
└──────────────┬───────────────┘
               │ comandi Tauri (IPC)
┌──────────────▼───────────────┐
│  core (Rust)                 │
│  1. carica strati config     │  default → team → personale
│  2. migra se versione vecchia│
│  3. calcola config effettiva │
│  4. generatori (funzioni     │  config → contenuto dei file
│     pure, senza I/O)         │
│  5. applicatore              │  backup → scrittura → manifest
└──────────────┬───────────────┘
               │
   ┌───────────┼───────────────┬──────────────────┐
   ▼           ▼               ▼                  ▼
Fragment WT  init.ps1     script Clink      config prompt
(profili,    (funzioni,   (macro, lua)      (Oh My Posh /
 schemi,      PSReadLine,                    Starship)
 immagini)    hook)
```

La separazione tra **generatori** (puri: ricevono la config, restituiscono testo) e **applicatore** (l'unico che tocca il disco) è il punto più importante dell'architettura: i generatori si testano senza Windows, senza file e senza mock.

### 5.2 File gestiti dall'app

| File / cartella | Proprietario | Contenuto |
|---|---|---|
| `%APPDATA%\CliSuperpower\config.personal.json` | App | Preferenze personali |
| `%APPDATA%\CliSuperpower\team\*.json` | App | Pacchetti team e preset importati |
| `%LOCALAPPDATA%\CliSuperpower\generated\` | App | `init.ps1`, macro, config prompt generati |
| `%LOCALAPPDATA%\CliSuperpower\backups\` | App | Backup con rotazione |
| `%LOCALAPPDATA%\CliSuperpower\manifest.json` | App | Registro di ciò che è stato scritto |
| `%LOCALAPPDATA%\Microsoft\Windows Terminal\Fragments\CliSuperpower\` | App | Fragment JSON e immagini |
| `$PROFILE` di pwsh e di powershell.exe | **Utente** | Solo un blocco delimitato che carica `init.ps1` |
| `settings.json` di Windows Terminal | **Utente** | Solo le chiavi globali gestite (es. quake mode), con backup |
| Cartella script di Clink | Condivisa | Un nostro file Lua dedicato |

Il blocco inserito in `$PROFILE`:

```powershell
# >>> CLI Superpower >>> (non modificare: gestito automaticamente)
if (Test-Path "$env:LOCALAPPDATA\CliSuperpower\generated\init.ps1") {
    . "$env:LOCALAPPDATA\CliSuperpower\generated\init.ps1"
}
# <<< CLI Superpower <<<
```

### 5.3 Modello di configurazione (concettuale)

> Lo schema dettagliato di `config.json` è il **prossimo passo** del progetto. Qui solo le sezioni previste.

- `schemaVersion` — numero intero, guida le migrazioni
- `appearance` — per profilo (`pwsh`, `powershell`, `cmd`): colori, font, sfondo, trasparenza
- `globalTerminal` — impostazioni globali di Windows Terminal gestite (profilo predefinito, quake mode)
- `commands[]` — comandi rapidi: nome, template, parametri, shell, cartella, origine (team/personale)
- `workspaces[]` — tab e pannelli
- `prompt` — motore e segmenti
- `shellBehavior` — opzioni autocomplete/cronologia per shell
- `tools` — tool attivati e loro opzioni
- `team` — sorgenti dei pacchetti team e versioni installate
- `overrides` — elementi del team disattivati o modificati dall'utente

### 5.4 Manifest e modifiche ai file dell'utente

Il manifest registra per ogni file: percorso, tipo di modifica (creato / blocco inserito / chiavi modificate), hash dopo la scrittura, riferimento al backup.

Prima di riscrivere un file, l'applicatore confronta l'hash attuale con quello del manifest: se diverso, **l'utente ha modificato il file a mano** → l'app non sovrascrive e mostra le differenze chiedendo cosa fare.

Per `settings.json`: lettura che tollera commenti e virgole finali (Windows Terminal li accetta), modifica solo delle chiavi gestite, scrittura preservando il resto. Va scelta una libreria JSON che mantenga commenti e formattazione, o in alternativa una modifica testuale mirata e testata.

---

## 6. Aggiornamenti

Tre cose diverse si aggiornano, con meccanismi diversi.

| Cosa | Meccanismo |
|---|---|
| **L'app** (nuovo eseguibile) | Plugin updater di Tauri: all'avvio controlla un manifest di release (`latest.json` su GitHub Releases), scarica, verifica la firma, installa. |
| **La configurazione** (struttura dati cambiata) | `schemaVersion` + una funzione di migrazione per ogni salto (1→2, 2→3…), eseguite in sequenza al caricamento. Ogni migrazione ha i suoi test. Backup della config prima di migrare. |
| **I file generati** | Dopo un aggiornamento dell'app i generatori possono produrre output diverso: l'app rilancia **Applica** automaticamente (con riepilogo delle differenze). |
| **Il pacchetto team** | Versionato; l'app controlla l'URL configurato e propone l'aggiornamento con anteprima delle novità. |

**Regola per Claude Code:** ogni modifica alla struttura della configurazione richiede incremento di `schemaVersion`, migrazione e test della migrazione. Senza eccezioni.

---

## 7. Stack tecnologico

| Ambito | Scelta | Motivazione |
|---|---|---|
| Framework desktop | **Tauri 2** | Eseguibile piccolo, backend Rust, updater e installer integrati |
| Logica | **Rust** (workspace Cargo) | Il compilatore blocca molti errori: ciclo compila-correggi efficace per Claude Code |
| Frontend | **Angular** (standalone components, signals) | Linguaggio già conosciuto → il codice generato si può davvero rivedere |
| UI kit | Da scegliere (es. Angular Material) | Componenti pronti per form, toggle, liste |
| Tipi condivisi Rust ↔ TypeScript | **tauri-specta** (o ts-rs) | La config si definisce una volta in Rust, le interfacce TS si generano: niente disallineamenti |
| Serializzazione | serde, serde_json | Standard Rust |
| Schema config | schemars (JSON Schema dalla struct) | Validazione e documentazione automatica della config |
| Comando `csp` | clap (argomenti), comfy-table (tabelle), ratatui (TUI, v3), arboard (appunti) | Librerie mature |
| API Windows | crate `windows` | Porte TCP, notifiche, percorsi noti |
| Test Rust | `cargo test` + **insta** (snapshot test) | I file generati si confrontano con l'output atteso; ogni differenza è visibile in revisione |
| Test frontend | Test unitari Angular + mock delle chiamate Tauri | Il pannello si sviluppa e prova nel browser senza backend |
| Installer | Bundle NSIS di Tauri, installazione per utente | Nessun privilegio di amministratore richiesto |
| CI/CD | GitHub Actions su runner Windows + **tauri-action** | Build, firma updater, release e `latest.json` automatici |

### 7.1 Struttura del repository

```
CLI-superpower/
├─ CLAUDE.md                 regole per Claude Code
├─ docs/
│  ├─ PROGETTO.md            questo documento
│  └─ config-schema.md       schema della configurazione (prossimo passo)
├─ crates/
│  ├─ core/                  config, strati, migrazioni, generatori, applicatore
│  │  └─ tests/snapshots/    output attesi dei generatori
│  └─ csp/                   comando da terminale (table, port, notify, help, view)
├─ app/
│  ├─ src-tauri/             backend Tauri: espone i comandi di core alla UI
│  └─ src/                   frontend Angular
├─ presets/                  preset di stack e pacchetto team di esempio
└─ .github/workflows/        CI e release
```

`core` non dipende da Tauri: la stessa logica serve l'app grafica e il comando `csp` (es. `csp apply` per applicare la config senza aprire il pannello, utile anche per test e automazioni).

---

## 8. Sviluppo con Claude Code

### 8.1 Il ciclo di lavoro

Seguendo l'approccio **prima il frontend, poi il backend**:

1. **Frontend con dati simulati.** Si costruisce la schermata in Angular con le chiamate a Tauri simulate (mock). Si prova in `ng serve` nel browser, si valida l'esperienza.
2. **Contratto.** Si definiscono in Rust le strutture dati che la schermata usa; i tipi TypeScript vengono rigenerati e sostituiscono quelli provvisori.
3. **Backend.** Si implementano generatori e comandi in `core`, con snapshot test.
4. **Integrazione.** Si toglie il mock e si prova l'app vera su Windows.

### 8.2 Contenuto del `CLAUDE.md` (bozza)

- I generatori sono funzioni pure: niente I/O, niente variabili d'ambiente, niente orologio. Ogni generatore ha snapshot test.
- Solo il modulo applicatore scrive su disco. Ogni scrittura passa da backup e manifest.
- Mai scrivere in `$PROFILE` o `settings.json` fuori dai blocchi/chiavi gestiti.
- Ogni modifica alle struct di configurazione: incremento `schemaVersion` + migrazione + test.
- Dopo modifiche alle struct Rust esposte alla UI: rigenerare i tipi TypeScript.
- I percorsi di sistema si risolvono a runtime (mai hardcodati): `$PROFILE` si chiede alla shell stessa, le cartelle note tramite API.
- Prima di dichiarare un task finito: `cargo test`, `cargo clippy`, test Angular, build.

### 8.3 Testing

| Livello | Cosa | Dove gira |
|---|---|---|
| Unità + snapshot | Strati config, migrazioni, generatori | Qualsiasi OS (anche Linux in CI) |
| Integrazione applicatore | Scrittura in una cartella temporanea che simula `%APPDATA%` / `$PROFILE` | Runner Windows |
| Frontend | Componenti e servizi con mock Tauri | Browser / CI |
| Smoke test manuale | Checklist su un PC Windows pulito (o VM) prima di ogni release | Manuale |

---

## 9. Distribuzione

- **Release:** un tag git (`v1.2.0`) avvia la pipeline che compila, firma l'aggiornamento, pubblica su GitHub Releases e aggiorna `latest.json`.
- **Chiave dell'updater:** coppia di chiavi generata una volta; la privata va nei segreti di GitHub. **Se si perde, gli utenti non possono più ricevere aggiornamenti automatici**: va custodita con un backup sicuro.
- **Firma del codice Windows:** senza un certificato di firma del codice, Windows SmartScreen mostra un avviso al primo avvio. Accettabile per i primi colleghi; da valutare un certificato prima di una distribuzione più ampia.
- **Futuro:** pubblicazione su winget per installazione da riga di comando.

---

## 10. Roadmap

### Fase 0 — Fondamenta e spike
| Area | Attività |
|---|---|
| Setup | Repo, workspace Cargo, app Tauri + Angular vuota, CI di base, `CLAUDE.md` |
| Spike | Verificare: prefisso `/` nei nomi comando (pwsh, cmd/Clink); fragment con `updates` su profili predefiniti; lettura/scrittura `settings.json` preservando i commenti; risoluzione `$PROFILE` con Documenti su OneDrive; Oh My Posh vs Starship |
| Config | Definizione dello schema di configurazione v1 (`docs/config-schema.md`) |

### Fase 1 — MVP (v1)
| Area | Frontend | Backend |
|---|---|---|
| Aspetto (F1) | Editor per profilo con anteprima, "Copia su…" | Generatore fragment, immagini nella cartella fragment |
| Comandi rapidi (F2) | Lista, editor con parametri e scope shell | Generatore `init.ps1` (funzioni + help) |
| Strati (F3) | Badge Team/Personale, ripristino valore team | Caricamento e merge strati, import pacchetto |
| Workspace (F4) | Editor tab/pannelli | Generatore riga di comando `wt` |
| Prompt (F5) | Editor segmenti | Generatore config prompt |
| Tool (F7) | Pagina tool con stato e installa | Rilevamento, winget |
| Porta (F6) | — | `csp port` |
| Sicurezza (F8) | Pagina cronologia, disinstalla | Applicatore, backup, manifest |
| Aggiornamenti | Notifica nuova versione | Updater, migrazioni |

### Fase 2 (v2)
F9 autocomplete, F10 help, F11 navigazione, F12 quake mode, F13 preset stack, F14 notifiche, F15 `csp table`, supporto cmd con Clink, aggiornamento automatico del pacchetto team.

### Fase 3 (v3)
F16 `csp view`, F17 lazygit, notifiche su cmd, valutazione funzionalità future.

---

## 11. Rischi e punti aperti

| Rischio | Impatto | Mitigazione |
|---|---|---|
| Policy di esecuzione PowerShell restrittiva (comune su PC aziendali o Windows PowerShell 5.1) | Il profilo e `init.ps1` non vengono eseguiti | Rilevare la policy, spiegare all'utente, mai cambiarla in automatico |
| Cartella Documenti reindirizzata su OneDrive | `$PROFILE` in un percorso inatteso | Chiedere il percorso a `pwsh`/`powershell` stessi |
| Modifiche manuali dell'utente ai file gestiti | Sovrascrittura del lavoro dell'utente | Controllo hash dal manifest, conflitto mostrato all'utente |
| Cambiamenti futuri di Windows Terminal | Fragment o chiavi non più validi | Preferire i fragment (API documentata), test sulla versione corrente, catalogo aggiornabile |
| winget assente o bloccato | Installazione tool impossibile | Link manuali, rilevamento tool già installati altrove |
| Pacchetto team malevolo o errato | Codice eseguito nelle shell dei colleghi | Sorgenti esplicite, anteprima obbligatoria, diff prima di applicare |
| Perdita chiave updater | Aggiornamenti automatici interrotti | Backup sicuro della chiave |
| Codice generato da AI non rivisto | Bug in un'app che tocca file di sistema | Generatori puri + snapshot test + revisione del frontend in Angular |
| Cronologia comandi in chiaro | Esposizione di segreti digitati | Filtro dei comandi sensibili attivo di default |

**Punti aperti da decidere:** prefisso dei comandi rapidi; Oh My Posh o Starship; UI kit Angular; formato e hosting del pacchetto team; nome definitivo del comando (`csp`).

---

## 12. Glossario

| Termine | Significato |
|---|---|
| **Windows Terminal** | L'app Microsoft che mostra le shell in tab e pannelli; gestisce l'aspetto |
| **Shell** | Il programma che interpreta i comandi (cmd, PowerShell) |
| **Profilo (Windows Terminal)** | Una voce del terminale che avvia una shell con un certo aspetto |
| **`$PROFILE`** | Script che PowerShell esegue a ogni avvio; diverso tra PowerShell 7 e 5.1 |
| **JSON fragment** | File JSON che un'app deposita in una cartella nota per aggiungere o modificare profili e schemi di Windows Terminal senza toccare `settings.json` |
| **PSReadLine** | Modulo di PowerShell che gestisce la riga di input: autocomplete, cronologia, scorciatoie |
| **Clink** | Estensione per cmd.exe che aggiunge autocomplete, cronologia e script Lua |
| **doskey** | Meccanismo di cmd per definire macro (alias) |
| **Nerd Font** | Font con icone aggiuntive usate dai prompt moderni |
| **TUI** | Interfaccia testuale interattiva dentro il terminale |
| **winget** | Gestore pacchetti ufficiale di Windows |
| **IPC (Tauri)** | Canale con cui il frontend chiama le funzioni Rust del backend |
| **Snapshot test** | Test che salva l'output atteso e segnala qualsiasi differenza futura |
| **Idempotente** | Eseguito più volte, dà sempre lo stesso risultato |
| **Migrazione** | Funzione che converte una configurazione dal vecchio al nuovo formato |
| **Manifest** | Registro dei file scritti dall'app, usato per controlli, ripristino e disinstallazione |
| **Spike** | Breve esperimento tecnico per verificare la fattibilità prima di sviluppare |
