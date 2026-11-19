import common from '@ohos.app.ability.common';
import core from 'libentry.so';
import { Lifecycle, lifecycle } from '../model/Lifecycle';
import { Account } from '../model/Account';
import { AccountPanel } from '../components/AccountPanel';

interface RemoteDisplay { x: number; y: number; width: number; height: number; }
interface NativeEvent {
  name: string; sid: string; type?: string; title?: string; text?: string;
  permission?: string; enabled?: boolean; current_display?: number; displays?: RemoteDisplay[];
  x?: number; y?: number; width?: number; height?: number;
  speed?: string; fps?: Record<string, number>; delay?: number; codec?: string;
}

@Entry
@ComponentV2
struct Index {
  @Local account: Account = new Account();
  @Local peer: string = '';
  @Local password: string = '';
  @Local session: string = '';
  @Local status: string = '初始化中';
  @Local ready: boolean = false;
  @Local surfaceReady: boolean = false;
  @Local firstFrame: boolean = false;
  @Local viewOnly: boolean = true;
  @Local keyboardAllowed: boolean = true;
  @Local authKind: string = '';
  @Local authPassword: string = '';
  @Local inputText: string = '';
  @Local quality: string = '';
  @Local screenWidth: number = 1280;
  @Local screenHeight: number = 720;
  @Local availableWidth: number = 360;
  @Local availableHeight: number = 300;
  @Local lifecycle: Lifecycle = lifecycle();
  private originX: number = 0;
  private originY: number = 0;
  private pointerDown: boolean = false;
  private pointerX: number = 0;
  private pointerY: number = 0;

  aboutToAppear(): void {
    const context = this.getUIContext().getHostContext() as common.UIAbilityContext;
    try {
      const result = core.initialize(context.cacheDir, context.filesDir);
      if (result !== 0) { this.status = `初始化失败 (${result})`; return; }
      core.onEvent((message: string) => this.handleEvent(message));
      this.ready = true;
      this.status = `RustDesk ${core.version()}`;
    } catch { this.status = '原生核心初始化失败'; }
  }
  aboutToDisappear(): void { this.disconnect(); }
  @Monitor('lifecycle.foreground')
  foregroundChanged(): void { if (!this.lifecycle.foreground) this.disconnect(); }
  private surfaceWidth(): number {
    return Math.min(this.availableWidth, this.availableHeight * this.screenWidth / this.screenHeight);
  }
  private surfaceHeight(): number { return this.surfaceWidth() * this.screenHeight / this.screenWidth; }
  private releasePointer(): void {
    if (this.pointerDown && this.session.length > 0) core.mouse(this.session, 10, this.pointerX, this.pointerY, 0);
    this.pointerDown = false;
  }
  private disconnect(): void {
    this.releasePointer();
    if (this.session.length > 0) core.close(this.session);
    this.session = ''; this.firstFrame = false; this.authKind = ''; this.password = '';
    this.authPassword = ''; this.inputText = ''; this.quality = '';
    this.status = '已断开';
  }
  private connect(): void {
    if (!this.ready || !this.surfaceReady || this.peer.trim().length === 0 || this.session.length > 0) return;
    try {
      this.keyboardAllowed = true; this.viewOnly = true; this.firstFrame = false;
      this.session = core.connect(this.peer.trim(), this.password, false);
      if (core.viewOnly(this.session, true) !== 0) {
        this.disconnect(); this.status = '无法设置仅查看权限'; return;
      }
      this.password = ''; this.status = '正在建立加密连接';
    } catch { this.status = '无法建立会话，请检查设备 ID 与服务器配置'; }
  }
  private handleEvent(message: string): void {
    let event: NativeEvent;
    try { event = JSON.parse(message) as NativeEvent; } catch { this.status = '无法读取连接状态'; return; }
    if (event.sid !== this.session) return;
    if (event.name === 'first_frame') { this.firstFrame = true; this.authKind = ''; this.status = '已连接'; }
    if (event.name === 'authenticated') this.status = '认证成功，等待画面';
    if (event.name === 'peer_info' && event.displays && event.displays.length > 0) {
      const display = event.displays[event.current_display ?? 0] ?? event.displays[0];
      this.setDisplay(display.x, display.y, display.width, display.height);
    }
    if ((event.name === 'display' || event.name === 'switch_display') && event.width && event.height)
      this.setDisplay(event.x ?? 0, event.y ?? 0, event.width, event.height);
    if (event.name === 'permission' && event.permission === 'keyboard') {
      if (event.enabled === false) this.releasePointer();
      this.keyboardAllowed = event.enabled === true;
    }
    if (event.name === 'quality') this.quality = `FPS ${event.fps ? Object.values(event.fps).join('/') : '-'}  延迟 ${event.delay ?? '-'} ms  ${event.codec ?? ''}`;
    if (event.name === 'message') {
      this.status = `${event.title ?? ''} ${event.text ?? ''}`;
      const kind = event.type ?? '';
      if (kind === 'input-password' || kind === 're-input-password' || kind === 'input-2fa')
        this.authKind = kind;
    }
    if (event.name === 'connection_closed') { this.releasePointer(); this.session = ''; this.firstFrame = false; this.authKind = ''; this.authPassword = ''; this.inputText = ''; this.quality = ''; this.status = '连接已结束'; }
  }
  private setDisplay(x: number, y: number, width: number, height: number): void {
    if (width <= 0 || height <= 0) return;
    this.releasePointer(); this.originX = x; this.originY = y;
    this.screenWidth = width; this.screenHeight = height;
  }
  private touch(event: TouchEvent): void {
    if (!this.firstFrame || this.viewOnly || !this.keyboardAllowed || event.touches.length === 0) {
      if (event.type === TouchType.Up || event.type === TouchType.Cancel) this.releasePointer();
      return;
    }
    const point = event.touches[0];
    this.pointerX = this.originX + Math.max(0, Math.min(this.screenWidth - 1, Math.floor(point.x * this.screenWidth / this.surfaceWidth())));
    this.pointerY = this.originY + Math.max(0, Math.min(this.screenHeight - 1, Math.floor(point.y * this.screenHeight / this.surfaceHeight())));
    if (event.type === TouchType.Down && !this.pointerDown)
      this.pointerDown = core.mouse(this.session, 9, this.pointerX, this.pointerY, 0) === 0;
    else if (event.type === TouchType.Move && this.pointerDown) core.mouse(this.session, 8, this.pointerX, this.pointerY, 0);
    else if (event.type === TouchType.Up || event.type === TouchType.Cancel) this.releasePointer();
  }
  private submitAuthentication(): void {
    let result = -1;
    if (this.authKind === 'input-2fa') result = core.twoFactor(this.session, this.authPassword, false);
    else result = core.login(this.session, this.authPassword, '', '', false);
    if (result === 0) this.authKind = '';
    else this.status = '认证提交失败';
    this.authPassword = '';
  }
  build() {
    Column({ space: 12 }) {
      Text('RustDesk Private').fontSize(24).fontWeight(FontWeight.Bold)
      Text(this.status).fontSize(14).maxLines(3)
      if (this.session.length === 0) {
        if (this.ready) AccountPanel({ account: this.account, onSelect: (peer: string) => this.peer = peer })
        TextInput({ placeholder: '远程设备 ID', text: this.peer }).onChange((value: string) => this.peer = value)
        TextInput({ placeholder: '设备密码（也可等待人工授权）', text: this.password }).type(InputType.Password).onChange((value: string) => this.password = value)
        Button('连接远程设备').enabled(this.ready && this.surfaceReady).onClick(() => this.connect())
        Text('本阶段为原生控制端；手机屏幕共享尚未开放。连接仍需设备密码或人工授权。').fontSize(13).fontColor('#526174')
      } else {
        Row({ space: 12 }) {
          Toggle({ type: ToggleType.Switch, isOn: this.viewOnly }).onChange((value: boolean) => {
            this.releasePointer();
            if (core.viewOnly(this.session, value) === 0) this.viewOnly = value;
          }).accessibilityText('仅查看')
          Text('仅查看')
          Button('断开').onClick(() => this.disconnect())
        }
      }
      Stack() {
        XComponent({ id: 'remote-desktop', type: XComponentType.SURFACE, libraryname: 'entry' })
          .width(this.surfaceWidth()).height(this.surfaceHeight())
          .onLoad(() => this.surfaceReady = true)
          .onDestroy(() => { this.surfaceReady = false; this.disconnect(); })
          .onTouch((event: TouchEvent) => this.touch(event))
      }.width('100%').layoutWeight(1).backgroundColor('#111827')
        .onAreaChange((_old: Area, area: Area) => { this.availableWidth = Number(area.width); this.availableHeight = Number(area.height); })
      if (this.session.length > 0) {
        Text(this.quality).fontSize(12)
        if (this.authKind.length > 0) {
          TextInput({ placeholder: this.authKind === 'input-2fa' ? '二次验证码' : '远程设备密码', text: this.authPassword }).type(InputType.Password).onChange((value: string) => this.authPassword = value)
          Button('提交认证').onClick(() => this.submitAuthentication())
        }
        Row({ space: 8 }) {
          TextInput({ placeholder: '发送文字', text: this.inputText }).layoutWeight(1).onChange((value: string) => this.inputText = value)
          Button('发送').enabled(this.firstFrame && !this.viewOnly && this.keyboardAllowed).onClick(() => {
            if (core.text(this.session, this.inputText) === 0) this.inputText = '';
          })
          Button('回车').enabled(this.firstFrame && !this.viewOnly && this.keyboardAllowed).onClick(() => core.key(this.session, 'VK_RETURN', 0))
        }
      }
    }.padding(16).width('100%').height('100%').backgroundColor('#F6F8FC')
  }
}
