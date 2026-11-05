import { Pane, PersonalConfig } from './models';

export type EditorSection = 'appearance' | 'commands' | 'workspaces' | 'prompt' | 'autocomplete' | 'team';
export interface ConfigIssue { section: EditorSection; field: string; label: string; message: string; }

const identifier = /^[a-z][a-z0-9-]{0,63}$/;
const parameter = /^[a-z][a-z0-9_]{0,63}$/;
const identifierHelp = 'Usa da 1 a 64 caratteri. Inizia con una lettera minuscola e usa lettere minuscole, cifre o trattini.';
const pathHelp = 'Usa un percorso Windows assoluto, per esempio C:\\Progetti.';
const absolutePath = /^(?:[a-z]:[\\/]|[\\/]{2}(?:\?[\\/](?:[a-z]:[\\/]|UNC[\\/][^\\/]+[\\/][^\\/]+)|[^.?\\/][^\\/]*[\\/][^\\/]+))/i;

export function validateConfig(config: PersonalConfig): ConfigIssue[] {
  const issues: ConfigIssue[] = [];
  const add = (section: EditorSection, field: string, label: string, message: string) =>
    issues.push({section, field, label, message});

  const commandIds = new Set<string>();
  const commandNames = new Set<string>();
  for (const [index, command] of (config.commands ?? []).entries()) {
    const key = `command-${index}`;
    const issue = (field: string, label: string, message: string) =>
      add('commands', `${key}-${field}`, `Comando ${index + 1} · ${label}`, message);
    if (!identifier.test(command.id)) issue('id', 'ID stabile', identifierHelp);
    else if (commandIds.has(command.id)) issue('id', 'ID stabile', 'Usa un ID diverso dagli altri comandi.');
    commandIds.add(command.id);
    if (!identifier.test(command.name)) issue('name', 'Nome comando', identifierHelp);
    if (!command.shells.length) issue('shells', 'Shell', 'Seleziona almeno una shell.');
    if (command.shells.includes('cmd') && command.parameters?.length) {
      issue('shells', 'Shell', 'Per CMD usa argomenti fissi. Per i parametri al prompt seleziona PowerShell.');
    }
    for (const shell of command.shells) {
      const name = `${shell}:${command.name}`;
      if (identifier.test(command.name) && commandNames.has(name)) {
        issue('name', 'Nome comando', 'Questo nome è già usato nella stessa shell.');
      }
      commandNames.add(name);
    }
    if (command.changeDirectory !== undefined) {
      if (!absolutePath.test(command.changeDirectory)) issue('program', 'Cartella', pathHelp);
      if (command.parameters?.length) issue('program', 'Cartella', 'Un comando «cd» non accetta parametri: rimuovili.');
      if (command.workingDirectory) issue('directory', 'Cartella di lavoro', 'Un comando «cd» non usa una cartella di lavoro: svuota il campo.');
      continue;
    }
    const template = command.template ?? {program: '', args: []};
    const program = template.program;
    if (!program.trim()) issue('program', 'Programma', 'Inserisci il nome di un eseguibile.');
    else if (/[\\/:]/.test(program) && !absolutePath.test(program)) issue('program', 'Programma', pathHelp);
    if (command.workingDirectory && !absolutePath.test(command.workingDirectory)) {
      issue('directory', 'Cartella di lavoro', pathHelp);
    }
    const names = new Set<string>();
    let optionalSeen = false;
    for (const [i, param] of (command.parameters ?? []).entries()) {
      const label = `Parametro ${i + 1}`;
      if (!parameter.test(param.name)) {
        issue(`parameter-${i}-name`, label, 'Inizia con una lettera minuscola e usa lettere minuscole, cifre o trattini bassi, fino a 64 caratteri.');
      } else if (names.has(param.name)) {
        issue(`parameter-${i}-name`, label, 'Usa un nome diverso dagli altri parametri.');
      }
      names.add(param.name);
      if (param.required && optionalSeen) {
        issue(`parameter-${i}-required`, label, 'I parametri obbligatori devono precedere quelli facoltativi.');
      }
      optionalSeen ||= !param.required;
      if (param.type === 'integer' && param.default !== undefined &&
          (typeof param.default !== 'number' || !Number.isInteger(param.default) ||
            param.default < -2147483648 || param.default > 2147483647)) {
        issue(`parameter-${i}-default`, label, 'Inserisci un intero compreso tra −2147483648 e 2147483647.');
      }
    }
    const used = new Set<string>();
    for (const arg of template.args) {
      const match = /^\{([a-z][a-z0-9_]{0,63})\}$/.exec(arg);
      if (match) used.add(match[1]);
      else if (/[{}]/.test(arg.replaceAll('{{', '').replaceAll('}}', ''))) {
        issue('args', 'Argomenti', 'Scrivi ogni parametro da solo sulla riga. Raddoppia le parentesi graffe se devono comparire nel testo.');
        break;
      }
    }
    const unknown = [...used].find(name => !names.has(name));
    const unused = [...names].find(name => parameter.test(name) && !used.has(name));
    if (unknown) issue('args', 'Argomenti', `Definisci il parametro «${unknown}» oppure rimuovi il riferimento.`);
    if (unused) issue('args', 'Argomenti', `Aggiungi {${unused}} a una riga o rimuovi il parametro inutilizzato.`);
  }

  for (const [shell, behavior] of Object.entries(config.shellBehavior ?? {})) {
    const patterns = behavior?.historyExclude ?? [];
    if (patterns.length > 50) {
      add('autocomplete', `behavior-${shell}-exclude`, 'Frammenti da escludere', 'Inserisci al massimo 50 frammenti.');
    } else if (patterns.some(pattern => [...pattern].length > 100)) {
      add('autocomplete', `behavior-${shell}-exclude`, 'Frammenti da escludere', 'Ogni frammento può avere al massimo 100 caratteri.');
    }
  }

  const workspaceIds = new Set<string>();
  for (const [index, workspace] of (config.workspaces ?? []).entries()) {
    const key = `workspace-${index}`;
    const issue = (field: string, label: string, message: string) =>
      add('workspaces', `${key}-${field}`, `Workspace ${index + 1} · ${label}`, message);
    if (!identifier.test(workspace.id)) issue('id', 'ID stabile', identifierHelp);
    else if (workspaceIds.has(workspace.id)) issue('id', 'ID stabile', 'Usa un ID diverso dagli altri workspace.');
    workspaceIds.add(workspace.id);
    if (!workspace.name.trim()) issue('name', 'Nome', 'Inserisci un nome per il workspace.');
    const checkPane = (pane: Pane, field: string, label: string) => {
      if (pane.workingDirectory && !absolutePath.test(pane.workingDirectory)) issue(`${field}-directory`, label, pathHelp);
      if (pane.startupCommand !== undefined && !pane.startupCommand.trim()) {
        issue(`${field}-startup`, label, 'Inserisci un comando oppure lascia vuoto il campo.');
      }
    };
    for (const [tabIndex, tab] of workspace.tabs.entries()) {
      checkPane(tab.pane, `tab-${tabIndex}`, `Scheda ${tabIndex + 1}`);
      for (const [splitIndex, split] of (tab.splits ?? []).entries()) {
        checkPane(split.pane, `tab-${tabIndex}-split-${splitIndex}`, `Scheda ${tabIndex + 1}, pannello ${splitIndex + 2}`);
      }
    }
  }

  const prompt = config.prompt;
  if (prompt?.enabled && !(prompt.segments?.length) && (!config.team || prompt.segments !== undefined)) {
    add('prompt', 'prompt-segments', 'Segmenti del prompt', 'Aggiungi almeno un segmento oppure disattiva il prompt.');
  }
  for (const [index, segment] of (prompt?.segments ?? []).entries()) {
    if (segment.color && !/^#[0-9a-f]{6}$/i.test(segment.color)) {
      add('prompt', `segment-${index}-color`, 'Colore del segmento', 'Usa il formato #RRGGBB, per esempio #7AA2F7.');
    }
    const threshold = segment.thresholdMs;
    if (threshold !== undefined && (!Number.isInteger(threshold) || threshold < 0 || threshold > 2147483647)) {
      add('prompt', `segment-${index}-threshold`, 'Soglia del segmento', 'Inserisci un intero tra 0 e 2147483647 ms.');
    }
  }
  return issues;
}

export function sectionForError(message: string): EditorSection | undefined {
  const key = /^(?:personal:\s*)?(appearance|globalTerminal|commandPrefix|commands|workspaces|prompt|shellBehavior|team)(?:[.:\s]|$)/.exec(message)?.[1];
  switch (key) {
    case 'appearance':
    case 'globalTerminal': return 'appearance';
    case 'commandPrefix':
    case 'commands': return 'commands';
    case 'workspaces': return 'workspaces';
    case 'prompt': return 'prompt';
    case 'shellBehavior': return 'autocomplete';
    case 'team': return 'team';
    default: return undefined;
  }
}
