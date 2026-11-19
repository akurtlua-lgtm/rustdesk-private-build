use super::{raii, Connection, Message, Permission, Subscriber};
use base::mobile_host_stop::StopReceiver;
use std::net::SocketAddr;

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
    pub(super) async fn initialize_mobile(
        &mut self,
        stopped: &mut StopReceiver,
        addr: SocketAddr,
    ) -> bool {
        let initialized = stopped
            .until(async {
                if !self.on_open(addr).await {
                    return false;
                }
                for permission in [
                    Permission::Keyboard,
                    Permission::Clipboard,
                    Permission::Audio,
                    Permission::File,
                    Permission::Restart,
                    Permission::Recording,
                    Permission::BlockInput,
                    Permission::PrivacyMode,
                ] {
                    self.send_permission(permission, false).await;
                }
                true
            })
            .await;
        if initialized != Some(true) {
            self.cancel_mobile_session().await;
            if let Some(server) = self.server.upgrade() {
                server.write().unwrap().remove_connection(&self.inner);
            }
            return false;
        }
        true
    }

    pub(super) async fn cancel_mobile_session(&mut self) {
        raii::AuthedConnID::check_remove_session(self.inner.id(), self.session_key());
        self.on_close("capture session", true).await;
    }

    pub(super) async fn send_mobile(
        &mut self,
        stopped: &mut StopReceiver,
        message: &Message,
    ) -> bool {
        match stopped.until(self.stream.send(message)).await {
            Some(Ok(())) => true,
            Some(Err(error)) => {
                self.on_close(&error.to_string(), false).await;
                false
            }
            None => {
                self.cancel_mobile_session().await;
                false
            }
        }
    }

    pub(super) async fn send_mobile_notice(
        &mut self,
        stopped: &mut StopReceiver,
        message: Message,
    ) -> bool {
        if stopped.until(self.send(message)).await.is_none() {
            self.cancel_mobile_session().await;
            return false;
        }
        true
    }

    pub(super) async fn notify_mobile_close(
        &mut self,
        stopped: &mut StopReceiver,
        reason: &str,
    ) -> bool {
        if stopped
            .until(self.send_close_reason_no_retry(reason))
            .await
            .is_none()
        {
            self.cancel_mobile_session().await;
            return false;
        }
        true
    }

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
