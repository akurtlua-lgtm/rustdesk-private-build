use super::Connection;

pub(super) fn restrict_permissions(connection: &mut Connection) {
    connection.keyboard = false;
    connection.clipboard = false;
    connection.audio = false;
    connection.file = false;
    connection.restart = false;
    connection.recording = false;
    connection.block_input = false;
    connection.privacy_mode = false;
}

impl Connection {
    pub(super) async fn try_port_forward_loop(
        &mut self,
        _receiver: &mut hbb_common::tokio::sync::mpsc::UnboundedReceiver<super::Data>,
    ) -> hbb_common::ResultType<()> {
        if self.port_forward_socket.is_some() {
            hbb_common::bail!("Port forwarding is unsupported in a read-only mobile session");
        }
        Ok(())
    }
}
