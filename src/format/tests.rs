use facet::Facet;

use super::*;

fn test<T: Facet<'static>>(input: &str) {
    let deserialized = from_str::<T>(input).unwrap_or_else(|err| {
        panic!(
            "can't deserialize `{}` from `{input}`: {err}",
            T::SHAPE.type_identifier
        )
    });
    let serialized = to_string(&deserialized);

    assert_eq!(input, serialized, "for {}", T::SHAPE.type_identifier)
}

#[test]
fn error_in() {
    test::<ErrorIn>("Error in:this is an error");
}

#[test]
fn install() {
    test::<Install>("%%>install::engine.timer");
    test::<Install>("%%>install:50:engine.timer");
    test::<Install>("%%>install::engine.timer:key");
    test::<Install>("%%>install:50:engine.timer:key:value");
}

#[test]
fn install_ack() {
    test::<InstallAck>("%%<install:100:engine.timer:true");
    test::<InstallAck>("%%<install:50:engine.timer:false");
}

#[test]
fn uninstall() {
    test::<Uninstall>("%%>uninstall:engine.timer");
}

#[test]
fn uninstall_ack() {
    test::<UninstallAck>("%%<uninstall:50:engine.timer:true");
    test::<UninstallAck>("%%<uninstall:100:engine.timer:false");
}

#[test]
fn watch() {
    test::<Watch>("%%>watch:engine.timer");
}

#[test]
fn watch_ack() {
    test::<WatchAck>("%%<watch:engine.timer:true");
    test::<WatchAck>("%%<watch:engine.timer:false");
}

#[test]
fn unwatch() {
    test::<Unwatch>("%%>unwatch:engine.timer");
}

#[test]
fn unwatch_ack() {
    test::<UnwatchAck>("%%<unwatch:engine.timer:true");
    test::<UnwatchAck>("%%<unwatch:engine.timer:false");
}

#[test]
fn setlocal() {
    test::<SetLocal>("%%>setlocal:trackparam:yengine.1");
    test::<SetLocal>("%%>setlocal:trackparam:");
}

#[test]
fn setlocal_ack() {
    test::<SetLocalAck>("%%<setlocal:trackparam:yengine.1:true");
    test::<SetLocalAck>("%%<setlocal:trackparam:yengine.1:false");
}

#[test]
fn output() {
    test::<Output>("%%>output:this is getting logged");
}

#[test]
fn quit() {
    test::<Quit>("%%>quit");
}

#[test]
fn quit_ack() {
    test::<QuitAck>("%%<quit");
}
