import UIAbility from '@ohos.app.ability.UIAbility';
import window from '@ohos.window';
import { sharing } from '../model/Sharing';
import core from 'libentry.so';
import { Lifecycle, lifecycle } from '../model/Lifecycle';

export default class EntryAbility extends UIAbility {
  private lifecycle: Lifecycle = lifecycle();
  onWindowStageCreate(stage: window.WindowStage): void {
    this.lifecycle.foreground = true;
    stage.loadContent('pages/Index', (error) => {
      if (error.code) this.context.terminateSelf();
    });
  }
  onForeground(): void { this.lifecycle.foreground = true; }
  onBackground(): void {
    this.lifecycle.foreground = false;
    if (core.sharingSupported()) sharing().background(this.context);
  }
}
