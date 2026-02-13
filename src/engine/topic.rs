use crate::wire::{
    self, InstallAck, Message, MessageAck, QuitAck, SetLocalAck, UninstallAck, UnwatchAck, WatchAck,
};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Topic {
    InstallAck(String),
    UninstallAck(String),
    WatchAck(String),
    UnwatchAck(String),
    Watch,
    SetLocalAck(String),
    Message,
    MessageAck(String),
    QuitAck,

    Other,
}

impl subable::Topic for Topic {
    type Item = String;

    fn topic(item: &Self::Item) -> Self {
        if let Ok(msg) = wire::from_str::<InstallAck>(item) {
            Topic::InstallAck(msg.name)
        } else if let Ok(msg) = wire::from_str::<UninstallAck>(item) {
            Topic::UninstallAck(msg.name)
        } else if let Ok(msg) = wire::from_str::<WatchAck>(item) {
            Topic::WatchAck(msg.name)
        } else if let Ok(msg) = wire::from_str::<UnwatchAck>(item) {
            Topic::UnwatchAck(msg.name)
        } else if let Ok(msg) = wire::from_str::<SetLocalAck>(item) {
            Topic::SetLocalAck(msg.name)
        } else if wire::from_str::<Message>(item).is_ok() {
            Topic::Message
        } else if let Ok(msg) = wire::from_str::<MessageAck>(item) {
            Topic::MessageAck(msg.id)
        } else if wire::from_str::<QuitAck>(item).is_ok() {
            Topic::QuitAck
        } else {
            Topic::Other
        }
    }

    fn fallback(self) -> Self {
        match self {
            // Fallback unhandled `MessageAck` as `Watch`
            Self::MessageAck(_) => Self::Watch,
            other => other,
        }
    }
}
