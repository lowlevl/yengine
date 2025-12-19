use crate::{
    format::{
        self, InstallAck, Message, MessageAck, QuitAck, SetLocalAck, UninstallAck, UnwatchAck,
        WatchAck,
    },
    pubsub::PubSubable,
};

pub struct Msg(pub String);

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

impl PubSubable for Msg {
    type Topic = Topic;

    fn topic(&self) -> Self::Topic {
        if let Ok(msg) = format::from_str::<InstallAck>(&self.0) {
            Topic::InstallAck(msg.name)
        } else if let Ok(msg) = format::from_str::<UninstallAck>(&self.0) {
            Topic::UninstallAck(msg.name)
        } else if let Ok(msg) = format::from_str::<WatchAck>(&self.0) {
            Topic::WatchAck(msg.name)
        } else if let Ok(msg) = format::from_str::<UnwatchAck>(&self.0) {
            Topic::UnwatchAck(msg.name)
        } else if let Ok(msg) = format::from_str::<SetLocalAck>(&self.0) {
            Topic::SetLocalAck(msg.name)
        } else if format::from_str::<Message>(&self.0).is_ok() {
            Topic::Message
        } else if let Ok(msg) = format::from_str::<MessageAck>(&self.0) {
            Topic::MessageAck(msg.id)
        } else if format::from_str::<QuitAck>(&self.0).is_ok() {
            Topic::QuitAck
        } else {
            Topic::Other
        }
    }
}
