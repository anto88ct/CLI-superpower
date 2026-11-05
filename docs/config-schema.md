# Configurazione v1 di CLI Superpower

`schemaVersion: 6` (versione corrente; le versioni da 1 a 5 sono migrate automaticamente) identifica il formato dei file JSON. È indipendente dalla versione dell'app e dalla revisione di un pacchetto team. Questo contratto copre il modello persistente e la risoluzione degli strati. Il core passa ai generatori la configurazione effettiva già validata e riceve dal rilevamento di sistema i GUID dei profili, i percorsi e le capacità disponibili.

## File e tipi

| Campo radice | Personale | Pacchetto team | Tipo |
|---|---|---|---|
| `schemaVersion` | obbligatorio | obbligatorio | intero `6` (accettati anche da `1` a `5`, migrati alla lettura) |
| `id`, `version` | — | obbligatori | ID stabile e revisione intera positiva |
| `appearance` | facoltativo | facoltativo | mappa per profilo |
| `globalTerminal` | facoltativo | — | oggetto delle preferenze globali |
| `commandPrefix` | facoltativo | facoltativo | prefisso dei comandi rapidi |
| `commands` | facoltativo | facoltativo | array di comandi |
| `shellBehavior` | facoltativo | facoltativo | mappa per shell di autocomplete e cronologia |
| `banner` | facoltativo | facoltativo | banner di avvio delle shell |
| `workspaces` | facoltativo | facoltativo | array di workspace |
| `prompt` | facoltativo | facoltativo | oggetto del prompt |
| `tools` | facoltativo | facoltativo | mappa per ID del catalogo |
| `team` | facoltativo | — | riferimento al pacchetto attivo |
| `overrides` | facoltativo | — | mappa per ID pacchetto |

- Il file personale è `%APPDATA%\CliSuperpower\config.personal.json`. Un file assente equivale a `{"schemaVersion":1}`. I pacchetti installati sono in `%APPDATA%\CliSuperpower\team\<id>-<version>.json`. In v1 c'è al massimo un pacchetto attivo.
- I file sono JSON UTF-8 con nomi di proprietà in `camelCase`. Il lettore rifiuta proprietà sconosciute, chiavi duplicate, tipi errati e `null`. Un campo facoltativo omesso eredita dove si applica il merge. `false`, `0`, `""` e `[]` sono valori espliciti quando ammessi per il campo.
- Gli ID di pacchetto, comando e workspace seguono `^[a-z][a-z0-9-]{0,63}$`. Sono univoci nella rispettiva collezione e restano stabili attraverso rinomina e aggiornamenti. Gli interi dei contatori rientrano nell'intervallo `1..2147483647`.
- I valori di profilo sono `pwsh`, `powershell` e `cmd`. Identificano le tre famiglie di profili di Windows Terminal. In v1 `cmd` può ricevere aspetto ed essere aperto in un workspace. L'integrazione shell con cmd e i campi di `shellBehavior` entrano con le funzionalità v2.
- La v1 accetta percorsi Windows assoluti di unità o UNC. Il file contiene il percorso letterale JSON, senza espansione di variabili. L'anteprima verifica la disponibilità delle risorse locali, anche quando il pacchetto arriva da un'altra macchina.

## Sezioni condivise

### Aspetto e terminale globale

`appearance` contiene un oggetto facoltativo per ogni profilo. Le singole proprietà sono tutte facoltative.

| Campo del profilo | Tipo e significato |
|---|---|
| `colorScheme` | nome non vuoto di uno schema disponibile nel catalogo o in Windows Terminal |
| `font.face`, `font.size` | nome non vuoto del font e dimensione numerica positiva in punti |
| `backgroundImage` | percorso locale assoluto dell'immagine |
| `backgroundImageOpacity` | numero fra `0` e `1` inclusi |
| `backgroundImageAlignment` | `center`, `left`, `top`, `right`, `bottom`, `topLeft`, `topRight`, `bottomLeft` o `bottomRight` |
| `opacity` | intero fra `0` e `100` inclusi |
| `useAcrylic` | booleano |
| `cursorShape` | `bar`, `vintage`, `underscore`, `filledBox`, `emptyBox` o `doubleUnderscore` |
| `padding` | oggetto con `left`, `top`, `right`, `bottom` facoltativi, interi fra `0` e `2147483647` |

`globalTerminal.defaultProfile` è facoltativo e accetta un valore di profilo. Se omesso, CLI Superpower lascia al terminale la scelta del profilo predefinito. La possibilità di scrivere la chiave effettiva dipende dallo spike su `settings.json`. Quake mode è una funzionalità v2.

### Comandi rapidi

- `shellBehavior` è una mappa le cui chiavi sono `pwsh`, `powershell` e `cmd`. Una shell assente non viene toccata. Ogni voce può contenere `predictions` (`history` o `off`), `predictionView` (`inline` o `list`), `acceptKey` (`right` o `tab`, default `right`: con `tab` il tasto accetta il suggerimento in linea e, se non ce n'è uno, esegue il completamento che aveva prima), `hideSensitive` (booleano, default `true`) e `historyExclude` (fino a 50 frammenti di testo, al massimo 100 caratteri, senza caratteri di controllo). Con `hideSensitive` le righe che contengono parole come `password`, `token` o `secret`, e i frammenti di `historyExclude`, non vengono scritte nella cronologia su disco; il confronto ignora maiuscole e minuscole. In PowerShell il core configura PSReadLine solo se la versione lo consente (predizioni dalla 2.1, vista a elenco dalla 2.2, filtro della cronologia dalla 2.0) e conserva un filtro già impostato dall'utente. In CMD genera uno script Clink che imposta `autosuggest.inline` (vista `inline`, suggerimento in grigio dopo il cursore) o `suggestionlist.default` (vista `list`, elenco già aperto) con `settings.set`, valido per la sessione, senza modificare il file delle impostazioni di Clink, e `clink.onhistory` per il filtro. Le voci si fondono per chiave con il pacchetto team; `historyExclude` sostituisce l'elenco del livello inferiore.
- `banner` accetta `enabled` (booleano, default `true`). Con il banner attivo il core genera, per PowerShell 7 e Windows PowerShell, un frammento nello script `init` che stampa il logo una volta all'apertura di una sessione interattiva: salta le esecuzioni con `-Command`, `-File` o `-EncodedCommand`, l'output reindirizzato e le finestre più strette di 107 colonne. In CMD il banner entra nello script Clink solo se Clink serve già per comandi o autocomplete, perché il banner da solo non giustifica la modifica dell'avvio automatico di CMD.
- `commandPrefix` accetta `""`, `"/"` o `"x-"`. Il valore iniziale è `""`. Lo spike conferma `/` sia in PowerShell 7 sia in Windows PowerShell 5.1. Prima di generare le funzioni il core verifica che il prefisso sia supportato in ogni shell di destinazione e segnala eventuali collisioni con comandi esistenti. Il nome pubblico è `commandPrefix` concatenato a `name`.
- Un comando richiede `id`, `name`, `shells` e una tra `template` e `changeDirectory`. `name` segue la stessa sintassi degli ID. `shells` è un array non vuoto di valori distinti fra `pwsh`, `powershell` e `cmd`. In CMD con Clink sono supportate scorciatoie con argomenti fissi. La coppia shell e nome pubblico deve essere univoca dopo il merge, senza distinzione tra maiuscole e minuscole. `changeDirectory` (percorso assoluto) definisce una scorciatoia che sposta la shell corrente in quella cartella: in PowerShell genera `Set-Location -LiteralPath`, in CMD con Clink genera `cd /d`. Non si combina con `template`, `parameters` né `workingDirectory`. `description` (testo), `workingDirectory` (percorso assoluto) e `parameters` (array, default `[]`) sono facoltativi.
- `template` richiede `program` (nome di eseguibile o percorso assoluto non vuoto) e `args` (array ordinato di stringhe). Una voce è un argomento letterale oppure un riferimento a un parametro nella forma `{nome}`. In una voce `{{` e `}}` rappresentano parentesi graffe letterali. I segnaposto occupano un argomento intero. Il generatore passa ciascun valore come dato di un argomento, con quoting per la shell e senza reinterpretarlo come sorgente. Se un valore non è rappresentabile senza alterazioni in quella shell, l'invocazione fallisce con un errore.
- Ogni parametro richiede `name` univoco (`^[a-z][a-z0-9_]{0,63}$`), `type` (`string` o `integer`) e `required` (booleano). `description` e `default` sono facoltativi. `default` è ammesso solo per un parametro opzionale e ha il tipo dichiarato. `integer` è un intero con segno a 32 bit. Ogni riferimento deve corrispondere a un parametro dichiarato e ogni parametro deve essere usato. Se un parametro opzionale viene omesso e manca un default, l'argomento corrispondente viene saltato. Un parametro obbligatorio mancante blocca l'invocazione.
- I parametri sono posizionali nell'ordine di `parameters` e sono esposti come parametri nominati nelle funzioni PowerShell. Quelli obbligatori precedono quelli opzionali. Un `workingDirectory` specificato vale per la durata del comando, anche in caso di errore.

### Workspace

- Un workspace richiede `id`, `name` (testo non vuoto) e `tabs` (array ordinato non vuoto). Il comando di apertura `ws` usa l'`id`. Ogni tab richiede `pane` e può dichiarare `title` (testo) e `splits` (array ordinato, default `[]`).
- Un `pane` richiede `profile` e può dichiarare `workingDirectory` (percorso assoluto) e `startupCommand` (testo non vuoto in sintassi della shell scelta). In assenza di `startupCommand` si apre una shell interattiva. Lo script dichiarato è codice da includere nell'anteprima. Il generatore consegna il comando alla shell di destinazione e gestisce separatamente la sintassi di lancio di Windows Terminal.
- Ogni elemento di `splits` richiede `direction` (`right` o `down`) e `pane`. Divide l'ultimo pannello creato nella stessa tab e sposta il focus sul nuovo pannello. L'array descrive l'ordine di apertura.

### Prompt e strumenti

Il prompt informativo usa Oh My Posh e vale per le shell che lo supportano: PowerShell 7 e Windows PowerShell tramite lo script `init`, il Prompt dei comandi tramite lo script Clink, che esegue `oh-my-posh init cmd` con lo stesso file di configurazione. Se il prompt è attivo, l'app abilita Clink per CMD anche senza comandi o autocomplete. Se Oh My Posh o il file di configurazione mancano, ogni shell mantiene il proprio prompt. Il segmento `directory` accetta `pathStyle`: `short` (default, `~\..\Desktop`), `home` (`~\OneDrive\Desktop`), `full` (percorso intero senza tilde, `C:\Users\nome\OneDrive\Desktop`) o `folder` (solo `Desktop`).

- `prompt.enabled` è un booleano, con valore iniziale `false`. `prompt.segments` è un array ordinato che sostituisce la sequenza ereditata. Ogni elemento richiede un `id` distinto fra `directory`, `git`, `node`, `dotnet`, `duration`, `status`. Un segmento incluso è attivo e può specificare `color` nel formato `#RRGGBB`. Solo `duration` accetta `thresholdMs`, intero non negativo a 32 bit con default `2000`. Con `enabled: true` serve almeno un segmento. La v1 applica queste preferenze a PowerShell. Il motore predefinito sarà scelto dopo lo spike e il suo formato resta nei file generati.
- Le chiavi di `tools` sono gli ID del catalogo dell'app. In v1 il modello usa `pwsh`, `clink`, `oh-my-posh`, `starship`, `nerd-font`, `zoxide`, `fzf`, `tealdeer`, `eza`, `bat`, `jq` e `lazygit`. Ogni voce dichiara `enabled` (booleano). Il campo indica una preferenza di integrazione e ha effetto per i tool e le funzioni disponibili nella release. Stato, versione rilevata e identificativo winget appartengono al rilevamento e al catalogo. Il valore `true` non avvia un'installazione automatica.

## Team, merge e aggiornamenti

`team` richiede `packageId` e `installedVersion`, che devono corrispondere a `id` e `version` del pacchetto locale. `sourceUrl` è facoltativo e indica l'URL HTTPS configurato dall'utente per un futuro aggiornamento. L'origine scelta per l'importazione è un dato locale. In v1 l'importazione è avviata dall'utente e la nuova revisione diventa attiva solo dopo aver esaminato e approvato l'anteprima. Il core salva prima il file della revisione e poi sostituisce atomicamente il riferimento nel file personale. La coppia `id` e `version` identifica contenuto immutabile. Reimportare la stessa revisione è consentito se il contenuto coincide. Se il pacchetto selezionato manca o la revisione non corrisponde, il core segnala il problema e blocca Applica.

1. Si parte dai valori predefiniti integrati, poi si sovrappongono il pacchetto attivo e le preferenze personali. I default v1 lasciano l'aspetto e il profilo predefinito al sistema, impostano `commandPrefix` a `""`, `commands` e `workspaces` a `[]`, `prompt` a `{"enabled":false,"segments":[]}` e ogni tool a `enabled: false`.
2. Gli oggetti condivisi si fondono ricorsivamente per chiave, mentre i valori scalari e `prompt.segments` sono sostituiti dal valore del livello superiore. I campi omessi ereditano. Ripristinare un'impostazione del team equivale a rimuovere il campo personale. Un oggetto vuoto non azzera le preferenze ereditate.
3. `commands` e `workspaces` si risolvono per ID. Le voci personali nuove si aggiungono in coda nell'ordine del file. L'override di una voce team mantiene la sua posizione. Un array vuoto in uno strato equivale a zero aggiunte. Se un ID personale nuovo coincide con un ID team, il core mostra un conflitto da risolvere prima di attivare la configurazione.
4. `overrides` è una mappa per ID pacchetto. Ogni valore può contenere `commands` e `workspaces`, mappe per ID di elemento. Ogni override è `{"action":"disable"}` oppure `{"action":"replace","value":{...}}`. `replace` contiene l'intero comando o workspace e il suo `id` deve coincidere con la chiave. Si applicano solo gli override del pacchetto attivo. La UI ripristina una voce team rimuovendo l'override. Gli override di ID spariti dal pacchetto restano nel file, sono ignorati finché l'ID è assente e sono mostrati nell'anteprima. Gli override `replace` conservano la definizione personale completa anche dopo un aggiornamento del team.
5. Origine, stato dell'override e capacità rilevate sono metadati calcolati, disponibili alla UI e al riepilogo di Applica. Il core convalida le collisioni, i riferimenti e la disponibilità delle risorse prima della generazione. L'anteprima include i comandi eseguibili, gli script dei workspace e le differenze rispetto alla versione attiva. Un import respinto o una validazione fallita lascia attivo lo stato precedente.

Il core legge `schemaVersion` prima di interpretare le altre proprietà. Una versione assente, invalida o più recente blocca il caricamento e preserva il file. La versione 2 aggiunge `changeDirectory` ai comandi e la 3 aggiunge `shellBehavior`, la 4 aggiunge `banner`, la 5 aggiunge `acceptKey` e la 6 aggiunge `pathStyle`; i documenti della versione precedente sono già validi e ogni salto di migrazione cambia solo `schemaVersion`, in memoria a ogni lettura, mentre il file è riscritto alla prima operazione di salvataggio. Quando esistono più versioni, i documenti supportati vengono migrati separatamente, con backup prima della riscrittura, una funzione per salto e convalida del risultato. La sostituzione del file deve avvenire in modo atomico, dopo che la migrazione è riuscita. Ogni cambio della struttura richiede incremento di `schemaVersion`, migrazione e test.

## Esempio di pacchetto team

```json
{
  "schemaVersion": 1,
  "id": "sviluppo",
  "version": 3,
  "appearance": {
    "pwsh": { "font": { "face": "Cascadia Mono", "size": 12 } }
  },
  "commands": [
    {
      "id": "mig",
      "name": "mig",
      "shells": ["pwsh", "powershell"],
      "description": "Crea una migrazione EF",
      "template": { "program": "dotnet", "args": ["ef", "migrations", "add", "{nome}"] },
      "parameters": [{ "name": "nome", "type": "string", "required": true }]
    }
  ],
  "workspaces": [
    {
      "id": "efm",
      "name": "eFM",
      "tabs": [
        {
          "title": "Sviluppo",
          "pane": { "profile": "pwsh", "workingDirectory": "C:\\src\\efm\\frontend", "startupCommand": "ng serve" },
          "splits": [
            {
              "direction": "right",
              "pane": { "profile": "pwsh", "workingDirectory": "C:\\src\\efm\\backend", "startupCommand": "dotnet watch" }
            }
          ]
        },
        { "title": "Git", "pane": { "profile": "pwsh", "workingDirectory": "C:\\src\\efm" } }
      ]
    }
  ],
  "prompt": {
    "enabled": true,
    "segments": [{ "id": "directory" }, { "id": "git", "color": "#7AA2F7" }]
  },
  "tools": { "pwsh": { "enabled": true } }
}
```

## Esempio di preferenze personali

```json
{
  "schemaVersion": 1,
  "team": { "packageId": "sviluppo", "installedVersion": 3 },
  "appearance": { "pwsh": { "font": { "size": 14 }, "opacity": 90 } },
  "commands": [
    {
      "id": "node-version",
      "name": "node-version",
      "shells": ["pwsh"],
      "template": { "program": "node", "args": ["--version"] }
    }
  ],
  "overrides": { "sviluppo": { "commands": { "mig": { "action": "disable" } } } }
}
```

Il profilo `pwsh` eredita `Cascadia Mono`, usa dimensione `14` e opacità `90`. `node-version` è disponibile in `pwsh`, `mig` è disattivato e il workspace `efm` resta disponibile. Rimuovendo l'override di `mig` si ripristina il comando team.

## Confini con gli spike

| Spike | Confine del contratto |
|---|---|
| Prefisso `/` | Disponibile in entrambe le PowerShell in v1. Il supporto Clink va verificato prima di estendere lo scope a cmd. |
| Fragment con `updates` | Il file usa i tre ID logici. La risoluzione del GUID e la strategia di generazione restano nel backend. |
| Scrittura di `settings.json` | `defaultProfile` è l'intento dell'utente. L'applicatore abilita la modifica dopo aver verificato la conservazione dei contenuti utente. |
| `$PROFILE` e OneDrive | Il percorso del profilo è rilevato interrogando ogni shell. |
| Oh My Posh o Starship | Il core conserva segmenti e ordine. Le capacità del motore scelto saranno esposte alla UI. |

Le scelte di questa fase sono il formato JSON stretto, un solo pacchetto attivo, la sostituzione integrale degli override di comando e workspace e i template ad argomenti. In implementazione, i tipi Rust e lo schema generato saranno allineati a questo contratto.
