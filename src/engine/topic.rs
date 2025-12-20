use crate::{
    format::{
        self, InstallAck, Message, MessageAck, QuitAck, SetLocalAck, UninstallAck, UnwatchAck,
        WatchAck,
    },
    subable,
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
    type From = String;

    fn topic(input: &Self::From) -> Self {
        if let Ok(msg) = format::from_str::<InstallAck>(input) {
            Topic::InstallAck(msg.name)
        } else if let Ok(msg) = format::from_str::<UninstallAck>(input) {
            Topic::UninstallAck(msg.name)
        } else if let Ok(msg) = format::from_str::<WatchAck>(input) {
            Topic::WatchAck(msg.name)
        } else if let Ok(msg) = format::from_str::<UnwatchAck>(input) {
            Topic::UnwatchAck(msg.name)
        } else if let Ok(msg) = format::from_str::<SetLocalAck>(input) {
            Topic::SetLocalAck(msg.name)
        } else if format::from_str::<Message>(input).is_ok() {
            Topic::Message
        } else if let Ok(msg) = format::from_str::<MessageAck>(input) {
            Topic::MessageAck(msg.id)
        } else if format::from_str::<QuitAck>(input).is_ok() {
            Topic::QuitAck
        } else {
            Topic::Other
        }
    }
}
