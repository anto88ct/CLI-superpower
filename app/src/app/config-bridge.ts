import { Injectable } from '@angular/core';
import { HistoryEntry, PersonalConfig, PreviewFile, Profile, TeamPreview, ToolStatus } from './models';

type Invoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
declare global { interface Window { __TAURI__?: { core: { invoke: Invoke } }; } }

@Injectable({providedIn: 'root'})
export class ConfigBridge {
  readonly desktop = !!window.__TAURI__?.core?.invoke;
  private mock: PersonalConfig = {schemaVersion: 6};
  private readonly mockBackgrounds = new Map<string, string>();

  async terminalProfiles(): Promise<Profile[]> {
    return this.desktop
      ? window.__TAURI__!.core.invoke<Profile[]>('terminal_profiles')
      : ['pwsh', 'powershell', 'cmd'];
  }

  async uploadBackground(file: File): Promise<{path: string; previewUrl: string}> {
    if (file.size > 8 * 1024 * 1024) throw new Error('L’immagine supera il limite di 8 MB.');
    if (!/\.(png|jpe?g|gif|bmp|webp)$/i.test(file.name)) {
      throw new Error('Scegli un’immagine PNG, JPEG, GIF, BMP o WebP.');
    }
    try {
      const image = await createImageBitmap(file);
      image.close();
    } catch {
      throw new Error('Il file non contiene un’immagine leggibile.');
    }
    const previewUrl = await new Promise<string>((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result));
      reader.onerror = () => reject(new Error('Impossibile leggere l’immagine.'));
      reader.readAsDataURL(file);
    });
    if (this.desktop) {
      const data = previewUrl.split(',', 2)[1];
      const path = await window.__TAURI__!.core.invoke<string>('upload_background', {data});
      return {path, previewUrl};
    }
    const path = 'C:\\CLI-Superpower\\' + crypto.randomUUID() + '-' + file.name;
    this.mockBackgrounds.set(path, previewUrl);
    return {path, previewUrl};
  }

  async backgroundPreview(path: string): Promise<string | null> {
    return this.desktop
      ? window.__TAURI__!.core.invoke<string | null>('background_preview', {path})
      : this.mockBackgrounds.get(path) ?? null;
  }
  async load(): Promise<PersonalConfig> {
    return this.desktop ? window.__TAURI__!.core.invoke<PersonalConfig>('load_config') : structuredClone(this.mock);
  }
  async save(config: PersonalConfig): Promise<void> {
    if (this.desktop) await window.__TAURI__!.core.invoke('save_config', {config});
    else this.mock = structuredClone(config);
  }
  async preview(config: PersonalConfig): Promise<PreviewFile[]> {
    if (this.desktop) return window.__TAURI__!.core.invoke<PreviewFile[]>('preview_config', {config});
    return [{path: 'Simulazione browser / configurazione personale', content: JSON.stringify(config, null, 2)}];
  }
  async previewTeam(config: PersonalConfig, content: string): Promise<TeamPreview> {
    if (this.desktop) return window.__TAURI__!.core.invoke<TeamPreview>('preview_team_package', {config, content});
    const pack = JSON.parse(content) as {schemaVersion?:number; id?:string; version?:number};
    if (![1, 2, 3, 4, 5, 6].includes(pack.schemaVersion ?? 0) || !pack.id || !Number.isInteger(pack.version) || pack.version! < 1) throw new Error('Pacchetto non valido: servono schemaVersion da 1 a 6, id e version.');
    const result = structuredClone(config); result.team = {packageId:pack.id, installedVersion:pack.version!};
    return {config:result, changes:['Pacchetto ' + pack.id + ' · versione ' + pack.version], files:[{path:'Simulazione browser / pacchetto team',content}]};
  }
  async installTeam(config: PersonalConfig, content:string): Promise<PersonalConfig> {
    if (this.desktop) return window.__TAURI__!.core.invoke<PersonalConfig>('install_team_package', {config, content});
    const result = (await this.previewTeam(config, content)).config; this.mock = structuredClone(result); return result;
  }
  async listHistory(): Promise<HistoryEntry[]> {
    if (this.desktop) return window.__TAURI__!.core.invoke<HistoryEntry[]>('list_history');
    return [];
  }
  async restoreLatest(): Promise<void> {
    if (!this.desktop) throw new Error('Il ripristino è disponibile solo nell’app desktop.');
    await window.__TAURI__!.core.invoke('restore_latest');
  }
  async listTools(): Promise<ToolStatus[]> {
    if (this.desktop) return window.__TAURI__!.core.invoke<ToolStatus[]>('list_tools');
    return [{id:'pwsh',name:'PowerShell 7',installed:false,version:null,manualUrl:'https://learn.microsoft.com/powershell/scripting/install/installing-powershell-on-windows',canInstall:false}, {id:'starship',name:'Starship',installed:false,version:null,manualUrl:'https://starship.rs/',canInstall:false}];
  }
  async installTool(id:string): Promise<void> {
    if (!this.desktop) throw new Error('L’installazione è disponibile solo nell’app desktop.');
    await window.__TAURI__!.core.invoke('install_tool', {id});
  }
  async uninstallTool(id:string): Promise<void> {
    if (!this.desktop) throw new Error('La disinstallazione è disponibile solo nell’app desktop.');
    await window.__TAURI__!.core.invoke('uninstall_tool', {id});
  }
  async installRecommendedTools(): Promise<string[]> {
    if (!this.desktop) throw new Error('L’installazione è disponibile solo nell’app desktop.');
    return window.__TAURI__!.core.invoke<string[]>('install_recommended_tools');
  }
  async launchWorkspace(id: string): Promise<void> {
    if (!this.desktop) throw new Error('L’avvio dei workspace è disponibile solo nell’app desktop.');
    await window.__TAURI__!.core.invoke('launch_workspace', {id});
  }
  openLink(target: 'github' | 'website'): void {
    if (this.desktop) void window.__TAURI__!.core.invoke('open_link', {target});
    else window.open(target === 'github' ? 'https://github.com/anto88ct/CLI-superpower' : 'https://anto88ct.github.io/CLI-superpower', '_blank', 'noopener');
  }
  async removeIntegration(): Promise<void> {
    if (!this.desktop) throw new Error('Il ripristino è disponibile solo nell’app desktop.');
    await window.__TAURI__!.core.invoke('remove_integration');
  }
  async apply(config: PersonalConfig): Promise<PreviewFile[]> {
    if (this.desktop) return window.__TAURI__!.core.invoke<PreviewFile[]>('apply_config', {config});
    this.mock = structuredClone(config);
    return this.preview(config);
  }
}
