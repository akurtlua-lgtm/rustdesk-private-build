import { AppStorageV2 } from '@ohos.arkui.StateManagement';
import common from '@ohos.app.ability.common';
import backgroundTaskManager from '@ohos.resourceschedule.backgroundTaskManager';
import wantAgent from '@ohos.app.ability.wantAgent';
import core from 'libentry.so';

@ObservedV2
export class Sharing {
  @Trace active: boolean = false;
  @Trace busy: boolean = false;
  @Trace deviceId: string = '';
  @Trace status: string = '共享仅供查看；每次都需要系统确认及设备密码。';
  private backgroundAllowed: boolean = false;
  private backgroundRequested: boolean = false;
  private backgroundPending: boolean = false;
  private cancellation?: (info: backgroundTaskManager.ContinuousTaskCancelInfo) => void;

  async start(context: common.UIAbilityContext, password: string, width: number, height: number): Promise<void> {
    if (this.active || this.busy || this.backgroundPending) return;
    if (core.sharingPassword(password) !== 0) {
      this.status = '无法设置设备密码，请使用至少 8 个字符并检查安全设置。'; return;
    }
    const result = core.sharingStart(width, height);
    if (result !== 0) {
      await this.stop(context);
      this.status = `无法启动共享 (${result})。${this.status}`;
      return;
    }
    this.active = true;
    this.backgroundRequested = false;
    this.status = '请在系统提示中确认共享屏幕';
  }
  async refresh(context: common.UIAbilityContext): Promise<void> {
    if (!this.active || this.busy) return;
    const state = core.sharingStatus();
    this.deviceId = state.deviceId;
    if (state.phase === 2) this.status = '已获系统授权，正在注册加密共享服务';
    if (state.phase === 3) this.status = '共享已就绪，等待远端凭设备密码连接（仅查看）' +
      (this.backgroundRequested && !this.backgroundPending && !this.backgroundAllowed ? '；仅限前台，离开应用将停止' : '');
    if (state.phase >= 2 && state.phase <= 3 && !this.backgroundRequested) {
      this.backgroundRequested = true;
      this.backgroundPending = true;
      try {
        const agent = await wantAgent.getWantAgent({
          wants: [{ bundleName: context.applicationInfo.name, abilityName: 'EntryAbility' }],
          actionType: wantAgent.OperationType.START_ABILITY, requestCode: 0,
          actionFlags: [wantAgent.WantAgentFlags.UPDATE_PRESENT_FLAG]
        });
        if (this.active) {
          const cancelled = (_info: backgroundTaskManager.ContinuousTaskCancelInfo): void => {
            this.backgroundAllowed = false;
            this.stop(context);
          };
          backgroundTaskManager.on('continuousTaskCancel', cancelled);
          this.cancellation = cancelled;
          await backgroundTaskManager.startBackgroundRunning(context,
            backgroundTaskManager.BackgroundMode.AUDIO_RECORDING, agent);
          this.backgroundAllowed = true;
        }
        if (!this.active && this.backgroundAllowed) {
          await backgroundTaskManager.stopBackgroundRunning(context);
          this.backgroundAllowed = false;
        }
      } catch { this.status = '后台共享未获系统许可，离开应用时将停止共享。'; }
      finally { this.backgroundPending = false; }
    }
    if (state.phase === 0 || state.phase >= 4) await this.stop(context);
  }
  async stop(context: common.UIAbilityContext): Promise<void> {
    if (this.busy) return;
    this.busy = true;
    this.active = false;
    try {
      const result = await core.sharingStop();
      if (this.backgroundAllowed) {
        await backgroundTaskManager.stopBackgroundRunning(context);
        this.backgroundAllowed = false;
      }
      if (this.cancellation) {
        backgroundTaskManager.off('continuousTaskCancel', this.cancellation);
        this.cancellation = undefined;
      }
      this.status = result === 0 ? '共享已停止' : `会话已撤销，系统采集清理失败 (${result})，请重试停止`;
    } catch { this.status = '会话已撤销，停止操作未完成，请重试。'; }
    finally { this.busy = false; }
  }
  background(context: common.UIAbilityContext): void {
    if (this.active && !this.backgroundAllowed) this.stop(context);
  }
}
export function sharing(): Sharing {
  const state = AppStorageV2.connect(Sharing, 'rustdesk.sharing', () => new Sharing());
  if (!state) throw new Error('Cannot initialize screen sharing');
  return state;
}
