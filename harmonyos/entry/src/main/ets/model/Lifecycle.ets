import { AppStorageV2 } from '@ohos.arkui.StateManagement';

@ObservedV2
export class Lifecycle {
  @Trace foreground: boolean = true;
}

export function lifecycle(): Lifecycle {
  const state = AppStorageV2.connect(Lifecycle, 'rustdesk.lifecycle', () => new Lifecycle());
  if (!state) throw new Error('Cannot initialize application lifecycle');
  return state;
}
