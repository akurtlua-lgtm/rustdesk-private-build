import common from '@ohos.app.ability.common';
import display from '@ohos.display';
import { Sharing, sharing } from '../model/Sharing';

@ComponentV2
export struct SharingPanel {
  @Local sharing: Sharing = sharing();
  @Local password: string = '';
  private poll: number = -1;
  private context(): common.UIAbilityContext {
    return this.getUIContext().getHostContext() as common.UIAbilityContext;
  }
  aboutToAppear(): void {
    this.poll = setInterval(() => this.sharing.refresh(this.context()), 500);
  }
  aboutToDisappear(): void {
    if (this.poll !== -1) clearInterval(this.poll);
    this.sharing.stop(this.context());
    this.password = '';
  }
  private start(): void {
    try {
      const screen = display.getDefaultDisplaySync();
      this.sharing.start(this.context(), this.password, screen.width, screen.height);
    } catch { this.sharing.status = '无法读取屏幕信息，请稍后重试。'; }
    finally { this.password = ''; }
  }
  build() {
    Column({ space: 8 }) {
      Text('共享本机屏幕').fontSize(18).fontWeight(FontWeight.Bold)
      Text(this.sharing.status).fontSize(13)
      if (this.sharing.deviceId.length > 0) Text(`本机 ID：${this.sharing.deviceId}`).copyOption(CopyOptions.LocalDevice)
      if (!this.sharing.active) {
        TextInput({ placeholder: '设置设备密码（至少 8 个字符）', text: this.password })
          .type(InputType.Password).onChange((value: string) => this.password = value)
        Button('确认并打开系统共享提示').enabled(!this.sharing.busy && this.password.length >= 8)
          .onClick(() => this.start())
      }
      Button('停止共享').enabled(!this.sharing.busy).onClick(() => this.sharing.stop(this.context()))
      Text('不共享声音，不接受键鼠控制或文件传输；账号登录不会代替设备授权。').fontSize(12).fontColor('#526174')
    }.padding(12).width('100%').backgroundColor('#FFFFFF')
  }
}
