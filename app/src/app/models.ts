export type Profile = 'pwsh' | 'powershell' | 'cmd';
export type Shell = Profile;
export interface Appearance { colorScheme?: string; font?: { face?: string; size?: number }; backgroundImage?: string; backgroundImageOpacity?: number; backgroundImageAlignment?: 'center'|'left'|'top'|'right'|'bottom'|'topLeft'|'topRight'|'bottomLeft'|'bottomRight'; opacity?: number; useAcrylic?: boolean; cursorShape?: 'bar'|'vintage'|'underscore'|'filledBox'|'emptyBox'|'doubleUnderscore'; padding?: { left?: number; top?: number; right?: number; bottom?: number }; }
export interface CommandParameter { name: string; type: 'string'|'integer'; required: boolean; description?: string; default?: string|number; }
export interface QuickCommand { id: string; name: string; shells: Shell[]; template?: {program:string; args:string[]}; changeDirectory?: string; description?: string; workingDirectory?: string; parameters?: CommandParameter[]; }
export interface Framework { id: string; label: string; args: string[]; port: number; extraExample: string; }
export interface Pane { profile: Profile; workingDirectory?: string; startupCommand?: string; }
export interface WorkspaceTab { title?: string; pane: Pane; splits?: {direction:'right'|'down'; pane:Pane}[]; }
export interface Workspace { id:string; name:string; tabs:WorkspaceTab[]; }
export type PromptSegmentId = 'directory'|'git'|'node'|'dotnet'|'duration'|'status';
export type PathStyle = 'short'|'home'|'full'|'folder';
export interface PromptSegment { id: PromptSegmentId; color?: string; thresholdMs?: number; pathStyle?: PathStyle; }
export interface PromptConfig { enabled: boolean; segments: PromptSegment[]; }
export interface ShellBehavior { predictions?: 'history'|'off'; predictionView?: 'inline'|'list'; acceptKey?: 'right'|'tab'; hideSensitive?: boolean; historyExclude?: string[]; }
export interface PersonalConfig { schemaVersion: 1|2|3|4|5|6; appearance?: Partial<Record<Profile, Appearance>>; commandPrefix?: ''|'/'|'x-'; shellBehavior?: Partial<Record<Shell, ShellBehavior>>; banner?: {enabled?: boolean}; commands?: QuickCommand[]; workspaces?: Workspace[]; globalTerminal?: unknown; prompt?: PromptConfig; tools?: unknown; team?: {packageId:string; installedVersion:number; sourceUrl?:string}; overrides?: unknown; }
export interface PreviewFile { path:string; content:string; }

export interface TeamPreview { config:PersonalConfig; changes:string[]; files:PreviewFile[]; }

export interface ToolStatus { id:string; name:string; installed:boolean; version:string|null; manualUrl:string; canInstall:boolean; recommended?:boolean; managed?:boolean; canUninstall?:boolean; }

export interface HistoryEntry { id:string; timestamp:number; fileCount:number; canRestore:boolean; }
