import { DOCUMENT, NgTemplateOutlet } from '@angular/common';
import { afterNextRender, ChangeDetectorRef, Component, inject, Injector } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { ConfigBridge } from './config-bridge';
import { ConfigIssue, EditorSection, sectionForError, validateConfig } from './config-validation';
import { Appearance, Framework, HistoryEntry, Pane, PathStyle, PersonalConfig, Profile, PromptSegment, PromptSegmentId, QuickCommand, Shell, ShellBehavior, TeamPreview, ToolStatus, Workspace } from './models';

type Section = EditorSection | 'home' | 'tools' | 'history';
type Operation = 'save';
interface SegmentOption { id:PromptSegmentId; label:string; description:string; sample:string; }
interface FrameworkDraft { framework: Framework; name: string; port: string; extra: string; error: string; portError: string; extraError: string; }
const WALKTHROUGH_KEY = 'csp.walkthrough';
@Component({selector:'app-root', imports:[FormsModule, NgTemplateOutlet], templateUrl:'./app.html'})
export class App {
  readonly bridge = inject(ConfigBridge);
  private readonly changeDetector = inject(ChangeDetectorRef);
  private readonly injector = inject(Injector);
  private readonly document = inject(DOCUMENT);
  readonly profiles: {id:Profile; label:string}[] = [{id:'pwsh',label:'PowerShell 7'}, {id:'powershell',label:'Windows PowerShell'}, {id:'cmd',label:'Prompt dei comandi'}];
  /** Come può comparire il percorso della cartella nel prompt, con un esempio. */
  readonly pathStyles: {id: PathStyle; label: string; sample: string}[] = [
    {id:'short', label:'Abbreviato', sample:'~\\..\\Desktop'},
    {id:'home', label:'Dalla home, con ~', sample:'~\\OneDrive\\Desktop'},
    {id:'full', label:'Intero, senza ~', sample:'C:\\Users\\nome\\OneDrive\\Desktop'},
    {id:'folder', label:'Solo la cartella', sample:'Desktop'},
  ];
  readonly segmentOptions: SegmentOption[] = [
    {id:'directory', label:'Cartella', description:'La cartella in cui ti trovi.', sample:'~\\PROGETTI'},
    {id:'git', label:'Git', description:'Il branch e lo stato del repository, solo dentro un repository Git.', sample:'main'},
    {id:'node', label:'Node.js', description:'La versione di Node.js, solo nei progetti Node.', sample:'node 22.4'},
    {id:'dotnet', label:'.NET', description:'La versione di .NET, solo nei progetti .NET.', sample:'.NET 9.0'},
    {id:'duration', label:'Durata ultimo comando', description:'Quanto è durato l’ultimo comando, solo se supera la soglia.', sample:'3s'},
    {id:'status', label:'Esito ultimo comando', description:'Se l’ultimo comando è riuscito o fallito.', sample:'✓'},
  ];
  history:HistoryEntry[] | null = null;
  formatHistoryDate(timestamp:number):string { return new Date(timestamp * 1000).toLocaleString('it-IT'); }
  async openHistory() { this.section = 'history'; await this.refreshHistory(); }
  async refreshHistory() { this.busy = true; this.error = ''; try { this.history = await this.bridge.listHistory(); } catch(e) { this.error = this.message(e); } finally { this.finishAsync(); } }
  async restoreLatest() {
    if (!this.history?.[0]?.canRestore || !window.confirm('Ripristinare l’ultima applicazione? Le modifiche non salvate nell’editor andranno perse.')) return;
    this.busy = true; this.error = ''; let restored = false;
    try { await this.bridge.restoreLatest(); restored = true; this.config = await this.bridge.load(); this.loadFailed = false; this.issues = []; this.validationAttempted = false; this.savedSnapshot = JSON.stringify(this.config); this.teamPreview = null; this.history = await this.bridge.listHistory(); this.notice = 'Ultima applicazione ripristinata e configurazione ricaricata.'; }
    catch(e) { if (restored) { this.loadFailed = true; this.error = 'Ripristino eseguito, ma ricarica non riuscita: ' + this.message(e); } else this.error = this.message(e); }
    finally { this.finishAsync(); }
  }
  toolList:ToolStatus[] | null = null;
  toolFilter: 'all' | 'installed' | 'missing' = 'all';
  /** Strumento su cui sta lavorando l'installazione o la rimozione, per mostrare l'avanzamento sulla sua riga. */
  busyToolId: string | null = null;
  readonly toolInfo: Record<string, { group: string; text: string }> = {
    'pwsh': { group: 'shell', text: 'La versione moderna di PowerShell, più veloce e aggiornata di Windows PowerShell.' },
    'clink': { group: 'shell', text: 'Aggiunge autocompletamento e cronologia al Prompt dei comandi.' },
    'oh-my-posh': { group: 'prompt', text: 'Disegna il prompt con cartella, branch Git e altre informazioni.' },
    'starship': { group: 'prompt', text: 'Alternativa leggera a Oh My Posh per il prompt.' },
    'nerd-font': { group: 'prompt', text: 'Font con le icone usate dai prompt grafici.' },
    'zoxide': { group: 'utils', text: 'Ti porta nelle cartelle usate spesso con pochi caratteri.' },
    'fzf': { group: 'utils', text: 'Cerca file e comandi digitando solo una parte del nome.' },
    'tealdeer': { group: 'utils', text: 'Mostra esempi pratici d’uso per i comandi (tldr).' },
    'eza': { group: 'utils', text: 'Elenco dei file più chiaro, con colori e icone.' },
    'bat': { group: 'utils', text: 'Mostra il contenuto dei file con colori e numeri di riga.' },
    'jq': { group: 'utils', text: 'Legge e filtra dati JSON dalla riga di comando.' },
    'lazygit': { group: 'utils', text: 'Interfaccia a tastiera per usare Git senza ricordare i comandi.' },
  };
  readonly toolGroupLabels: { id: string; label: string; hint: string }[] = [
    { id: 'shell', label: 'Shell', hint: 'Dove scrivi i comandi' },
    { id: 'prompt', label: 'Prompt e aspetto', hint: 'Come appare la riga di comando' },
    { id: 'utils', label: 'Utilità da terminale', hint: 'Programmi che rendono più veloci i comandi di ogni giorno' },
  ];
  get toolSummary() {
    const tools = this.toolList ?? [];
    const installed = tools.filter(tool => tool.installed).length;
    return { installed, missing: tools.length - installed, total: tools.length };
  }
  get toolGroups() {
    const tools = (this.toolList ?? []).filter(tool => this.toolFilter === 'all' || (this.toolFilter === 'installed') === tool.installed);
    return this.toolGroupLabels
      .map(group => ({ ...group, tools: tools.filter(tool => (this.toolInfo[tool.id]?.group ?? 'utils') === group.id) }))
      .filter(group => group.tools.length);
  }
  /**
   * Stato degli strumenti. Rilevarli costa (l'app avvia processi di sistema), quindi si tiene una copia in memoria:
   * si rinnova quando scade, su richiesta («Aggiorna») e dopo ogni installazione o rimozione. Più richieste
   * contemporanee condividono la stessa risposta.
   */
  private toolsFetchedAt = 0;
  private toolsInFlight: Promise<ToolStatus[]> | null = null;
  private static readonly TOOLS_TTL_MS = 10 * 60 * 1000;
  /** Segna la copia come scaduta: la prossima lettura interroga di nuovo il sistema. */
  invalidateTools() { this.toolsFetchedAt = 0; }
  private get toolsFresh(): boolean { return this.toolList !== null && Date.now() - this.toolsFetchedAt < App.TOOLS_TTL_MS; }
  private async loadTools(force = false): Promise<ToolStatus[]> {
    // Dopo una modifica serve una lettura nuova: una già in corso potrebbe essere partita prima.
    if (force && this.toolsInFlight) await this.toolsInFlight.catch(() => undefined);
    else if (!force && this.toolsFresh) return this.toolList!;
    this.toolsInFlight ??= this.bridge.listTools()
      .then(tools => { this.toolList = tools; this.toolsFetchedAt = Date.now(); return tools; })
      .finally(() => { this.toolsInFlight = null; });
    return this.toolsInFlight;
  }
  async openTools() { this.section = 'tools'; await this.refreshTools(false); }
  /** Aggiorna lo stato degli strumenti senza bloccare l'interfaccia, per avvisare se manca oh-my-posh. */
  private async refreshToolStatus() {
    if (!this.bridge.desktop) return;
    try { await this.loadTools(); } catch { /* l'avviso resta nascosto se lo stato non è noto */ }
    this.changeDetector.markForCheck();
  }
  openPrompt() { this.section = 'prompt'; void this.refreshToolStatus(); }
  /** Il prompt usa oh-my-posh: senza lo strumento lo script di avvio lo salta in silenzio. */
  get promptToolMissing(): boolean {
    return !!this.config.prompt?.enabled && !!this.toolList?.some(tool => tool.id === 'oh-my-posh' && !tool.installed);
  }
  /** `force` è vero per il pulsante «Aggiorna»; aprendo la pagina si riusa la copia recente. */
  async refreshTools(force = true) {
    if (!force && this.toolsFresh) return;
    this.busy = true; this.error = '';
    try { await this.loadTools(force); } catch(e) { this.error = this.message(e); } finally { this.finishAsync(); }
  }
  async uninstallTool(id:string) {
    const tool = this.toolList?.find(item => item.id === id);
    if (!this.bridge.desktop || !tool?.canUninstall || !window.confirm(`Disinstallare ${tool.name}? Le funzioni che lo usano smetteranno di funzionare finché non lo reinstalli.`)) return;
    this.busy = true; this.busyToolId = id; this.error = '';
    try { await this.bridge.uninstallTool(id); await this.loadTools(true); this.notice = `${tool.name} disinstallato.`; }
    catch(e) { this.error = this.message(e); } finally { this.busyToolId = null; this.finishAsync(); }
  }
  /** Perché l'app propone ogni strumento consigliato. */
  readonly toolReasons: Record<string, string> = {'oh-my-posh': 'Disegna il prompt informativo di PowerShell. Pesa circa 6 MB.'};
  toolOffer: ToolStatus[] | null = null;
  private static readonly TOOLS_OFFER_KEY = 'csp.toolsOffer';
  private async checkToolOffer() {
    if (!this.bridge.desktop) return;
    try { if (localStorage.getItem(App.TOOLS_OFFER_KEY) === 'declined') return; } catch { /* si propone comunque */ }
    try {
      const tools = await this.loadTools();
      const missing = tools.filter(tool => tool.recommended && !tool.installed && tool.canInstall);
      this.toolOffer = missing.length ? missing : null;
    } catch { /* senza stato degli strumenti non si propone nulla */ }
    this.changeDetector.markForCheck();
  }
  declineToolOffer() {
    try { localStorage.setItem(App.TOOLS_OFFER_KEY, 'declined'); } catch { /* la proposta tornerà al prossimo avvio */ }
    this.toolOffer = null;
  }
  async acceptToolOffer() {
    this.busy = true; this.error = '';
    try {
      const names = await this.bridge.installRecommendedTools();
      await this.loadTools(true);
      this.toolOffer = null;
      this.notice = names.length ? `Installato: ${names.join(', ')}. Apri una nuova finestra del terminale per usarlo.` : 'Gli strumenti consigliati erano già installati.';
    } catch(e) { this.error = this.message(e); this.toolOffer = null; } finally { this.finishAsync(); }
  }
  async installTool(id:string) {
    if (!this.bridge.desktop) return;
    const name = this.toolList?.find(item => item.id === id)?.name ?? 'Lo strumento';
    this.busy = true; this.busyToolId = id; this.error = '';
    try { await this.bridge.installTool(id); await this.loadTools(true); this.notice = `${name} installato.`; }
    catch(e) { this.error = this.message(e); } finally { this.busyToolId = null; this.finishAsync(); }
  }
  teamContent = ''; teamFilename = ''; teamPreview: TeamPreview | null = null; private teamSnapshot = '';
  newSegment: PromptSegmentId = 'directory';
  section: Section = 'home'; profile: Profile = 'pwsh'; copyTarget: Profile = 'powershell';
  config: PersonalConfig = {schemaVersion:6};
  private savedSnapshot = ''; loadFailed = false; busy = true; error = '';
  toast: {kind:'success'|'error'; text:string} | null = null; private toastTimer?: ReturnType<typeof setTimeout>; private noticeText = '';
  get notice(): string { return this.noticeText; }
  set notice(value: string) { this.noticeText = value; if (value) this.showToast('success', value); else if (this.toast?.kind === 'success') this.dismissToast(); }
  showToast(kind:'success'|'error', text:string) { clearTimeout(this.toastTimer); this.toast = {kind, text}; this.toastTimer = setTimeout(() => this.dismissToast(), kind === 'error' ? 9000 : 5000); this.changeDetector.markForCheck(); }
  dismissToast() { clearTimeout(this.toastTimer); this.toast = null; this.changeDetector.markForCheck(); }
  operationInProgress: Operation | null = null;
  private validationAttempted = false;
  private backendErrorSection?: EditorSection;
  issues: ConfigIssue[] = [];
  get errorSection(): EditorSection | undefined { return this.backendErrorSection; }
  get dirty(): boolean { return this.savedSnapshot !== JSON.stringify(this.config); }
  get saveStatus(): string {
    if (this.busy) return 'Operazione in corso…';
    if (this.loadFailed) return 'Ricarica la configurazione per continuare.';
    if (this.issues.length) return 'Correggi i campi segnalati, poi salva.';
    return this.dirty ? 'Hai modifiche non salvate.' : 'Nessuna modifica da salvare.';
  }
  readonly walkthroughSteps = [
    {title:'Scegli una shell', text:'Nella pagina iniziale trovi PowerShell 7, Windows PowerShell e il Prompt dei comandi. Ognuna mostra come apparirà con le tue impostazioni.'},
    {title:'Personalizza', text:'Cambia tema, font e sfondo, e crea comandi rapidi come /run per avviare ciò che usi ogni giorno.'},
    {title:'Salva', text:'Salva aggiorna subito il terminale. Apri una nuova finestra per vedere il risultato. Puoi sempre annullare dalla Cronologia.'},
  ];
  walkthroughStep: number | null = null;
  openWalkthrough() { this.walkthroughStep = 0; this.reveal('walkthrough-title'); }
  nextWalkthroughStep() { if (this.walkthroughStep === null) return; if (this.walkthroughStep < this.walkthroughSteps.length - 1) this.walkthroughStep++; else this.closeWalkthrough(); }
  previousWalkthroughStep() { if (this.walkthroughStep) this.walkthroughStep--; }
  closeWalkthrough() { this.walkthroughStep = null; try { localStorage.setItem(WALKTHROUGH_KEY, 'done'); } catch { /* The guide shows again next time. */ } }
  private walkthroughSeen(): boolean { try { return localStorage.getItem(WALKTHROUGH_KEY) === 'done'; } catch { return false; } }
  openProfile(profile:Profile) { this.selectProfile(profile); this.section = 'appearance'; this.reveal('editor-heading'); }
  shellCommands(profile:Profile): QuickCommand[] { return this.commands.filter(command => command.shells.includes(profile) && command.name); }
  themeFor(profile:Profile) { return this.appearanceThemes.find(theme => theme.name === this.config.appearance?.[profile]?.colorScheme) ?? this.appearanceThemes[0]; }
  imageFor(profile:Profile): string | null { const path = this.config.appearance?.[profile]?.backgroundImage; return path ? this.previewImages[path] ?? null : null; }
  profileLabel(profile:Profile): string { return this.profiles.find(item => item.id === profile)!.label; }
  hasCustomConfig(profile:Profile): boolean {
    const appearance = this.config.appearance?.[profile];
    return !!(appearance && Object.keys(appearance).length) || this.shellCommands(profile).length > 0 || !!this.behavior(profile as Shell) || (profile !== 'cmd' && !!this.config.prompt?.enabled);
  }
  commandCountLabel(profile:Profile): string { const count = this.shellCommands(profile).length; return count === 0 ? 'Nessun comando rapido' : count === 1 ? '1 comando rapido' : `${count} comandi rapidi`; }
  fieldIssue(field: string): string | null { return this.issues.find(issue => issue.field === field)?.message ?? null; }
  issueCount(section: EditorSection): number { return this.issues.filter(issue => issue.section === section).length; }
  goToIssue(section: EditorSection, field = 'editor-heading') {
    // Autocomplete vive nelle schede delle shell, in Inizio.
    this.section = section === 'autocomplete' ? 'appearance' : section;
    const shell = /^behavior-(pwsh|powershell|cmd)-/.exec(field)?.[1] as Shell | undefined;
    if (shell) this.profile = shell;
    this.reveal(section === 'commands' ? field.replace(/-args$/, '-program') : field);
  }
  private reveal(id: string) {
    afterNextRender(() => {
      const element = this.document.getElementById(id) ??
        this.document.querySelector<HTMLElement>('.page h1');
      const disclosure = element?.closest('details');
      if (disclosure) disclosure.open = true;
      element?.focus({preventScroll: true});
      element?.scrollIntoView?.({block: 'nearest'});
    }, {injector: this.injector});
  }
  constructor() {
    if (!this.walkthroughSeen()) this.openWalkthrough();
    void this.load();
    // Tornando nell'app dopo un po' l'utente può aver installato qualcosa altrove: la copia non è più affidabile.
    window.addEventListener('focus', () => { if (Date.now() - this.toolsFetchedAt > 60_000) this.invalidateTools(); });
  }
  get appearance(): Appearance { return this.config.appearance?.[this.profile] ?? {}; }
  get commands(): QuickCommand[] { return this.config.commands ?? []; }
  get segments(): PromptSegment[] { return this.config.prompt?.segments ?? []; }
  segmentLabel(id:PromptSegmentId): string { return this.segmentOptions.find(s => s.id === id)?.label ?? id; }
  setPromptEnabled(enabled:boolean) {
    const prompt = this.config.prompt ??= {enabled:false,segments:[]};
    prompt.enabled = enabled;
    if (enabled && !prompt.segments.length) prompt.segments.push({id:'directory'});
    this.changed();
    if (enabled) void this.refreshToolStatus();
  }
  /** Tutte le informazioni disponibili: prima quelle scelte, nell'ordine del prompt, poi le altre. */
  get promptRows(): {option: SegmentOption; segment?: PromptSegment; index: number}[] {
    const chosen = this.segments.map((segment, index) => ({option: this.segmentOptions.find(item => item.id === segment.id)!, segment, index}));
    const others = this.segmentOptions.filter(option => !this.hasSegment(option.id)).map(option => ({option, segment: undefined, index: -1}));
    return [...chosen, ...others];
  }
  /** Il prompt è attivo quando c'è almeno un'informazione: scegliere la prima lo accende, togliere l'ultima lo spegne. */
  toggleSegment(id:PromptSegmentId, on:boolean) {
    const prompt = this.config.prompt ??= {enabled:false,segments:[]};
    if (on && !prompt.segments.some(segment => segment.id === id)) prompt.segments.push({id});
    if (!on) prompt.segments = prompt.segments.filter(segment => segment.id !== id);
    prompt.enabled = prompt.segments.length > 0;
    this.changed();
    if (prompt.enabled) void this.refreshToolStatus();
  }
  /** Colore mostrato nel selettore: quello scelto, altrimenti quello che l'app usa di default. */
  swatchColor(segment: PromptSegment): string {
    return /^#[0-9a-fA-F]{6}$/.test(segment.color ?? '') ? segment.color!.toLowerCase() : segment.id === 'status' ? '#f87171' : '#d8dee9';
  }
  pickColor(segment: PromptSegment, value: string) { this.setOptional(segment, 'color', value.toUpperCase()); }
  pathSample(segment: PromptSegment): string { return this.pathStyles.find(item => item.id === (segment.pathStyle ?? 'short'))?.sample ?? ''; }
  setPathStyle(segment: PromptSegment, style: PathStyle) { if (style === 'short') delete segment.pathStyle; else segment.pathStyle = style; this.changed(); }
  get promptPreview(): {text:string; color?:string}[] {
    return this.segments.map(segment => ({text: segment.id === 'directory' ? (this.pathStyles.find(item => item.id === (segment.pathStyle ?? 'short'))?.sample ?? '') : (this.segmentOptions.find(option => option.id === segment.id)?.sample ?? ''), color: /^#[0-9a-fA-F]{6}$/.test(segment.color ?? '') ? segment.color : undefined}));
  }
  hasSegment(id:PromptSegmentId): boolean { return this.segments.some(s => s.id === id); }
  setPromptThreshold(segment:PromptSegment, value:number|null) { if (value === null) delete segment.thresholdMs; else segment.thresholdMs = value; this.changed(); }
  addSegment() { const prompt = this.config.prompt ??= {enabled:false,segments:[]}; if (!prompt.segments.some(s => s.id === this.newSegment)) prompt.segments.push({id:this.newSegment}); this.changed(); }
  moveSegment(index:number, delta:number) { const to = index + delta; if (to < 0 || to >= this.segments.length) return; [this.segments[index],this.segments[to]] = [this.segments[to],this.segments[index]]; this.changed(); }
  readonly behaviorShells: {id:Shell; label:string; view:boolean; note:string}[] = [
    {id:'pwsh', label:'PowerShell 7', view:true, note:''},
    {id:'powershell', label:'Windows PowerShell', view:true, note:'Le predizioni richiedono PSReadLine 2.1 o successivo (la 5.1 di serie include la 2.0). Il filtro della cronologia funziona anche con la 2.0.'},
    {id:'cmd', label:'Prompt dei comandi', view:true, note:'Usa Clink, incluso nell’app. Con «In linea» il primo suggerimento compare in grigio dopo il cursore; F2 apre comunque l’elenco.'},
  ];
  private readonly excludeDrafts: Partial<Record<Shell, string>> = {};
  behavior(shell: Shell): ShellBehavior | undefined { return this.config.shellBehavior?.[shell]; }
  setBehaviorEnabled(shell: Shell, enabled: boolean) {
    const all = this.config.shellBehavior ??= {};
    if (enabled) all[shell] = {predictions:'history', hideSensitive:true, predictionView:'inline', acceptKey:'tab'};
    else { delete all[shell]; delete this.excludeDrafts[shell]; }
    if (!Object.keys(all).length) delete this.config.shellBehavior;
    this.changed();
  }
  setBehavior<K extends keyof ShellBehavior>(shell: Shell, key: K, value: ShellBehavior[K]) {
    const behavior = this.behavior(shell);
    if (!behavior) return;
    behavior[key] = value;
    this.changed();
  }
  excludeText(shell: Shell): string { return this.excludeDrafts[shell] ?? (this.behavior(shell)?.historyExclude ?? []).join('\n'); }
  setExcludeText(shell: Shell, text: string) {
    const behavior = this.behavior(shell);
    if (!behavior) return;
    this.excludeDrafts[shell] = text;
    const patterns = text.split(/\r?\n/).map(line => line.trim()).filter(Boolean);
    if (patterns.length) behavior.historyExclude = patterns; else delete behavior.historyExclude;
    this.changed();
  }
  get appearanceIssueCount(): number { return this.issueCount('appearance') + this.issueCount('autocomplete'); }
  behaviorNote(shell: Shell): string { return this.behaviorShells.find(item => item.id === shell)?.note ?? ''; }
  get bannerEnabled(): boolean { return this.config.banner?.enabled !== false; }
  setBannerEnabled(enabled: boolean) { this.config.banner = {enabled}; this.changed(); }
  get workspaces(): Workspace[] { return this.config.workspaces ?? []; }
  get canInstallTeam(): boolean { return !this.busy && !this.loadFailed && !!this.teamPreview && this.teamSnapshot === JSON.stringify([this.config,this.teamContent]); }
  async selectTeamFile(event: Event) { const file = (event.target as HTMLInputElement).files?.[0]; this.teamPreview = null; this.teamContent = ''; this.teamFilename = file?.name ?? ''; if (!file) return; try { this.teamContent = await file.text(); } catch(e) { this.error = this.message(e); } finally { this.changeDetector.markForCheck(); } }
  async previewTeam() { if (!this.teamContent) return; this.busy = true; this.error = ''; this.teamPreview = null; try { const config = structuredClone(this.config); const content = this.teamContent; this.teamPreview = await this.bridge.previewTeam(config, content); this.teamSnapshot = JSON.stringify([config,content]); this.notice = 'Controlla le modifiche e conferma l’importazione.'; } catch(e) { this.error = this.message(e); } finally { this.finishAsync(); } }
  async installTeam() { if (!this.canInstallTeam) return; this.busy = true; this.error = ''; try { this.config = await this.bridge.installTeam(structuredClone(this.config), this.teamContent); this.savedSnapshot = JSON.stringify(this.config); this.teamPreview = null; this.notice = this.bridge.desktop ? 'Pacchetto importato. Salva per aggiornare il terminale.' : 'Importazione simulata: nessun file modificato.'; } catch(e) { this.error = this.message(e); } finally { this.finishAsync(); } }
  get canLaunch(): boolean { return this.bridge.desktop && !this.busy && !this.loadFailed && this.savedSnapshot === JSON.stringify(this.config); }
  async load() {
    this.busy = true;
    this.error = '';
    this.notice = '';
    this.backendErrorSection = undefined;
    try {
      this.config = await this.bridge.load();
      this.savedSnapshot = JSON.stringify(this.config);
      this.normalizeCommands();
      this.loadFailed = false;
      this.validationAttempted = false;
      this.issues = [];
      if (!this.bridge.desktop) this.notice = 'Modalità browser. Le modifiche sono simulate in questa sessione.';
      if (this.config.prompt?.enabled) void this.refreshToolStatus();
      void this.checkToolOffer();
    } catch (error) {
      this.loadFailed = true;
      this.error = this.message(error);
    } finally {
      this.finishAsync();
    }
  }
  private finishAsync() {
    this.busy = false;
    this.changeDetector.markForCheck();
  }
  message(e: unknown): string { return e instanceof Error ? e.message : typeof e === 'string' ? e : JSON.stringify(e); }
  changed() {
    this.notice = '';
    if (this.backendErrorSection) this.error = '';
    this.backendErrorSection = undefined;
    if (this.validationAttempted) this.issues = validateConfig(this.config);
  }
  detectedProfiles:Profile[] | null = null; private userSelectedProfile = false;
  ngOnInit() { void this.detectProfiles(); }
  async detectProfiles() { try { const profiles = await this.bridge.terminalProfiles(); this.detectedProfiles = profiles as Profile[]; if (!this.userSelectedProfile && !profiles.includes(this.profile) && profiles.length) this.selectProfile(profiles[0] as Profile); } catch { this.detectedProfiles = null; } finally { this.changeDetector.markForCheck(); } }
  readonly appearanceThemes = [
    {name:'Campbell', background:'#0c0c0c', foreground:'#cccccc', accent:'#3b78ff', green:'#16c60c'},
    {name:'Campbell Powershell', background:'#012456', foreground:'#cccccc', accent:'#61d6d6', green:'#16c60c'},
    {name:'One Half Dark', background:'#282c34', foreground:'#dcdfe4', accent:'#61afef', green:'#98c379'},
    {name:'One Half Light', background:'#fafafa', foreground:'#383a42', accent:'#0184bc', green:'#50a14f'},
    {name:'Solarized Dark', background:'#002b36', foreground:'#839496', accent:'#268bd2', green:'#859900'},
    {name:'Solarized Light', background:'#fdf6e3', foreground:'#657b83', accent:'#268bd2', green:'#859900'},
    {name:'Tango Dark', background:'#000000', foreground:'#d3d7cf', accent:'#729fcf', green:'#8ae234'},
    {name:'Tango Light', background:'#ffffff', foreground:'#555753', accent:'#3465a4', green:'#4e9a06'},
  ];
  readonly terminalFonts = ['Cascadia Code', 'Cascadia Mono', 'Consolas', 'Lucida Console', 'Courier New', 'Fira Code', 'JetBrains Mono', 'Source Code Pro', 'Ubuntu Mono', 'DejaVu Sans Mono'];
  private previewImages:Record<string,string|null> = {}; private previewPending = new Set<string>();
  uploadingBackground = false; customFont = false;
  ngDoCheck() { for (const appearance of Object.values(this.config.appearance ?? {})) { const path = appearance?.backgroundImage; if (path && !(path in this.previewImages) && !this.previewPending.has(path)) void this.resolveBackground(path); } }
  private async resolveBackground(path:string) { this.previewPending.add(path); try { this.previewImages[path] = await this.bridge.backgroundPreview(path); } catch { this.previewImages[path] = null; } finally { this.previewPending.delete(path); this.changeDetector.markForCheck(); } }
  async chooseBackground(event:Event) { const input = event.target as HTMLInputElement; const file = input.files?.[0]; if (!file) return; const target = this.profile; this.uploadingBackground = true; this.error = ''; try { const image = await this.bridge.uploadBackground(file); this.previewImages[image.path] = image.previewUrl; const savedProfile = this.profile; this.profile = target; this.setAppearance('backgroundImage', image.path); this.profile = savedProfile; } catch(e) { this.error = this.message(e); } finally { this.uploadingBackground = false; input.value = ''; this.changeDetector.markForCheck(); } }
  get backgroundPreviewUrl():string|null { const path = this.appearance.backgroundImage; return path ? this.previewImages[path] ?? null : null; }
  get backgroundPosition():string { const map:Record<string,string> = {center:'center',left:'left center',right:'right center',top:'center top',bottom:'center bottom',topLeft:'left top',topRight:'right top',bottomLeft:'left bottom',bottomRight:'right bottom'}; return map[this.appearance.backgroundImageAlignment ?? 'center']; }
  get selectedTheme() { return this.appearanceThemes.find(theme => theme.name === this.appearance.colorScheme) ?? this.appearanceThemes[0]; }
  get terminalBackground():string { return this.selectedTheme.background; }
  get terminalText():string { return this.selectedTheme.foreground; }
  get imageOpacityPercent(): number { return Math.round((this.appearance.backgroundImageOpacity ?? 1) * 100); }
  get fontSelection():string { const font = this.appearance.font?.face; return this.customFont || (font && !this.terminalFonts.includes(font)) ? '__custom__' : font ?? ''; }
  selectFont(value:string) { this.customFont = value === '__custom__'; if (!this.customFont) this.setFont('face', value); else if (this.terminalFonts.includes(this.appearance.font?.face ?? '')) this.setFont('face', ''); }
  selectProfile(profile:Profile) { this.userSelectedProfile = true; this.profile = profile; this.customFont = false; if (this.copyTarget === profile) this.copyTarget = this.profiles.find(p => p.id !== profile)!.id; }
  setAppearance<K extends keyof Appearance>(key: K, value: Appearance[K] | '') { const all = this.config.appearance ??= {}; const a = all[this.profile] ??= {}; if (value === '' || value === undefined || value === null) delete a[key]; else a[key] = value as Appearance[K]; this.changed(); }
  setFont(key: 'face'|'size', value: string|number|null) { const all = this.config.appearance ??= {}; const a = all[this.profile] ??= {}; const font = a.font ??= {}; if (value === '' || value === null) delete font[key]; else if(key === 'face') font.face = String(value); else font.size = Number(value); if (!Object.keys(font).length) delete a.font; this.changed(); }
  setPadding(key:'left'|'top'|'right'|'bottom', value:number|null) { const a = (this.config.appearance ??= {})[this.profile] ??= {}; const padding = a.padding ??= {}; if (value === null) delete padding[key]; else padding[key] = value; if (!Object.keys(padding).length) delete a.padding; this.changed(); }
  setDefault(p: import('./models').CommandParameter, value:string) { if (!value) delete p.default; else p.default = p.type === 'integer' ? Number(value) : value; this.changed(); }
  copyAppearance() {
    if (this.copyTarget === this.profile) return;
    (this.config.appearance ??= {})[this.copyTarget] = structuredClone(this.appearance);
    this.changed();
    this.notice = 'Aspetto copiato. Controlla l’anteprima prima di applicare.';
  }
  private get commandShell(): Shell {
    return this.profile === 'cmd' ? 'cmd'
      : this.detectedProfiles?.includes('pwsh') || !this.bridge.desktop ? 'pwsh' : 'powershell';
  }
  addCommand() {
    const commands = this.config.commands ??= [];
    if (!commands.length && !this.config.team) this.config.commandPrefix ??= '/';
    commands.push({id:`cmd-${crypto.randomUUID()}`,name:'',shells:[this.commandShell],template:{program:'',args:[]},parameters:[]});
    this.changed();
  }
  /** Avvio predefinito dei framework più usati. Angular parte con `npm start` (ng serve), gli altri con `npm run dev`. */
  readonly frameworks: Framework[] = [
    {id:'angular', label:'Angular', args:['start'], port:4200, extraExample:'--configuration=production'},
    {id:'react', label:'React', args:['run','dev'], port:5173, extraExample:'--host'},
    {id:'vue', label:'Vue', args:['run','dev'], port:5173, extraExample:'--host'},
    {id:'svelte', label:'Svelte', args:['run','dev'], port:5173, extraExample:'--host'},
    {id:'astro', label:'Astro', args:['run','dev'], port:4321, extraExample:'--host'},
    {id:'next', label:'Next.js', args:['run','dev'], port:3000, extraExample:'--turbopack'},
    {id:'nuxt', label:'Nuxt', args:['run','dev'], port:3000, extraExample:'--host'},
    {id:'vite', label:'Vite', args:['run','dev'], port:5173, extraExample:'--host'},
  ];
  frameworkDraft: FrameworkDraft | null = null;
  frameworkLine(framework: Framework): string { return ['npm', ...framework.args].join(' '); }
  openFramework(framework: Framework) {
    this.frameworkDraft = {framework, name: framework.id, port: '', extra: '', error: '', portError: '', extraError: ''};
    this.reveal('framework-name');
  }
  closeFramework() { this.frameworkDraft = null; }
  /** Argomenti finali: quelli del framework, poi `--` con porta e parametri, che npm inoltra allo script. */
  private frameworkArguments(draft: FrameworkDraft): {args: string[]} | {portError?: string; extraError?: string} {
    const port = draft.port.trim();
    if (port && (!/^\d{1,5}$/.test(port) || +port < 1 || +port > 65535)) {
      return {portError: 'Inserisci un numero di porta tra 1 e 65535 oppure lascia vuoto.'};
    }
    const extra = this.tokenize(draft.extra);
    if (extra === null) return {extraError: 'Chiudi le virgolette oppure toglile.'};
    // Le graffe sono dati, non segnaposto: vanno raddoppiate.
    const literal = extra.map(value => value.replaceAll('{', '{{').replaceAll('}', '}}'));
    const forwarded = [...(port ? ['--port', port] : []), ...literal];
    return {args: [...draft.framework.args, ...(forwarded.length ? ['--', ...forwarded] : [])]};
  }
  /** Comando completo che verrà salvato, mostrato mentre si compila il dialogo. */
  frameworkPreview(draft: FrameworkDraft): string {
    const composed = this.frameworkArguments(draft);
    const args = 'args' in composed ? composed.args : draft.framework.args;
    return ['npm', ...args].map(value => this.quoteToken(value)).join(' ');
  }
  createFrameworkCommand() {
    const draft = this.frameworkDraft;
    if (!draft) return;
    const name = draft.name.trim().replace(/^\//, '').toLowerCase();
    const shell = this.commandShell;
    draft.error = draft.portError = draft.extraError = '';
    if (!/^[a-z][a-z0-9-]{0,63}$/.test(name)) {
      draft.error = 'Inizia con una lettera minuscola e usa lettere minuscole, cifre o trattini, fino a 64 caratteri.';
    } else if (this.commands.some(command => command.name === name && command.shells.includes(shell))) {
      draft.error = 'Questo nome è già usato in ' + this.profileLabel(shell) + '. Scegline un altro.';
    }
    const composed = this.frameworkArguments(draft);
    if (!('args' in composed)) { draft.portError = composed.portError ?? ''; draft.extraError = composed.extraError ?? ''; }
    if (draft.error || draft.portError || draft.extraError || !('args' in composed)) return;
    const commands = this.config.commands ??= [];
    if (!commands.length && !this.config.team) this.config.commandPrefix ??= '/';
    commands.push({id:`cmd-${crypto.randomUUID()}`, name, shells:[shell], template:{program:'npm.cmd', args:composed.args}, description:'Avvia ' + draft.framework.label, parameters:[]});
    this.frameworkDraft = null;
    this.changed();
    this.notice = `Comando ${this.quickCommandPrefix}${name} creato: ${['npm', ...composed.args].map(value => this.quoteToken(value)).join(' ')}. Salva per usarlo.`;
  }
  get quickCommandPrefix(): string {
    return this.config.commandPrefix ?? (this.commands.length || this.config.team ? '' : '/');
  }
  private readonly commandDrafts = new WeakMap<QuickCommand, string>();
  commandLine(command: QuickCommand): string {
    if (command.changeDirectory !== undefined && !this.commandDrafts.has(command)) {
      return 'cd ' + this.quoteToken(command.changeDirectory);
    }
    return this.commandDrafts.get(command) ?? [command.template?.program ?? '', ...(command.template?.args ?? [])]
      .filter((value, index) => index !== 0 || value !== '')
      .map(value => this.quoteToken(value))
      .join(' ');
  }
  private quoteToken(value: string): string {
    return !value || /[\s"]/.test(value) ? '"' + value.replaceAll('"', '""') + '"' : value;
  }
  setCommandLine(command: QuickCommand, line: string) {
    this.commandDrafts.set(command, line);
    const [program = '', ...args] = this.tokenize(line) ?? [];
    this.assignInvocation(command, program, args);
    this.changed();
  }
  /** Divide una riga in argomenti; le virgolette raddoppiate dentro un argomento tra virgolette valgono una virgoletta. `null` se una virgoletta non è chiusa. */
  private tokenize(line: string): string[] | null {
    const input = line.trim();
    const tokenPattern = /("(?:""|[^"])*"|[^\s"]+)(?:\s+|$)/y;
    const tokens: string[] = [];
    let end = 0;
    while (end < input.length) {
      const match = tokenPattern.exec(input);
      if (!match) break;
      const token = match[1];
      tokens.push(token.startsWith('"') ? token.slice(1, -1).replaceAll('""', '"') : token);
      end = tokenPattern.lastIndex;
    }
    return end === input.length ? tokens : null;
  }
  /** «cd <cartella>» (anche «cd /d») sposta la shell corrente: non è un programma da avviare. */
  private assignInvocation(command: QuickCommand, program: string, args: string[]) {
    const [directory, ...extra] = /^cd$/i.test(program) && /^\/d$/i.test(args[0] ?? '') ? args.slice(1) : args;
    if (/^cd$/i.test(program) && directory !== undefined && !extra.length) {
      delete command.template;
      command.changeDirectory = directory;
    } else {
      delete command.changeDirectory;
      command.template = {program: /^npm$/i.test(program) ? 'npm.cmd' : program, args};
    }
  }
  /** Converte i vecchi comandi salvati come programma «cd» in veri cambi di cartella. */
  private normalizeCommands() {
    for (const command of this.config.commands ?? []) {
      if (command.template && /^cd$/i.test(command.template.program)) {
        this.assignInvocation(command, command.template.program, command.template.args);
      }
    }
  }
  setCommandName(command: QuickCommand, name: string) {
    command.name = name.trim().replace(/^\//, '').toLowerCase();
    this.changed();
  }
  removeCommand(i:number) { this.commands.splice(i,1); this.changed(); }
  setShell(c:QuickCommand, shell:Shell, checked:boolean) { c.shells = checked ? [...c.shells.filter(s => s !== shell),shell] : c.shells.filter(s => s !== shell); this.changed(); }
  addParameter(c:QuickCommand) { (c.parameters ??= []).push({name:'',type:'string',required:true}); this.changed(); }
  removeParameter(c:QuickCommand,i:number) { c.parameters?.splice(i,1); this.changed(); }
  private defaultPane(): Pane {
    const available = this.detectedProfiles;
    const profile = available?.includes(this.profile) || !this.bridge.desktop
      ? this.profile
      : available?.[0] ?? 'powershell';
    return {profile};
  }
  addWorkspace() { (this.config.workspaces ??= []).push({id:'',name:'',tabs:[{pane:this.defaultPane(),splits:[]}]}); this.changed(); }
  removeWorkspace(i:number) { this.workspaces.splice(i,1); this.changed(); }
  addTab(w:Workspace) { w.tabs.push({pane:this.defaultPane(),splits:[]}); this.changed(); }
  addSplit(tab:Workspace['tabs'][number]) { (tab.splits ??= []).push({direction:'right',pane:this.defaultPane()}); this.changed(); }
  setOptional(object: object, key: string, value: string) { const target = object as Record<string, unknown>; if(value === '') delete target[key]; else target[key] = value; this.changed(); }
  async launch(id:string) { if (!this.canLaunch) return; this.busy = true; this.error = ''; try { await this.bridge.launchWorkspace(id); this.notice = 'Workspace avviato.'; } catch(e) { this.error = this.message(e); } finally { this.finishAsync(); } }
  async removeIntegration() { if (!this.bridge.desktop || !window.confirm('Ripristinare i file gestiti da CLI Superpower per tutte le shell? Le preferenze salvate saranno conservate.')) return; this.busy = true; this.error = ''; try { await this.bridge.removeIntegration(); this.notice = 'Integrazione rimossa.'; } catch(e) { this.error = this.message(e); } finally { this.finishAsync(); } }
  async operation(kind: Operation) {
    if (this.busy || this.loadFailed) return;
    this.validationAttempted = true;
    this.issues = validateConfig(this.config);
    this.backendErrorSection = undefined;
    this.error = '';
    this.notice = '';
    const issue = this.issues[0];
    if (issue) {
      this.showToast('error', 'Errore: controlla i campi segnalati.');
      this.goToIssue(issue.section, issue.field);
      return;
    }
    this.busy = true;
    this.operationInProgress = kind;
    try {
      const config = structuredClone(this.config);
      await this.bridge.save(config);
      this.savedSnapshot = JSON.stringify(config);
      try { await this.bridge.apply(config); }
      catch (error) { throw 'Preferenze salvate, ma il terminale non è stato aggiornato. ' + this.message(error); }
      this.notice = this.bridge.desktop ? 'Configurazione salvata con successo. Apri una nuova finestra del terminale per vedere le modifiche.' : 'Configurazione salvata nella sessione del browser.';
      if (this.promptToolMissing) this.notice += ' Il prompt non comparirà finché non installi oh-my-posh da Strumenti.';
    } catch (error) {
      this.error = this.message(error);
      this.showToast('error', 'Errore durante il salvataggio: ' + this.error);
      this.backendErrorSection = sectionForError(this.error);
      if (this.backendErrorSection) this.goToIssue(this.backendErrorSection);
    } finally {
      this.operationInProgress = null;
      this.finishAsync();
    }
  }
}
