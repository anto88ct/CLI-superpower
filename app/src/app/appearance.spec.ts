import { TestBed } from '@angular/core/testing';
import { App } from './app';
import { ConfigBridge } from './config-bridge';

describe('Aspetto', () => {
  it('cambia palette e font, e associa l’immagine al profilo scelto prima del caricamento', async () => {
    let complete!: (result:{path:string;previewUrl:string}) => void;
    const bridge = {
      desktop:false,
      load:vi.fn().mockResolvedValue({schemaVersion:1}),
      terminalProfiles:vi.fn().mockResolvedValue(['pwsh','powershell','cmd']),
      backgroundPreview:vi.fn().mockResolvedValue(null),
      uploadBackground:vi.fn().mockReturnValue(new Promise(resolve => complete = resolve)),
    };
    await TestBed.configureTestingModule({imports:[App],providers:[{provide:ConfigBridge,useValue:bridge}]}).compileComponents();
    const fixture = TestBed.createComponent(App); fixture.detectChanges(); await fixture.whenStable();
    const app = fixture.componentInstance;
    app.setAppearance('colorScheme','Solarized Light');
    expect(app.terminalBackground).toBe('#fdf6e3'); expect(app.terminalText).toBe('#657b83');
    app.selectFont('Ubuntu Mono'); expect(app.appearance.font?.face).toBe('Ubuntu Mono');
    app.selectFont('__custom__'); app.setFont('face','Font personale'); expect(app.fontSelection).toBe('__custom__');
    const file = new File(['immagine'],'sfondo.png',{type:'image/png'});
    const input = {files:[file],value:'sfondo.png'} as unknown as HTMLInputElement;
    const upload = app.chooseBackground({target:input} as unknown as Event);
    app.selectProfile('powershell'); complete({path:'C:\\foto.png',previewUrl:'data:image/png;base64,AA=='}); await upload;
    expect(app.profile).toBe('powershell'); expect(app.config.appearance?.pwsh?.backgroundImage).toBe('C:\\foto.png');
    expect(app.appearance.backgroundImage).toBeUndefined(); app.selectProfile('pwsh');
    expect(app.backgroundPreviewUrl).toBe('data:image/png;base64,AA==');
  });
});
