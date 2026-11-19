use std::convert::Infallible;
use tokio::sync::watch;

pub struct StopSignal {
    sender: watch::Sender<bool>,
}

impl StopSignal {
    pub fn new() -> Self {
        let (sender, _) = watch::channel(false);
        Self { sender }
    }

    pub fn is_stopped(&self) -> bool {
        *self.sender.borrow()
    }

    pub fn stop(&self) {
        self.sender.send_replace(true);
    }

    pub fn receiver(&self, connection_id: i32) -> StopReceiver {
        StopReceiver {
            receiver: self.sender.subscribe(),
            connection_id,
        }
    }
}

impl Default for StopSignal {
    fn default() -> Self {
        Self::new()
    }
}

pub struct StopReceiver {
    receiver: watch::Receiver<bool>,
    connection_id: i32,
}

impl StopReceiver {
    pub async fn recv(&mut self) -> Result<Vec<i32>, Infallible> {
        // Losing the authority that owns this channel also revokes the session.
        match self.receiver.wait_for(|stopped| *stopped).await {
            Ok(_) | Err(_) => Ok(vec![self.connection_id]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn stop_reaches_existing_and_late_subscribers() {
        let signal = StopSignal::new();
        let mut existing = signal.receiver(7);
        assert!(timeout(Duration::ZERO, existing.recv()).await.is_err());
        signal.stop();
        assert_eq!(
            timeout(Duration::from_secs(1), existing.recv())
                .await
                .unwrap()
                .unwrap(),
            vec![7]
        );
        let mut late = signal.receiver(9);
        assert_eq!(
            timeout(Duration::from_secs(1), late.recv())
                .await
                .unwrap()
                .unwrap(),
            vec![9]
        );
    }

    #[tokio::test]
    async fn owner_loss_revokes_the_receiver() {
        let signal = StopSignal::new();
        let mut receiver = signal.receiver(11);
        drop(signal);
        assert_eq!(
            timeout(Duration::from_secs(1), receiver.recv())
                .await
                .unwrap()
                .unwrap(),
            vec![11]
        );
    }
}
