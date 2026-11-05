# CLI Superpower

Il documento di riferimento è `PROGETTO.md`. La Fase 0 prepara le fondamenta; non anticipare funzionalità dell'MVP.

## Principi

- Non distruttivo: scrivere nei file dell'app; modificare file dell'utente solo per il minimo indispensabile, con backup e marcatori o chiavi gestite.
- Dichiarativo: la configurazione salvata è l'unica fonte di verità; i file di sistema sono generati.
- Idempotente: applicare due volte produce lo stesso risultato; non accodare righe ripetutamente.
- Reversibile: tracciare le modifiche in un manifest e ripristinare lo stato originale alla disinstallazione.
- Riutilizzare strumenti maturi invece di reinventarli.
- Un'unica configurazione: ogni elemento dichiara lo scope delle shell a cui si applica.

## Regole di sviluppo

- I generatori sono funzioni pure, senza I/O, ambiente o orologio; ogni generatore ha snapshot test.
- Solo l'applicatore scrive su disco, passando da backup e manifest. Non modificare `$PROFILE` o `settings.json` fuori dalle parti gestite.
- Ogni modifica alle struct di configurazione richiede incremento di `schemaVersion`, migrazione e relativi test.
- Dopo modifiche alle struct Rust esposte alla UI, rigenerare i tipi TypeScript.
- Risolvere i percorsi di sistema a runtime; chiedere `$PROFILE` alla shell e usare le API per le cartelle note.
- Prima di dichiarare concluso un task, eseguire test Rust, clippy, test Angular e build pertinenti; dichiarare eventuali limiti.
