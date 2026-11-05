# Spike di fase 0

Verifiche del 5 novembre 2026 su Windows build 26300.9818. Le shell sono partite come processi figli con i profili disabilitati e i binari portabili e le fixture sono rimasti in una cartella temporanea.

## Prefisso dei comandi rapidi

| Ambiente | Prova | Risultato |
| --- | --- | --- |
| PowerShell 7.6.5 | `function /csp-spike-quick { param([string]$Value) $Value }` e invocazione diretta `/csp-spike-quick "two words"` | Restituisce `two words` |
| Windows PowerShell 5.1.26100.9818 | Stessa funzione e invocazione | Restituisce `two words` |
| Cmd interattivo con `/d`, in ConPTY | `doskey /csp-spike=echo __CSP_RESULT__ $1` e `/csp-spike "two words"` | Esegue la macro. `$1` produce `"two`. Una seconda macro con `$*` conserva `"two words"` |
| Cmd con Clink portabile 1.9.35 | Stessa macro, Clink iniettato nel processo figlio con profilo dedicato e `doskey.enhanced` predefinito | `$1` conserva `"two words"` |

Anche un alias con nome `/csp-spike-alias` funziona in entrambe le edizioni di PowerShell. Un comando con virgola iniziale dà `ParseException` in entrambe. Usare `/` come default configurabile e il nome senza prefisso come fallback. Tenere distinti il supporto del prefisso e la gestione degli argomenti. Le macro Doskey operano al prompt interattivo. Per cmd v2 servono test dedicati su metacaratteri e passaggio dei parametri a un eseguibile. [Doskey](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/doskey) · [Enhanced Doskey di Clink](https://chrisant996.github.io/clink/clink.html#enhanced-doskey-expansion)

## Frammenti di Windows Terminal

Microsoft documenta `profiles[].updates` per profili di sistema e dinamici. Nella versione locale 1.24.12741.0, il sorgente upstream cerca il GUID tra i profili già risolti e aggiunge il frammento come genitore del profilo utente. Un GUID sconosciuto viene ignorato. Il test upstream `LoadFragmentsWithMultipleUpdates` usa il GUID di Windows PowerShell per rinominarlo e quello di cmd per impostare il cursore. Le asserzioni del test coprono il nome e il numero di profili. Il test `FragmentUpdatesBaseProfile` copre il percorso di un'immagine del frammento. [Documentazione](https://learn.microsoft.com/en-us/windows/terminal/json-fragment-extensions) · [Loader della release](https://github.com/microsoft/terminal/blob/v1.24.12741.0/src/cascadia/TerminalSettingsModel/CascadiaSettingsSerialization.cpp#L981) · [Test sui profili](https://github.com/microsoft/terminal/blob/v1.24.12741.0/src/cascadia/UnitTests_SettingsModel/DeserializationTests.cpp#L2079) · [Test sulle risorse](https://github.com/microsoft/terminal/blob/v1.24.12741.0/src/cascadia/UnitTests_SettingsModel/MediaResourceTests.cpp#L596)

Generare JSON UTF-8 nella sottocartella dell'app in `%LOCALAPPDATA%\Microsoft\Windows Terminal\Fragments`, usando i GUID effettivi. Segnalare i valori che l'utente ha impostato sia sul singolo profilo sia in `profiles.defaults`. Terminal 1.24 carica definizioni di azioni dai frammenti ma esclude i loro binding a tasti. Per il profilo predefinito e i binding rimane necessario l'editing mirato di `settings.json`. [Gestione delle azioni nel loader](https://github.com/microsoft/terminal/blob/v1.24.12741.0/src/cascadia/TerminalSettingsModel/CascadiaSettingsSerialization.cpp#L925)

### Smoke test con il binario ufficiale

Windows Sandbox non era disponibile e il processo non disponeva dell'accesso a Hyper-V. Ho eseguito il binario x64 ufficiale 1.24.12741.0 con isolamento a livello di processo. Il file `.portable` mantiene `settings.json` e `state.json` nella copia sotto TEMP. Il launcher usa Microsoft Detours per reindirizzare le richieste a `SHGetKnownFolderPath` di LocalAppData e ProgramData e disabilitare il catalogo AppExtension WinRT nel solo processo di prova. Il log ha confermato entrambi i reindirizzamenti verso TEMP. I frammenti di prova, i file di stato e le impostazioni sono stati scritti sotto TEMP. [Release Terminal](https://github.com/microsoft/terminal/releases/tag/v1.24.12741.0) · [Microsoft Detours](https://github.com/microsoft/Detours/tree/v4.0.1)

Il runner ha aperto una scheda per ciascun GUID di sistema. Un comando osservatore avviato da ogni scheda ha registrato la directory effettiva. Entrambi i profili hanno dato lo stesso risultato in tutti e sei i casi.

| Caso | Prompt dei comandi | Windows PowerShell |
| --- | --- | --- |
| Baseline senza frammento | Home utente | Home utente |
| `updates` con GUID inesistente | Home utente | Home utente |
| `updates` sui GUID effettivi | Directory del frammento | Directory del frammento |
| Frammento e valore in `profiles.defaults` | Directory dei defaults | Directory dei defaults |
| Frammento, `profiles.defaults` e valore sul profilo in `settings.json` | Directory del profilo | Directory del profilo |
| Rimozione del frammento e ripristino della baseline | Home utente | Home utente |

Per ripetere la prova eseguire `pwsh -NoProfile -File docs/spikes/windows-terminal-smoke.ps1` su Windows x64 con gli strumenti C++ di Visual Studio. Lo [script di prova](windows-terminal-smoke.ps1) scarica le release fissate e stampa la posizione della cartella con le evidenze. La prova copre il caricamento dei frammenti da disco e la precedenza di una proprietà del profilo. Il catalogo MSIX e la resa grafica rimangono fuori dal perimetro di questo test.

## Modifica JSONC

Un prototipo con `jsonc-parser` di Microsoft ha localizzato il valore di `defaultProfile` nel testo e sostituito solo quel valore. La fixture CRLF conteneva commenti di riga e a blocco, un carattere accentato, virgole finali, chiavi annidate e un URL. Le cinque asserzioni hanno verificato uguaglianza byte per byte con il risultato atteso, rilettura, idempotenza, rifiuto di una chiave assente e rifiuto del JSONC malformato. [Parser Microsoft](https://github.com/microsoft/node-jsonc-parser)

Un secondo prototipo in Rust con `jsonc-parser` 0.34.0 e feature `cst` ha passato quattro asserzioni sulla stessa struttura: round trip byte per byte, sostituzione del solo valore, idempotenza e rifiuto del JSONC malformato. Usare il CST nel backend. L'applicatore deve preservare la codifica e l'eventuale BOM, rifiutare chiavi gestite ambigue e controllare l'hash prima della sostituzione. Servono test distinti per inserimento e rimozione di proprietà e virgole adiacenti. [API CST Rust](https://docs.rs/jsonc-parser/0.34.0/jsonc_parser/cst/index.html)

## Profilo PowerShell con OneDrive

I processi avviati con `-NoLogo -NoProfile -NonInteractive` hanno restituito `ConsoleHost` e i percorsi sotto il vero Documenti reindirizzato in OneDrive. I percorsi qui sono relativi a `OneDrive\Documenti`.

| Shell | `$PROFILE.CurrentUserCurrentHost` | `$PROFILE.CurrentUserAllHosts` |
| --- | --- | --- |
| `pwsh` | `PowerShell\Microsoft.PowerShell_profile.ps1` | `PowerShell\profile.ps1` |
| `powershell` | `WindowsPowerShell\Microsoft.PowerShell_profile.ps1` | `WindowsPowerShell\profile.ps1` |

Eseguire ciascun binario con `-NoProfile` e raccogliere `[string]$PROFILE.CurrentUserCurrentHost` per limitare l'integrazione al ConsoleHost usato da Windows Terminal. L'app può offrire `CurrentUserAllHosts` se si decide di estendere l'integrazione ad altri host. Rilevare la policy effettiva in ogni edizione e gestire il caso in cui il profilo non sia ancora stato creato. [Profili PowerShell 7](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_profiles?view=powershell-7.6) · [Profili Windows PowerShell 5.1](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_profiles?view=powershell-5.1)

## Motore del prompt

| Criterio | Oh My Posh | Starship |
| --- | --- | --- |
| Generazione e ordinamento | JSON con blocchi e segmenti ordinati, schema pubblico e colori per segmento | TOML con moduli ordinati tramite la stringa `format` e configurazione per modulo |
| Funzioni F5 | Segmenti percorso, Git, Node, .NET, durata e stato | Moduli directory, Git, Node, .NET, durata e stato |
| PowerShell | `oh-my-posh init pwsh` con `--config` per il file dell'app | `starship init powershell` con `STARSHIP_CONFIG` per il file dell'app |
| cmd v2 | Integrazione incorporata in Clink da 1.7.0 | Script Lua di inizializzazione con Clink da 1.2.30 |

In uno smoke test locale, Oh My Posh 31.9.1 e Starship 1.27.0 portabili hanno inizializzato e renderizzato un prompt minimo in PowerShell 7.6.5 e 5.1. Starship ha richiesto `TERM=xterm-256color` nel processo di test, che ereditava `TERM=dumb`. L'integrazione dei motori con cmd è verificata tramite le guide ufficiali.

Consiglio **Oh My Posh** per v1. L'elenco ordinato di segmenti corrisponde direttamente ai controlli dell'editor e la configurazione JSON è generabile con la serializzazione già prevista per il core. Tenere separati il modello dell'editor e il file generato. Il confronto copre la configurabilità e l'integrazione. Le prestazioni su repository grandi richiedono misure dedicate. [Configurazione Oh My Posh](https://ohmyposh.dev/docs/configuration/general) · [Integrazione Oh My Posh](https://ohmyposh.dev/docs/installation/prompt) · [Configurazione Starship](https://starship.rs/config/) · [Integrazione Starship](https://starship.rs/guide/)
