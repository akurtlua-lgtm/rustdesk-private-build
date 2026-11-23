import core from 'libentry.so';

export interface AccountDevice {
  did: number; peer_id: string; name: string; note: string; state: string;
}
interface AccountBody {
  error?: string; message?: string; access_token?: string;
  uid?: number; name?: string; status?: number; data?: AccountDevice[];
}
interface AccountResponse { status: number; error?: string; body?: AccountBody; }

@ObservedV2
export class Account {
  @Trace userName: string = '';
  @Trace message: string = '';
  @Trace busy: boolean = false;
  @Trace devices: AccountDevice[] = [];
  private token: string = '';

  private clear(): void { this.token = ''; this.userName = ''; this.devices = []; }
  private async call(operation: string, payload: string = '{}'): Promise<AccountBody> {
    const raw = await core.account(operation, payload, this.token);
    const response = JSON.parse(raw) as AccountResponse;
    if (response.status === 401) {
      this.clear(); throw new Error('账号已过期或已撤销，请重新登录');
    }
    if (response.status !== 200 || !response.body)
      throw new Error(response.body?.error ?? response.error ?? '账号请求失败');
    return response.body;
  }
  async login(username: string, password: string): Promise<void> {
    if (this.busy) return;
    this.busy = true;
    try {
      const body = await this.call('login', JSON.stringify({ username: username, password: password, type: 'account', autoLogin: false }));
      if (!body.access_token || !body.access_token.startsWith('rdoss_v1_')) throw new Error('账号凭据格式不兼容');
      this.token = body.access_token;
      const user = await this.call('me');
      if (!user.name || user.status !== 1) throw new Error('账号尚未启用');
      this.userName = user.name;
      const result = await this.call('devices');
      this.devices = result.data ?? [];
      this.message = '已登录；设备连接仍需独立授权';
    } catch (error) {
      this.clear(); this.message = (error as Error).message;
    } finally { this.busy = false; }
  }
  async register(username: string, password: string): Promise<void> {
    if (this.busy) return;
    this.busy = true;
    try {
      const result = await this.call('register', JSON.stringify({ username: username, password: password }));
      this.message = result.message ?? '已提交注册，等待管理员审核后才能登录';
    } catch (error) { this.message = (error as Error).message; }
    finally { this.busy = false; }
  }
  async refresh(): Promise<void> {
    if (this.busy) return;
    this.busy = true;
    try {
      await this.call('me');
      const result = await this.call('devices');
      this.devices = result.data ?? []; this.message = '设备资料已更新';
    } catch (error) { this.message = (error as Error).message; }
    finally { this.busy = false; }
  }
  async saveDevice(peer: string, name: string, note: string, did: number = 0): Promise<void> {
    if (this.busy) return;
    this.busy = true;
    try {
      const payload = did > 0 ? JSON.stringify({ did: did, name: name, note: note }) :
        JSON.stringify({ peer_id: peer, name: name, note: note });
      const result = await this.call(did > 0 ? 'updateDevice' : 'registerDevice', payload);
      this.message = result.message ?? '设备资料已保存';
      const devices = await this.call('devices');
      this.devices = devices.data ?? [];
    } catch (error) { this.message = (error as Error).message; }
    finally { this.busy = false; }
  }
  async logout(): Promise<void> {
    if (this.busy) return;
    this.busy = true;
    try { await this.call('logout'); this.message = '已退出并撤销本次账号登录'; }
    catch { this.message = '本地登录已清除；服务端撤销未确认，可稍后重新登录后管理会话'; }
    finally { this.clear(); this.busy = false; }
  }
}
