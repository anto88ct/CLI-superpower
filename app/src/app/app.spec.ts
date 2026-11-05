import { ComponentFixture, TestBed } from '@angular/core/testing';
import { App } from './app';
import { ConfigBridge } from './config-bridge';
import { PersonalConfig } from './models';

describe('Configurazione', () => {
  let app: App;
  let fixture: ComponentFixture<App>;
  let bridge: { desktop:boolean; load: ReturnType<typeof vi.fn>; save: ReturnType<typeof vi.fn>; preview: ReturnType<typeof vi.fn>; apply: ReturnType<typeof vi.fn>; launchWorkspace: ReturnType<typeof vi.fn>; removeIntegration: ReturnType<typeof vi.fn>; previewTeam: ReturnType<typeof vi.fn>; installTeam: ReturnType<typeof vi.fn>; listTools: ReturnType<typeof vi.fn>; installTool: ReturnType<typeof vi.fn>; listHistory: ReturnType<typeof vi.fn>; restoreLatest: ReturnType<typeof vi.fn> };
  beforeEach(async () => {
    bridge = {desktop:true, load:vi.fn().mockResolvedValue({schemaVersion:1, prompt:{enabled:false,segments:[]}}), save:vi.fn().mockResolvedValue(undefined), preview:vi.fn().mockResolvedValue([{path:'file',content:'contenuto'}]), apply:vi.fn().mockResolvedValue([]), launchWorkspace:vi.fn().mockResolvedValue(undefined), removeIntegration:vi.fn().mockResolvedValue(undefined), previewTeam:vi.fn().mockResolvedValue({config:{schemaVersion:1},changes:['Comandi'],files:[]}), installTeam:vi.fn().mockResolvedValue({schemaVersion:1,team:{packageId:'dev',installedVersion:1}}), listTools:vi.fn().mockResolvedValue([{id:'pwsh',name:'PowerShell',installed:false,version:null,manualUrl:'https://example.com',canInstall:true}]), installTool:vi.fn().mockResolvedValue(undefined), listHistory:vi.fn().mockResolvedValue([{id:'backup',timestamp:1,fileCount:2,canRestore:true}]), restoreLatest:vi.fn().mockResolvedValue(undefined)};
    localStorage.setItem('csp.walkthrough', 'done');
    await TestBed.configureTestingModule({imports:[App], providers:[{provide:ConfigBridge,useValue:bridge}]}).compileComponents();
    fixture = TestBed.createComponent(App); app = fixture.componentInstance; await fixture.whenStable();
  });
  const button = (label: string): HTMLButtonElement => {
    const host = fixture.nativeElement as HTMLElement;
    return [...host.querySelectorAll('button')].find(button => button.textContent.includes(label))!;
  };
  it('Salva aggiorna subito il terminale', async () => {
    app.setAppearance('opacity', 90); expect(app.dirty).toBe(true);
    await app.operation('save');
    expect(bridge.save).toHaveBeenCalledOnce(); expect(bridge.apply).toHaveBeenCalledOnce();
    expect((bridge.apply.mock.calls[0][0] as PersonalConfig).appearance?.pwsh?.opacity).toBe(90);
    expect(app.dirty).toBe(false); expect(app.notice).toContain('nuova finestra');
  });
  it('conserva le preferenze se il terminale non si aggiorna', async () => {
    bridge.apply.mockRejectedValue('Conflitto: profilo');
    app.setAppearance('opacity', 90); await app.operation('save');
    expect(bridge.save).toHaveBeenCalledOnce(); expect(app.dirty).toBe(false);
    expect(app.error).toContain('Preferenze salvate'); expect(app.error).toContain('Conflitto: profilo');
  });
  it('dalla home apre l’aspetto della shell scelta', async () => {
    expect(app.section).toBe('home');
    app.openProfile('cmd');
    expect(app.section).toBe('appearance'); expect(app.profile).toBe('cmd');
  });
  it('mostra nella home la prima scorciatoia di ogni shell', () => {
    app.config.commandPrefix = '/';
    app.config.commands = [{id:'a',name:'run',shells:['cmd'],template:{program:'npm.cmd',args:['run','dev']},parameters:[]}];
    expect(app.shellCommands('cmd').map(command => command.name)).toEqual(['run']);
    expect(app.shellCommands('pwsh')).toEqual([]);
  });
  it('conserva le sezioni non modificate nel salvataggio', async () => {
    app.setFont('size', 14); await app.operation('save');
    const saved = bridge.save.mock.calls[0][0] as PersonalConfig;
    expect(saved.prompt).toEqual({enabled:false,segments:[]}); expect(saved.appearance?.pwsh?.font?.size).toBe(14);
  });
  it('copia l’aspetto senza condividere gli oggetti', () => {
    app.setFont('face','Cascadia Mono'); app.copyAppearance(); app.setFont('face','Altro');
    expect(app.config.appearance?.powershell?.font?.face).toBe('Cascadia Mono');
  });
  it('consente l’avvio solo dopo il salvataggio delle modifiche', async () => {
    app.addWorkspace(); expect(app.canLaunch).toBe(false);
    app.workspaces[0].id = 'demo';
    app.workspaces[0].name = 'Demo';
    await app.operation('save'); expect(app.canLaunch).toBe(true);
    await app.launch('demo'); expect(bridge.launchWorkspace).toHaveBeenCalledWith('demo');
    app.workspaces[0].name = 'Modificato'; expect(app.canLaunch).toBe(false);
  });
  it('mantiene ordine e opzioni dei segmenti prompt', async () => {
    app.toggleSegment('git', true); app.toggleSegment('duration', true);
    app.setPromptThreshold(app.segments[1], 3000); app.moveSegment(1, -1); await app.operation('save');
    expect((bridge.save.mock.calls[0][0] as PersonalConfig).prompt).toEqual({enabled:true,segments:[{id:'duration',thresholdMs:3000},{id:'git'}]});
  });
  it('accende il prompt scegliendo la prima informazione e lo spegne togliendo l’ultima', () => {
    expect(app.config.prompt?.enabled).toBe(false);
    expect(app.promptRows.map(row => row.option.id)).toEqual(['directory', 'git', 'node', 'dotnet', 'duration', 'status']);
    app.toggleSegment('status', true);
    expect(app.config.prompt).toEqual({enabled: true, segments: [{id: 'status'}]});
    app.toggleSegment('git', true);
    expect(app.promptRows.map(row => row.option.id).slice(0, 2)).toEqual(['status', 'git']);
    expect(app.promptPreview.map(part => part.text)).toEqual(['✓', 'main']);
    app.toggleSegment('status', false);
    expect(app.config.prompt?.enabled).toBe(true);
    app.toggleSegment('git', false);
    expect(app.config.prompt).toEqual({enabled: false, segments: []});
    app.setPromptEnabled(true);
    expect(app.config.prompt).toEqual({enabled: true, segments: [{id: 'directory'}]});
  });
  it('sceglie come mostrare il percorso e lo riflette nell’anteprima', () => {
    app.toggleSegment('directory', true);
    const directory = app.segments[0];
    expect(app.promptPreview[0].text).toBe(String.raw`~\..\Desktop`);
    app.setPathStyle(directory, 'full');
    expect(directory.pathStyle).toBe('full');
    expect(app.promptPreview[0].text).toBe(String.raw`C:\Users\nome\OneDrive\Desktop`);
    app.setPathStyle(directory, 'folder');
    expect(app.promptPreview[0].text).toBe('Desktop');
    app.setPathStyle(directory, 'short');
    expect(directory.pathStyle).toBeUndefined();
  });
  it('mostra il colore nel selettore e salva quello scelto in maiuscolo', () => {
    app.toggleSegment('directory', true);
    app.toggleSegment('status', true);
    const [directory, status] = app.segments;
    expect(app.swatchColor(directory)).toBe('#d8dee9');
    expect(app.swatchColor(status)).toBe('#f87171');
    app.pickColor(directory, '#7aa2f7');
    expect(directory.color).toBe('#7AA2F7');
    expect(app.swatchColor(directory)).toBe('#7aa2f7');
    expect(app.promptPreview[0].color).toBe('#7AA2F7');
    directory.color = 'blu';
    expect(app.swatchColor(directory)).toBe('#d8dee9');
    app.setOptional(directory, 'color', '');
    expect(directory.color).toBeUndefined();
  });
  it('richiede una nuova anteprima del pacchetto dopo modifiche', async () => {
    app.teamContent = '{}'; await app.previewTeam(); expect(app.canInstallTeam).toBe(true);
    app.setAppearance('opacity', 90); expect(app.canInstallTeam).toBe(false); await app.installTeam(); expect(bridge.installTeam).not.toHaveBeenCalled();
    await app.previewTeam(); await app.installTeam(); expect(app.config.team?.packageId).toBe('dev');
  });
  it('aggiorna lo stato dopo l’installazione di uno strumento', async () => {
    await app.openTools(); await app.installTool('pwsh'); expect(bridge.installTool).toHaveBeenCalledWith('pwsh'); expect(bridge.listTools).toHaveBeenCalledTimes(2);
  });
  it('ricarica la configurazione dopo conferma del ripristino', async () => {
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(true); await app.openHistory(); await app.restoreLatest();
    expect(bridge.restoreLatest).toHaveBeenCalledOnce(); expect(bridge.load).toHaveBeenCalledTimes(2); confirm.mockRestore();
  });
  it('mostra gli errori se il salvataggio fallisce', async () => {
    bridge.save.mockRejectedValue('Configurazione non valida'); await app.operation('save');
    expect(app.error).toBe('Configurazione non valida');
  });
  it('segnala i campi obbligatori e porta il focus al primo errore', async () => {
    button('Comandi rapidi').click();
    await fixture.whenStable();
    button('Nuovo comando').click();
    await fixture.whenStable();
    button('Salva').click();
    await fixture.whenStable();
    const host = fixture.nativeElement as HTMLElement;
    expect(bridge.save).not.toHaveBeenCalled();
    expect(host.querySelector('#command-0-name')?.getAttribute('aria-invalid')).toBe('true');
    expect(host.querySelector('#issue-command-0-name')?.textContent).toContain('lettera minuscola');
    expect(document.activeElement?.id).toBe('command-0-name');
  });
  it('salva una scorciatoia npm con prefisso predefinito e ID automatico', async () => {
    app.addCommand();
    const command = app.commands[0];
    app.setCommandName(command, '/rundev');
    app.setCommandLine(command, 'npm run dev');
    await app.operation('save');
    expect(bridge.save).toHaveBeenCalledWith(expect.objectContaining({
      commandPrefix: '/',
      commands: [expect.objectContaining({
        id: expect.stringMatching(/^cmd-[a-f0-9-]+$/),
        name: 'rundev',
        template: {program: 'npm.cmd', args: ['run', 'dev']},
      })],
    }));
  });
  it('aggiorna il DOM durante e dopo una risposta IPC asincrona di salvataggio', async () => {
    let reply!: () => void;
    bridge.save.mockImplementation(() => new Promise<void>(resolve => { reply = resolve; }));
    button('Salva').click();
    await fixture.whenStable();
    expect(button('Salvataggio…').disabled).toBe(true);
    reply();
    await vi.waitFor(() => { fixture.detectChanges(); expect(button('Salva').disabled).toBe(false); });
  });
  it('riconosce «cd <cartella>» come cambio cartella e non come programma', async () => {
    app.addCommand();
    const command = app.commands[0];
    app.setCommandName(command, 'progetti');
    app.setCommandLine(command, String.raw`cd C:\Users\anton\OneDrive\Desktop\PROGETTI`);
    expect(command.changeDirectory).toBe(String.raw`C:\Users\anton\OneDrive\Desktop\PROGETTI`);
    expect(command.template).toBeUndefined();
    app.setCommandLine(command, String.raw`cd /d "C:\Mie cartelle"`);
    expect(command.changeDirectory).toBe(String.raw`C:\Mie cartelle`);
    app.setCommandLine(command, 'cd');
    expect(command.changeDirectory).toBeUndefined();
    expect(command.template).toEqual({program: 'cd', args: []});
    app.setCommandLine(command, 'npm run dev');
    expect(command.template).toEqual({program: 'npm.cmd', args: ['run', 'dev']});
    app.setCommandLine(command, String.raw`cd C:\a b`);
    expect(command.changeDirectory).toBeUndefined();
  });
  it('converte al caricamento un vecchio comando salvato come programma cd', async () => {
    bridge.load.mockResolvedValue({schemaVersion:1, commandPrefix:'/', commands:[{id:'a', name:'cdprog', shells:['cmd'], template:{program:'cd', args:[String.raw`C:\Progetti`]}, parameters:[]}]});
    await app.load();
    expect(app.commands[0].changeDirectory).toBe(String.raw`C:\Progetti`);
    expect(app.commands[0].template).toBeUndefined();
    expect(app.dirty).toBe(true);
  });
  it('attiva e disattiva l’autocomplete per una shell con valori predefiniti sicuri', () => {
    app.setBehaviorEnabled('pwsh', true);
    expect(app.config.shellBehavior?.pwsh).toEqual({predictions:'history', hideSensitive:true, predictionView:'inline', acceptKey:'tab'});
    app.setBehaviorEnabled('cmd', true);
    expect(app.config.shellBehavior?.cmd).toEqual({predictions:'history', hideSensitive:true, predictionView:'inline', acceptKey:'tab'});
    app.setBehavior('pwsh', 'predictionView', 'list');
    expect(app.config.shellBehavior?.pwsh?.predictionView).toBe('list');
    app.setBehaviorEnabled('pwsh', false); app.setBehaviorEnabled('cmd', false);
    expect(app.config.shellBehavior).toBeUndefined();
  });
  it('trasforma le righe da escludere in elenco e consente di digitare a capo', () => {
    app.setBehaviorEnabled('pwsh', true);
    app.setExcludeText('pwsh', ' alfa \n\nbeta\n');
    expect(app.config.shellBehavior?.pwsh?.historyExclude).toEqual(['alfa', 'beta']);
    expect(app.excludeText('pwsh')).toBe(' alfa \n\nbeta\n');
    app.setExcludeText('pwsh', '');
    expect(app.config.shellBehavior?.pwsh?.historyExclude).toBeUndefined();
  });
  it('segnala troppi frammenti da escludere nella pagina Autocomplete', async () => {
    app.setBehaviorEnabled('pwsh', true);
    app.config.shellBehavior!.pwsh!.historyExclude = Array.from({length: 51}, (_, i) => 'x' + i);
    await app.operation('save');
    expect(bridge.save).not.toHaveBeenCalled();
    expect(app.issues[0]).toMatchObject({section:'autocomplete', field:'behavior-pwsh-exclude'});
  });
  it('il banner è attivo di default e si disattiva dalle impostazioni', async () => {
    expect(app.bannerEnabled).toBe(true);
    app.setBannerEnabled(false);
    expect(app.bannerEnabled).toBe(false);
    await app.operation('save');
    expect(bridge.save).toHaveBeenCalledWith(expect.objectContaining({banner: {enabled: false}}));
    app.setBannerEnabled(true);
    expect(app.config.banner).toEqual({enabled: true});
  });
  it('crea un comando dal framework scelto chiedendo il nome', async () => {
    const angular = app.frameworks.find(f => f.id === 'angular')!;
    app.openFramework(angular);
    expect(app.frameworkDraft?.name).toBe('angular');
    app.frameworkDraft!.name = 'Mio Sito';
    app.createFrameworkCommand();
    expect(app.frameworkDraft?.error).toContain('lettera minuscola');
    expect(app.commands).toHaveLength(0);
    app.frameworkDraft!.name = '/ng';
    app.createFrameworkCommand();
    expect(app.frameworkDraft).toBeNull();
    expect(app.commands[0]).toMatchObject({name:'ng', shells:[expect.stringMatching(/^(pwsh|powershell)$/)], template:{program:'npm.cmd', args:['start']}});
    expect(app.config.commandPrefix).toBe('/');
    expect(app.notice).toContain('/ng');
    app.openFramework(angular);
    app.frameworkDraft!.name = 'ng';
    app.createFrameworkCommand();
    expect(app.frameworkDraft?.error).toContain('già usato');
    expect(app.commands).toHaveLength(1);
    app.closeFramework();
    app.openFramework(app.frameworks.find(f => f.id === 'astro')!);
    app.createFrameworkCommand();
    expect(app.commands[1]).toMatchObject({name:'astro', template:{program:'npm.cmd', args:['run','dev']}});
  });
  it('compone porta e parametri nel comando del framework', () => {
    const angular = app.frameworks.find(f => f.id === 'angular')!;
    app.openFramework(angular);
    app.frameworkDraft!.name = 'angular';
    app.frameworkDraft!.port = '4200';
    app.frameworkDraft!.extra = '--configuration=prod --host-disabled';
    expect(app.frameworkPreview(app.frameworkDraft!)).toBe('npm start -- --port 4200 --configuration=prod --host-disabled');
    app.createFrameworkCommand();
    expect(app.commands[0].template).toEqual({
      program: 'npm.cmd',
      args: ['start', '--', '--port', '4200', '--configuration=prod', '--host-disabled'],
    });
    expect(app.notice).toContain('npm start -- --port 4200');
  });
  it('convalida porta e parametri del framework senza creare il comando', () => {
    app.openFramework(app.frameworks.find(f => f.id === 'vite')!);
    app.frameworkDraft!.port = '70000';
    app.createFrameworkCommand();
    expect(app.frameworkDraft?.portError).toContain('1 e 65535');
    app.frameworkDraft!.port = 'abc';
    app.createFrameworkCommand();
    expect(app.frameworkDraft?.portError).toContain('1 e 65535');
    app.frameworkDraft!.port = '';
    app.frameworkDraft!.extra = '--title "aperta';
    app.createFrameworkCommand();
    expect(app.frameworkDraft?.extraError).toContain('virgolette');
    expect(app.commands).toHaveLength(0);
    app.frameworkDraft!.extra = '--title "Mio {sito}"';
    app.createFrameworkCommand();
    expect(app.commands[0].template?.args).toEqual(['run', 'dev', '--', '--title', 'Mio {{sito}}']);
    app.openFramework(app.frameworks.find(f => f.id === 'vue')!);
    expect(app.frameworkPreview(app.frameworkDraft!)).toBe('npm run dev');
  });
  it('avvisa se il prompt è attivo ma oh-my-posh non è installato', async () => {
    expect(app.promptToolMissing).toBe(false);
    bridge.listTools.mockResolvedValue([{id:'oh-my-posh', name:'oh-my-posh', installed:false, version:null, manualUrl:'', canInstall:true}]);
    app.invalidateTools();
    app.openPrompt();
    app.setPromptEnabled(true);
    app.addSegment();
    await vi.waitFor(() => expect(app.promptToolMissing).toBe(true));
    await app.operation('save');
    expect(app.notice).toContain('oh-my-posh');
    bridge.listTools.mockResolvedValue([{id:'oh-my-posh', name:'oh-my-posh', installed:true, version:'1', manualUrl:'', canInstall:true}]);
    await app.installTool('oh-my-posh');
    expect(app.promptToolMissing).toBe(false);
  });
  it('propone gli strumenti consigliati una volta e ricorda il rifiuto', async () => {
    const recommended = {id:'oh-my-posh', name:'Oh My Posh', installed:false, version:null, manualUrl:'', canInstall:true, recommended:true};
    bridge.listTools.mockResolvedValue([recommended, {id:'pwsh', name:'PowerShell 7', installed:false, version:null, manualUrl:'', canInstall:true}]);
    (bridge as unknown as {installRecommendedTools: ReturnType<typeof vi.fn>}).installRecommendedTools = vi.fn().mockResolvedValue(['Oh My Posh']);
    localStorage.removeItem('csp.toolsOffer');
    app.invalidateTools();
    await app.load();
    await vi.waitFor(() => expect(app.toolOffer?.map(tool => tool.id)).toEqual(['oh-my-posh']));
    await app.acceptToolOffer();
    expect(app.toolOffer).toBeNull();
    expect(app.notice).toContain('Oh My Posh');
    app.invalidateTools();
    await app.load();
    await vi.waitFor(() => expect(app.toolOffer).not.toBeNull());
    app.declineToolOffer();
    expect(app.toolOffer).toBeNull();
    app.invalidateTools();
    await app.load();
    await new Promise(resolve => setTimeout(resolve));
    expect(app.toolOffer).toBeNull();
    localStorage.removeItem('csp.toolsOffer');
  });
  it('disinstalla solo gli strumenti installati dall’app, dopo conferma', async () => {
    (bridge as unknown as {uninstallTool: ReturnType<typeof vi.fn>}).uninstallTool = vi.fn().mockResolvedValue(undefined);
    bridge.listTools.mockResolvedValue([
      {id:'oh-my-posh', name:'Oh My Posh', installed:true, version:'1', manualUrl:'', canInstall:false, managed:true, canUninstall:true},
      {id:'jq', name:'jq', installed:true, version:'1', manualUrl:'', canInstall:false, managed:false, canUninstall:false},
    ]);
    app.invalidateTools();
    await app.openTools();
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);
    await app.uninstallTool('oh-my-posh');
    expect((bridge as unknown as {uninstallTool: ReturnType<typeof vi.fn>}).uninstallTool).not.toHaveBeenCalled();
    confirm.mockReturnValue(true);
    await app.uninstallTool('jq');
    expect((bridge as unknown as {uninstallTool: ReturnType<typeof vi.fn>}).uninstallTool).not.toHaveBeenCalled();
    await app.uninstallTool('oh-my-posh');
    expect((bridge as unknown as {uninstallTool: ReturnType<typeof vi.fn>}).uninstallTool).toHaveBeenCalledWith('oh-my-posh');
    confirm.mockRestore();
  });
  it('mostra il loader mentre il salvataggio è in corso', async () => {
    let finish!: () => void;
    bridge.save.mockImplementation(() => new Promise<void>(resolve => { finish = resolve; }));
    const host = fixture.nativeElement as HTMLElement;
    expect(host.querySelector('.save-loader')).toBeNull();
    button('Salva').click();
    await fixture.whenStable();
    expect(host.querySelector('.save-loader')?.textContent).toContain('Salvataggio in corso');
    expect(host.querySelector('.action-bar .spinner')).not.toBeNull();
    finish();
    await vi.waitFor(() => { fixture.detectChanges(); expect(host.querySelector('.save-loader')).toBeNull(); });
  });
  it('riusa lo stato degli strumenti in memoria e lo rinnova solo quando serve', async () => {
    const tool = (installed: boolean) => [{id:'oh-my-posh', name:'Oh My Posh', installed, version:null, manualUrl:'', canInstall:!installed, managed:installed, canUninstall:installed}];
    bridge.listTools.mockResolvedValue(tool(false));
    await app.load();
    await vi.waitFor(() => expect(bridge.listTools).toHaveBeenCalledTimes(1));
    bridge.listTools.mockClear();
    // aprire la pagina più volte non rilegge nulla
    await app.openTools(); await app.openTools(); app.openPrompt(); await app.openTools();
    expect(bridge.listTools).not.toHaveBeenCalled();
    // «Aggiorna» forza la lettura
    await app.refreshTools();
    expect(bridge.listTools).toHaveBeenCalledTimes(1);
    // installare invalida la copia
    bridge.listTools.mockResolvedValue(tool(true));
    await app.installTool('oh-my-posh');
    expect(bridge.listTools).toHaveBeenCalledTimes(2);
    expect(app.toolList?.[0].installed).toBe(true);
    // disinstallare la invalida di nuovo
    bridge.listTools.mockResolvedValue(tool(false));
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(true);
    (bridge as unknown as {uninstallTool: ReturnType<typeof vi.fn>}).uninstallTool = vi.fn().mockResolvedValue(undefined);
    await app.uninstallTool('oh-my-posh');
    confirm.mockRestore();
    expect(bridge.listTools).toHaveBeenCalledTimes(3);
    expect(app.toolList?.[0].installed).toBe(false);
    // scaduta la copia, riaprire la pagina rilegge
    const now = Date.now();
    const clock = vi.spyOn(Date, 'now').mockReturnValue(now + 11 * 60 * 1000);
    await app.openTools();
    clock.mockRestore();
    expect(bridge.listTools).toHaveBeenCalledTimes(4);
  });
  it('condivide una sola lettura tra richieste contemporanee', async () => {
    let answer!: (tools: unknown[]) => void;
    bridge.listTools.mockClear();
    bridge.listTools.mockImplementation(() => new Promise(resolve => { answer = resolve as (tools: unknown[]) => void; }));
    (app as unknown as {toolList: null}).toolList = null;
    const first = app.refreshTools(false);
    const second = app.refreshTools(false);
    expect(bridge.listTools).toHaveBeenCalledTimes(1);
    answer([]);
    await Promise.all([first, second]);
    expect(bridge.listTools).toHaveBeenCalledTimes(1);
  });
  it('salva un comando cd con changeDirectory e senza template', async () => {
    app.addCommand();
    const command = app.commands[0];
    app.setCommandName(command, 'progetti');
    app.setCommandLine(command, String.raw`cd C:\Progetti`);
    await app.operation('save');
    expect(bridge.save).toHaveBeenCalledWith(expect.objectContaining({
      commands: [expect.objectContaining({name: 'progetti', changeDirectory: String.raw`C:\Progetti`})],
    }));
    expect(bridge.save.mock.calls[0][0].commands[0].template).toBeUndefined();
  });
  it('segnala una cartella cd non assoluta', async () => {
    app.addCommand();
    const command = app.commands[0];
    app.setCommandName(command, 'progetti');
    app.setCommandLine(command, 'cd relativa');
    await app.operation('save');
    expect(bridge.save).not.toHaveBeenCalled();
    expect(app.issues[0].field).toBe('command-0-program');
  });
  it('consente di riprovare un caricamento non riuscito', async () => {
    bridge.load.mockRejectedValueOnce('disco occupato');
    await app.load();
    await fixture.whenStable();
    expect(button('Salva').disabled).toBe(true);
    button('Riprova caricamento').click();
    await fixture.whenStable();
    expect(button('Salva').disabled).toBe(false);
    expect((fixture.nativeElement as HTMLElement).textContent).toContain('Tutto salvato');
  });
});

describe('Guida iniziale', () => {
  const bridge = {desktop:true, load:vi.fn().mockResolvedValue({schemaVersion:1}), terminalProfiles:vi.fn().mockResolvedValue(['pwsh','powershell','cmd'])};
  const create = async () => {
    TestBed.resetTestingModule();
    await TestBed.configureTestingModule({imports:[App], providers:[{provide:ConfigBridge,useValue:bridge}]}).compileComponents();
    const fixture = TestBed.createComponent(App); await fixture.whenStable(); return fixture;
  };
  it('compare al primo avvio e non torna dopo la chiusura', async () => {
    localStorage.removeItem('csp.walkthrough');
    let fixture = await create(); let app = fixture.componentInstance;
    expect(app.walkthroughStep).toBe(0);
    const dialog = (fixture.nativeElement as HTMLElement).querySelector('[role=dialog]');
    expect(dialog?.getAttribute('aria-modal')).toBe('true');
    app.nextWalkthroughStep(); app.nextWalkthroughStep(); expect(app.walkthroughStep).toBe(2);
    app.nextWalkthroughStep(); expect(app.walkthroughStep).toBeNull();
    fixture = await create(); app = fixture.componentInstance;
    expect(app.walkthroughStep).toBeNull();
    app.openWalkthrough(); expect(app.walkthroughStep).toBe(0);
  });
});
