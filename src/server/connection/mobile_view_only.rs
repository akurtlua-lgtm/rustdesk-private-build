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
    pub(super) fn add_mobile_monitor_service(
        &self,
        server: &mut crate::server::Server,
    ) -> hbb_common::ResultType<()> {
        let name =
            crate::server::video_service::get_service_name(crate::server::VideoSource::Monitor, 0);
        if !server.contains(&name) {
            let service = crate::server::video_service::mobile_view_only::new_for_source(
                self.capture_source.clone(),
            )?;
            server.add_service(Box::new(service));
        }
        Ok(())
    }

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
