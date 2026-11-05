---
name: CLI Superpower
description: Pannello desktop per configurare Windows Terminal e le shell, con anteprima fedele e modifiche reversibili.
colors:
  primary: "#317bc4"
  primary-strong: "#2469ad"
  primary-strong-hover: "#1d5a96"
  primary-ink: "#2768aa"
  primary-ring: "#317bc426"
  focus-ring: "#8abaf4"
  night-sidebar: "#172435"
  night-sidebar-active: "#2d435b"
  night-sidebar-text: "#b6c5d6"
  night-sidebar-label: "#8fa4ba"
  night-sidebar-icon-active: "#83baf3"
  terminal-bg: "#111c2a"
  terminal-bar: "#253449"
  terminal-text: "#e1ebf6"
  terminal-prompt: "#75b1f1"
  terminal-success: "#81d7ae"
  ink: "#1c2939"
  text: "#202b3a"
  text-label: "#354457"
  text-secondary: "#5d6e82"
  text-muted: "#66778b"
  canvas: "#f5f7fa"
  surface: "#ffffff"
  surface-hover: "#f3f7fb"
  surface-sunken: "#e9f0f5"
  border: "#dfe6ee"
  border-control: "#d9e1ea"
  border-strong: "#d6e0e9"
  border-hover: "#9cc0e6"
  divider: "#e6ebf1"
  mint-live: "#62cda9"
  success: "#388c70"
  success-ink: "#2f7a60"
  success-deep: "#24604c"
  success-bg: "#e7f5ef"
  toast-success: "#173b30"
  danger: "#c64c55"
  danger-ink: "#a43b37"
  danger-link: "#a95656"
  danger-bg: "#fff1f0"
  danger-border: "#f1c9c5"
  toast-danger: "#7c1d25"
  warning-ink: "#85601f"
  info-bg: "#eaf4ff"
  info-ink: "#315e87"
  info-border: "#cee3f8"
typography:
  page-title:
    fontFamily: "\"Segoe UI Variable\", \"Segoe UI\", sans-serif"
    fontSize: "28px"
    fontWeight: 700
    lineHeight: 1.15
    letterSpacing: "-0.035em"
  headline:
    fontFamily: "\"Segoe UI Variable\", \"Segoe UI\", sans-serif"
    fontSize: "16px"
    fontWeight: 700
    lineHeight: 1.3
    letterSpacing: "-0.01em"
  title:
    fontFamily: "\"Segoe UI Variable\", \"Segoe UI\", sans-serif"
    fontSize: "15px"
    fontWeight: 650
    lineHeight: 1.35
  body:
    fontFamily: "\"Segoe UI Variable\", \"Segoe UI\", sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: "\"Segoe UI Variable\", \"Segoe UI\", sans-serif"
    fontSize: "12px"
    fontWeight: 650
    lineHeight: 1.4
  caption:
    fontFamily: "\"Segoe UI Variable\", \"Segoe UI\", sans-serif"
    fontSize: "11px"
    fontWeight: 400
    lineHeight: 1.5
  mono:
    fontFamily: "\"Cascadia Mono\", Consolas, monospace"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.8
rounded:
  xs: "5px"
  sm: "6px"
  md: "8px"
  lg: "10px"
  xl: "12px"
  pill: "999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "24px"
  2xl: "32px"
  page-x: "42px"
components:
  button-primary:
    backgroundColor: "{colors.primary-strong}"
    textColor: "{colors.surface}"
    typography: "{typography.label}"
    rounded: "{rounded.xs}"
    padding: "10px 15px"
  button-primary-hover:
    backgroundColor: "{colors.primary-strong-hover}"
  button-secondary:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text-secondary}"
    typography: "{typography.label}"
    rounded: "{rounded.xs}"
    padding: "10px 15px"
  button-secondary-hover:
    backgroundColor: "{colors.surface-hover}"
  button-link:
    backgroundColor: "transparent"
    textColor: "{colors.primary-strong}"
    typography: "{typography.label}"
    padding: "5px"
  button-link-danger:
    backgroundColor: "transparent"
    textColor: "{colors.danger-link}"
    typography: "{typography.label}"
    padding: "5px"
  input:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    typography: "{typography.body}"
    rounded: "{rounded.xs}"
    padding: "10px 11px"
    height: "39px"
  nav-item:
    backgroundColor: "transparent"
    textColor: "{colors.night-sidebar-text}"
    typography: "{typography.body}"
    rounded: "{rounded.sm}"
    padding: "12px"
  nav-item-active:
    backgroundColor: "{colors.night-sidebar-active}"
    textColor: "{colors.surface}"
  segmented-item-active:
    backgroundColor: "{colors.night-sidebar}"
    textColor: "{colors.surface}"
    typography: "{typography.label}"
    rounded: "{rounded.sm}"
    padding: "6px 12px"
  badge-team:
    backgroundColor: "{colors.success-bg}"
    textColor: "{colors.success-deep}"
    typography: "{typography.caption}"
    rounded: "{rounded.pill}"
    padding: "5px 10px"
  card-shell:
    backgroundColor: "{colors.surface}"
    rounded: "{rounded.lg}"
    padding: "12px 12px 18px"
  alert-info:
    backgroundColor: "{colors.info-bg}"
    textColor: "{colors.info-ink}"
    rounded: "{rounded.sm}"
    padding: "12px 15px"
  alert-error:
    backgroundColor: "{colors.danger-bg}"
    textColor: "{colors.danger-ink}"
    rounded: "{rounded.sm}"
    padding: "12px 15px"
  toast:
    backgroundColor: "{colors.toast-success}"
    textColor: "{colors.surface}"
    typography: "{typography.body}"
    rounded: "{rounded.lg}"
    padding: "12px 14px"
  terminal-preview:
    backgroundColor: "{colors.terminal-bg}"
    textColor: "{colors.terminal-text}"
    typography: "{typography.mono}"
    rounded: "7px"
    padding: "20px 18px"
---

# Design System: CLI Superpower

Questo documento descrive il linguaggio visivo dell'**app desktop** (Angular dentro Tauri, `app/src`). È normativo per ogni nuova schermata: i token nel frontmatter sono la fonte di verità, la prosa spiega dove e perché usarli. La landing (`landing/`) è una superficie di marca con regole sue, ma condivide la famiglia cromatica blu notte e azzurro e il logo del terminale spezzato.

## 1. Overview

**Creative North Star: "Il banco dell'officina"**

Un banco da lavoro ordinato: piano chiaro, attrezzi appesi al loro posto, ogni pezzo in lavorazione visibile sotto la lampada. Lo spazio di lavoro è chiaro e neutro, freddo appena verso il blu (`canvas`, `surface`); la struttura che orienta, cioè la sidebar di navigazione, è blu notte e resta ferma. Il pezzo in lavorazione è il terminale vero: l'anteprima scura e fedele è l'unico elemento ad alto contrasto della pagina, e tutto il resto le fa da cornice.

È un'interfaccia di prodotto, non di marca: l'utente è dentro un compito (scegliere colori, definire un comando, applicare). La densità è medio-alta, la scala tipografica è fissa e compatta, il colore d'accento compare solo dove c'è un'azione, una selezione o uno stato. La fiducia si costruisce mostrando cosa verrà scritto e come si annulla, non con decorazioni. Ogni affermazione dell'interfaccia ha accanto un'anteprima o un dato concreto.

Il sistema rifiuta esplicitamente: il template SaaS generico (hero con gradiente, griglie di card icona+titolo+testo, statistiche inventate), il tema hacker neon/verde su nero, lo stile rivista/editoriale con serif corsivo e filetti, e l'interfaccia interamente dark "perché i tool per sviluppatori sono dark".

**Key Characteristics:**
- Sidebar blu notte fissa (244px) + area di lavoro chiara con testata sottile e barra azioni fissa in basso.
- Una sola famiglia sans di sistema (Segoe UI Variable) e una mono (Cascadia Mono) riservata a terminale e codice.
- Superfici piatte separate da bordi da 1px; ombre solo per ciò che galleggia.
- Accento azzurro (`primary`) su meno del 10% della schermata.
- Anteprima del terminale come elemento firma, sempre fedele ai valori reali.
- Layout: contenuto con `max-width: 1470px` (Home `1120px`), margini laterali `42px`; a 1000px la sidebar scende a 200px, a 700px diventa una barra orizzontale scorrevole e i form passano a una colonna.

## 2. Colors: La palette Blu notte e Azzurro

Neutri freddi leggermente tinti verso il blu, un azzurro di lavoro come unico accento, un blu notte per la struttura e una famiglia semantica sobria per gli stati.

### Primary
- **Azzurro di lavoro** (`primary`): accento non testuale: checkbox (`accent-color`), bordi e indicatori di selezione, bordo dei campi con focus e anello `primary-ring` (3px). Mai come colore di testo piccolo né come fondo sotto testo bianco (4.41:1, sotto AA). Mai come sfondo di aree ampie né per decorazione.
- **Azzurro profondo** (`primary-strong`): fondo del pulsante primario e di ogni elemento pieno con testo bianco (5.68:1), testo azzurro su fondo chiaro: link e pulsanti testuali (`link-button`, `tool-link`, sempre con peso 650), passo corrente della guida. Hover dei pieni: `primary-strong-hover`.
- **Inchiostro azzurro** (`primary-ink`): testo della tab o voce selezionata su fondo chiaro.

### Secondary
- **Blu notte** (`night-sidebar`): sfondo della sidebar e stato attivo dei controlli segmentati (`tool-filter`). È il colore della struttura: orienta, non chiede azione.
- **Blu notte acceso** (`night-sidebar-active`): voce di navigazione attiva. Testo voci `night-sidebar-text`, etichette di gruppo `night-sidebar-label`, icona attiva `night-sidebar-icon-active`.

### Tertiary
- **Notte del terminale** (`terminal-bg`, barra `terminal-bar`): solo dentro l'anteprima del terminale. Testo `terminal-text`, prompt `terminal-prompt`, esito positivo `terminal-success`. Quando l'utente ha scelto uno schema colori, l'anteprima usa i colori reali dello schema e questi token diventano solo il fallback.
- **Menta di stato** (`mint-live`): pallino "attivo" nella sidebar e anello di focus dei link su fondo blu notte. Uso puntiforme.

### Neutral
- **Inchiostro** (`ink`): titoli di pagina, nomi, valori importanti.
- **Testo** (`text`): testo corrente; colore di default del `body`.
- **Testo etichetta** (`text-label`): etichette dei campi.
- **Testo secondario** (`text-secondary`): descrizioni, sottotitoli, testo dei pulsanti secondari. È il grigio più chiaro ammesso per testo leggibile su `canvas` (4.87:1).
- **Testo attenuato** (`text-muted`): note e didascalie solo su `surface` bianco; placeholder dei campi.
- **Tela** (`canvas`): sfondo della pagina.
- **Superficie** (`surface`): card, campi, testata, barra azioni.
- **Superficie hover** (`surface-hover`) e **superficie incassata** (`surface-sunken`, fondo dei controlli segmentati).
- **Bordi**: `border` per card, `border-control` per campi, `border-strong` per pulsanti secondari e filtri, `border-hover` per card al passaggio del mouse, `divider` per separatori tra righe.

### Semantic
- **Successo**: `success` (pallino "installato"), `success-ink` (testo di stato su bianco), `success-deep` (testo su fondi verdi chiari, badge "Team"), `success-bg` (fondo del badge), `toast-success` (fondo del toast).
- **Errore**: `danger` (anello di errore), `danger-ink` (testo di avviso), `danger-link` (azioni distruttive testuali, es. "Rimuovi"), `danger-bg` + `danger-border` (avviso), `toast-danger`.
- **Attenzione**: `warning-ink` per stati "mancante" o "non installato".
- **Informazione**: `info-bg`, `info-ink`, `info-border` per gli avvisi neutri in testa alla pagina.

### Named Rules
**The Ten Percent Rule.** L'azzurro `primary` occupa meno del 10% di qualunque schermata. Se una pagina ha più di un pulsante primario visibile, uno dei due è sbagliato.

**The Readable Gray Rule.** Il testo leggibile non scende mai sotto `text-secondary` su `canvas` né sotto `text-muted` su `surface`. Grigi più chiari (es. `#738294`, `#8a98a8`, `#a5b0bd`) restano sotto 4.5:1: ammessi solo per icone, bordi e testo su fondo blu notte, mai per testo su fondo chiaro.

**The Night Is Structure Rule.** Il blu notte indica navigazione e selezione forte, mai contenuto. Niente card, pannelli o pagine dark nell'area di lavoro, tranne l'anteprima del terminale.

## 3. Typography

**Display Font:** nessuno (l'app non usa un font display)
**Body Font:** Segoe UI Variable (con Segoe UI, sans-serif)
**Label/Mono Font:** Cascadia Mono (con Consolas, monospace)

**Character:** il font di sistema di Windows fa sembrare l'app nativa e sparisce dietro al compito; la mono di Windows Terminal compare solo dove c'è testo che finirà davvero in un terminale, così il passaggio di famiglia segnala "questo è codice".

### Hierarchy
- **Page title** (700, 28px, 1.15, -0.035em): una volta per pagina, nella testata. Unica eccezione fluida ammessa: l'editor aspetto (`clamp(26px, 3vw, 38px)`).
- **Headline** (700, 16px, 1.3, -0.01em): titolo di sezione (`h2`).
- **Title** (650, 15px, 1.35): nome di un elemento (shell, gruppo di strumenti), titolo di dialogo.
- **Body** (400, 13px, 1.5): testo corrente, celle di tabella, voci di navigazione. Prosa limitata a 62ch.
- **Label** (650, 12px): etichette dei campi, pulsanti, link testuali. Parte facoltativa dell'etichetta in 400 e `text-secondary`.
- **Caption** (400, 11px): testata, metadati, stati, sottotitoli della barra azioni.
- **Mono** (400, 13px, 1.8): corpo del terminale; 12px/1.75 nelle miniature della Home; 13px/1.5 per codice inline e anteprime di comando.

### Named Rules
**The Fixed Scale Rule.** Scala fissa in px con passi da 11, 12, 13, 15, 16, 28. Niente valori intermedi (12.5px, 13.5px) e niente tipografia fluida fuori dal titolo dell'editor aspetto.

**The Mono Means Terminal Rule.** Cascadia Mono solo per testo che è o diventerà input/output di una shell: anteprime, comandi, percorsi, segnaposto `{param}`. Mai per etichette, pulsanti o titoli.

**The Quiet Kicker Rule.** Il maiuscolo spaziato (10px, 0.08em, 700) è ammesso solo per l'indice di un elemento in una lista (`item-index`). Non mettere occhielli maiuscoli sopra le sezioni.

## 4. Elevation

Il sistema è piatto. La profondità a riposo nasce dal contrasto tonale fra `canvas`, `surface` e la sidebar blu notte, e da bordi da 1px. Le ombre esistono solo per tre motivi: rispondere a uno stato (hover di una card), far galleggiare qualcosa sopra il contenuto (toast, dialogo, guida), o staccare l'anteprima del terminale come oggetto fisico.

### Shadow Vocabulary
- **Hover card** (`box-shadow: 0 6px 18px #23476d14`): card selezionabili al passaggio del mouse, insieme a `border-hover`.
- **Oggetto terminale** (`box-shadow: 0 12px 30px #15283d20`): anteprima del terminale nell'editor.
- **Galleggiante** (`box-shadow: 0 12px 32px #0b162455`): toast.
- **Dialogo** (`box-shadow: 0 24px 60px #0b162455`): dialoghi, guida passo passo, schermata di salvataggio.
- **Anello di focus** (`box-shadow: 0 0 0 3px #317bc426`): campi con focus. Varianti di stato a 4px: errore `#c64c5530`, attenzione `#e0a03030`, successo `#62cda926`.
- **Filo interno** (`box-shadow: inset 0 0 0 1px #ffffff17`): bordo luminoso delle miniature del terminale.

### Livelli (z-index)
Scala semantica, mai valori arbitrari: contenuto `1`, barra azioni fissa `2`, sfondo di dialogo `9`, dialogo e guida `10`, toast `20`.

### Named Rules
**The Flat-At-Rest Rule.** Una superficie ferma non ha ombra. Se una card ha ombra senza essere in hover o galleggiare, l'ombra va tolta.

**The Two-Layer Rule.** Al massimo due livelli di superficie annidati (pagina, card). Card dentro card è vietato: dentro una card si separa con `divider`.

## 5. Components

Carattere generale: familiare e preciso, come un pannello impostazioni di Windows fatto con cura. Ogni componente interattivo ha stati default, hover, focus visibile, disabilitato (opacità 0.5, cursore `not-allowed`) e, dove serve, caricamento ed errore.

### Buttons
- **Shape:** angoli appena smussati (5px).
- **Primary:** fondo e bordo `primary-strong`, testo bianco, label 12px/700, padding 10px 15px, `white-space: nowrap`. Un solo primario per vista; di norma vive nella barra azioni ("Applica").
- **Hover / Focus:** hover su `primary-strong-hover`; focus con contorno 3px `focus-ring` e offset 2px, su tutti i controlli.
- **Secondary:** fondo bianco, bordo `border-strong`, testo `text-secondary`; hover `surface-hover`.
- **Link:** testo `primary-strong` 12px/650 senza fondo; variante distruttiva `danger-link`. Le etichette dicono l'azione: verbo + oggetto ("Rimuovi comando", non "OK").
- **Icon button:** quadrato 39px, stessa forma del secondario, con `aria-label` obbligatorio.

### Chips
- **Badge origine:** pillola (999px) `success-bg` / `success-deep`, 11px/700, padding 5px 10px. Mostra "Team" o "Personale" accanto all'elemento.
- **Controllo segmentato** (`tool-filter`): contenitore bianco con bordo `border-strong`, raggio 8px, padding 3px; voci 12px/650 raggio 6px, hover `#eef3f8`, voce attiva in `night-sidebar` con testo bianco. Variante tab profili: fondo `surface-sunken`, voce selezionata bianca con bordo `#b8cbdc` e testo `#16456e`.

### Cards / Containers
- **Corner Style:** 10px per card, 6px per avvisi, 12px per dialoghi.
- **Background:** `surface` su `canvas`.
- **Shadow Strategy:** nessuna a riposo; hover card e galleggianti come da Elevation.
- **Border:** 1px `border`; in hover `border-hover`.
- **Internal Padding:** 12px per card con anteprima, 26px 30px per dialoghi.
- **Liste e righe:** gli elenchi (strumenti, comandi, cronologia) sono righe separate da `divider`, non card ripetute.

### Inputs / Fields
- **Style:** fondo bianco, bordo 1px `border-control`, raggio 5px, padding 10px 11px, altezza minima 39px, testo 13px `text`. Etichetta sopra il campo (12px/650 `text-label`) con 7px di distanza. Le `textarea` usano la mono con interlinea 1.5.
- **Focus:** contorno 3px `focus-ring` offset 2px, oppure anello `primary-ring`.
- **Error / Disabled:** messaggio sotto il campo in `field-error` (`#a72731`, 0.75rem, 1.4) che dice cosa correggere; disabilitato a opacità 0.5.
- **Form:** griglia a due colonne con gap 19px 17px; i campi lunghi occupano la riga intera; sotto 700px una colonna.

### Navigation
- **Sidebar:** 244px, `night-sidebar`, sticky a tutta altezza, padding 28px 16px 20px. Logo del terminale spezzato in alto, etichetta di gruppo 11px/600 `night-sidebar-label`, voci 13px con icona 20px e raggio 6px; voce attiva `night-sidebar-active` con icona `night-sidebar-icon-active`. Link esterni in fondo separati da un filo `#36485d`.
- **Testata:** 40px, bianca, filo inferiore, testo 11px: percorso e versione.
- **Barra azioni:** fissa in basso, bianca, filo superiore, altezza minima 82px: a sinistra stato delle modifiche (12px forte + 11px), a destra i pulsanti.
- **Tab di pagina:** testo 13px `#748497` con sottolineatura 2px trasparente; selezionata `primary-ink`, sottolineatura `primary`, 700.
- **Mobile (≤700px):** sidebar diventa barra orizzontale scorrevole, etichette e link nascosti; barra azioni a tutta larghezza in colonna.

### Feedback
- **Avviso in pagina:** `alert-info` o `alert-error`, con bordo 1px del colore relativo e testo 12px.
- **Toast:** in basso a destra sopra la barra azioni (24px, 88px), larghezza massima 440px, icona tonda 22px, pulsante di chiusura; variante errore `toast-danger`.
- **Stato vuoto:** centrato, icona 48px su `#eaf2fb` raggio 12px, titolo, una frase che insegna cosa fare (max 410px) e il pulsante dell'azione.

### Anteprima del terminale (componente firma)
- **Struttura:** barra superiore 38px (`terminal-bar`, testo 10px) e corpo con padding 20px 18px, altezza minima 235px, mono 13px/1.8, raggio 7px, ombra "oggetto terminale".
- **Fedeltà:** rende i valori reali del profilo: schema colori, font, cursore (`bar`, `filledBox`, `emptyBox`, `underscore`, `doubleUnderscore`, `vintage`), immagine di sfondo con `object-fit: cover`, opacità e acrilico (`backdrop-filter: blur(8px)`). Dietro uno sfondo trasparente compare la scacchiera `#43556a`/`#526478` a 24px.
- **Miniature della Home:** proporzione 16:10, raggio 6px, mono 12px/1.75, filo interno luminoso, dentro una card shell con nome (15px/650) e dati (12px).

## 6. Do's and Don'ts

### Do:
- **Do** affiancare a ogni scelta di aspetto un'anteprima del terminale che usa i valori reali, non illustrazioni decorative.
- **Do** mostrare cosa verrà scritto e come si annulla: backup, manifest e disinstallazione sono contenuto di primo piano, con il loro spazio nella pagina.
- **Do** tenere l'azzurro `primary` per azioni, selezione e stato, su meno del 10% della schermata.
- **Do** usare `text-secondary` come grigio minimo per il testo su `canvas` e verificare 4.5:1 per ogni nuovo abbinamento.
- **Do** dare a ogni controllo il focus visibile (contorno 3px `focus-ring`, offset 2px) e un nome accessibile ai pulsanti solo icona.
- **Do** limitare le transizioni a 150–200ms su colore, bordo, ombra e `transform`, e prevedere sempre `@media (prefers-reduced-motion: reduce)` con transizione istantanea.
- **Do** separare le righe di un elenco con `divider` da 1px e tenere le card per oggetti selezionabili (shell, profili).
- **Do** scrivere testi diretti in italiano con esempi concreti (`/run`, `/ndev`) e pulsanti verbo + oggetto.

### Don't:
- **Don't** usare il template SaaS generico: hero con gradiente, griglia di card icona+titolo+testo, statistiche inventate.
- **Don't** usare il tema hacker neon/verde su nero, nemmeno nell'anteprima di default.
- **Don't** usare lo stile rivista/editoriale: serif corsivo, colonne con filetti, tutto monocromo.
- **Don't** rendere l'app interamente dark come ogni tool per sviluppatori: il blu notte resta nella sidebar e nel terminale.
- **Don't** usare bordi laterali colorati più spessi di 1px come accento su card, righe o avvisi.
- **Don't** usare testo in gradiente, glassmorphism decorativo o ombre su superfici ferme.
- **Don't** annidare card dentro card.
- **Don't** usare Cascadia Mono per etichette, pulsanti o titoli, né aggiungere una terza famiglia di font.
- **Don't** usare valori di z-index fuori dalla scala (niente 999 o 9999).
- **Don't** aprire un dialogo quando basta un'azione in linea; i dialoghi sono per conferme distruttive e guide.
- **Don't** introdurre nuovi grigi, raggi o dimensioni di testo fuori dai token: se serve un valore nuovo, si aggiunge qui prima di usarlo.
- **Don't** mettere testo bianco su `primary` né usare `primary` come colore di testo: 4.41:1 è sotto AA per testo piccolo. Per pieni e testo azzurro si usa `primary-strong`.
