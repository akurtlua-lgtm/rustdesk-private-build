import { Account, AccountDevice } from '../model/Account';

@ComponentV2
export struct AccountPanel {
  @Param account: Account = new Account();
  @Event onSelect: (peer: string) => void = (_peer: string) => {};
  @Local username: string = '';
  @Local password: string = '';
  @Local peer: string = '';
  @Local deviceName: string = '';
  @Local note: string = '';
  @Local editing: number = 0;
  @Monitor('account.userName')
  accountChanged(): void {
    this.editing = 0; this.peer = ''; this.deviceName = ''; this.note = '';
    this.username = ''; this.password = '';
  }
  build() {
    Scroll() {
      Column({ space: 8 }) {
        Text('团队账号与设备资料').fontSize(18).fontWeight(FontWeight.Medium)
        Text(this.account.message).fontSize(13)
        if (this.account.userName.length === 0) {
          TextInput({ placeholder: '账号（3–32位字母、数字、下划线或短横线）', text: this.username })
            .onChange((value: string) => this.username = value)
          TextInput({ placeholder: '账号密码（注册需12–128位）', text: this.password }).type(InputType.Password)
            .onChange((value: string) => this.password = value)
          Row({ space: 8 }) {
            Button('登录').enabled(!this.account.busy).onClick(() => {
              this.account.login(this.username, this.password); this.password = '';
            })
            Button('注册并等待审核').enabled(!this.account.busy).onClick(() => {
              this.account.register(this.username, this.password); this.password = '';
            })
          }
        } else {
          Row({ space: 8 }) {
            Text(this.account.userName).layoutWeight(1)
            Button('刷新').enabled(!this.account.busy).onClick(() => this.account.refresh())
            Button('退出').enabled(!this.account.busy).onClick(() => this.account.logout())
          }
          ForEach(this.account.devices, (device: AccountDevice) => {
            Row({ space: 8 }) {
              Column({ space: 3 }) {
                Text(`${device.name} · ${device.peer_id}`).fontSize(14)
                Text(device.state === 'approved' ? '资料已审核；仍需设备授权' : '资料待审核或已停用').fontSize(12)
              }.layoutWeight(1).alignItems(HorizontalAlign.Start)
              Button('填写 ID').enabled(device.state === 'approved').onClick(() => this.onSelect(device.peer_id))
              Button('编辑').enabled(!this.account.busy).onClick(() => {
                this.editing = device.did; this.peer = device.peer_id; this.deviceName = device.name; this.note = device.note;
              })
            }
          }, (device: AccountDevice) => device.did.toString())
          TextInput({ placeholder: '要登记的设备 ID', text: this.peer }).enabled(this.editing === 0)
            .onChange((value: string) => this.peer = value)
          TextInput({ placeholder: '设备名称', text: this.deviceName }).onChange((value: string) => this.deviceName = value)
          TextInput({ placeholder: '备注', text: this.note }).onChange((value: string) => this.note = value)
          Row({ space: 8 }) {
            Button(this.editing > 0 ? '保存设备资料' : '登记并等待审核').enabled(!this.account.busy).onClick(() =>
              this.account.saveDevice(this.peer, this.deviceName, this.note, this.editing))
            if (this.editing > 0) Button('取消编辑').onClick(() => { this.editing = 0; this.peer = ''; this.deviceName = ''; this.note = ''; })
          }
        }
      }.width('100%').alignItems(HorizontalAlign.Start)
    }.height(260).width('100%')
  }
}
