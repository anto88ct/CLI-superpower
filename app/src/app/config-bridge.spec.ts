import { ConfigBridge } from './config-bridge';

describe('Bridge desktop', () => {
  afterEach(() => { delete window.__TAURI__; });
  it('usa i nomi e gli argomenti IPC concordati', async () => {
    const invoke = vi.fn().mockResolvedValue([]);
    window.__TAURI__ = {core:{invoke}};
    const bridge = new ConfigBridge();
    const config = {schemaVersion:1 as const};
    await bridge.preview(config); await bridge.save(config); await bridge.apply(config);
    await bridge.previewTeam(config, '{}'); await bridge.installTeam(config, '{}');
    await bridge.listHistory(); await bridge.restoreLatest(); await bridge.launchWorkspace('demo'); await bridge.listTools(); await bridge.installTool('pwsh');
    expect(invoke).toHaveBeenCalledWith('preview_config', {config});
    expect(invoke).toHaveBeenCalledWith('save_config', {config});
    expect(invoke).toHaveBeenCalledWith('apply_config', {config});
    expect(invoke).toHaveBeenCalledWith('preview_team_package', {config,content:'{}'});
    expect(invoke).toHaveBeenCalledWith('install_team_package', {config,content:'{}'});
    expect(invoke).toHaveBeenCalledWith('list_history');
    expect(invoke).toHaveBeenCalledWith('restore_latest');
    expect(invoke).toHaveBeenCalledWith('launch_workspace', {id:'demo'});
    expect(invoke).toHaveBeenCalledWith('list_tools');
    expect(invoke).toHaveBeenCalledWith('install_tool', {id:'pwsh'});
  });
});
