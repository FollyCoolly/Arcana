use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct InboundMessage {
    pub channel: String,
    pub chat_id: String,
    pub sender_id: String,
    pub content: String,
    pub session_key: String,
}

#[derive(Debug, Clone)]
pub struct OutboundMessage {
    pub channel: String,
    pub chat_id: String,
    pub content: String,
}

pub type InboundTx = mpsc::Sender<InboundMessage>;
pub type InboundRx = mpsc::Receiver<InboundMessage>;
pub type OutboundTx = mpsc::Sender<OutboundMessage>;
pub type OutboundRx = mpsc::Receiver<OutboundMessage>;

/// Two-directional message bus (borrowed from Nanobot's bus/queue.py).
/// Channels push InboundMessages; the agent loop consumes them.
/// The agent pushes OutboundMessages; channel adapters consume them.
pub struct MessageBus {
    pub inbound_tx: InboundTx,
    pub outbound_tx: OutboundTx,
    pub outbound_rx: OutboundRx,
}

impl MessageBus {
    pub fn new(buffer: usize) -> (Self, InboundRx) {
        let (inbound_tx, inbound_rx) = mpsc::channel(buffer);
        let (outbound_tx, outbound_rx) = mpsc::channel(buffer);
        (
            Self {
                inbound_tx,
                outbound_tx,
                outbound_rx,
            },
            inbound_rx,
        )
    }
}
