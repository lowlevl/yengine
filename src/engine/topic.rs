use crate::format::{
    self, InstallAck, Message, MessageAck, QuitAck, SetLocalAck, UninstallAck, UnwatchAck, WatchAck,
};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Topic {
    InstallAck(String),
    UninstallAck(String),
    WatchAck(String),
    UnwatchAck(String),
    SetLocalAck(String),
    Message,
    MessageAck(String),
    QuitAck,

    Other,
}

impl subable::Topic for Topic {
    type Item = String;

    fn topic(item: &Self::Item) -> Self {
        if let Ok(msg) = format::from_str::<InstallAck>(item) {
            Topic::InstallAck(msg.name)
        } else if let Ok(msg) = format::from_str::<UninstallAck>(item) {
            Topic::UninstallAck(msg.name)
        } else if let Ok(msg) = format::from_str::<WatchAck>(item) {
            Topic::WatchAck(msg.name)
        } else if let Ok(msg) = format::from_str::<UnwatchAck>(item) {
            Topic::UnwatchAck(msg.name)
        } else if let Ok(msg) = format::from_str::<SetLocalAck>(item) {
            Topic::SetLocalAck(msg.name)
        } else if format::from_str::<Message>(item).is_ok() {
            Topic::Message
        } else if let Ok(msg) = format::from_str::<MessageAck>(item) {
            Topic::MessageAck(msg.id)
        } else if format::from_str::<QuitAck>(item).is_ok() {
            Topic::QuitAck
        } else {
            Topic::Other
        }
    }
}
