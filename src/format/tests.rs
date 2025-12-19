use std::fmt::Debug;

use facet::Facet;

use super::*;

fn test<T: Facet<'static> + Debug>(input: &str) {
    let deserialized = from_str::<T>(input).unwrap_or_else(|err| {
        panic!(
            "can't deserialize `{}` from `{input}`: {err}",
            T::SHAPE.type_identifier
        )
    });
    eprintln!("{deserialized:?}");

    let serialized = to_string(&deserialized);
    assert_eq!(input, serialized, "for {}", T::SHAPE.type_identifier)
}

#[test]
fn error_in() {
    test::<ErrorIn>("Error in:this is an error");
}

#[test]
fn message() {
    test::<Message>("%%>message:yengine.1.1:1095112795:engine.timer:");
    test::<Message>("%%>message:yengine.1.2:1095112795:engine.timer:1234");
    test::<Message>("%%>message:yengine.1.3:1095112795:engine.timer::time=1095112795");
    test::<Message>(
        "%%>message:yengine.1.4:1095112794:app.job::done=75%%:job=cleanup:path=/bin%Z/usr/bin",
    );
}

#[test]
fn message_ack() {
    test::<MessageAck>("%%<message:234479208:false:engine.timer::time=1095112795");
    test::<MessageAck>("%%<message:234479288:false:engine.timer::extra=true:time=1095112796");
    test::<MessageAck>(
        "%%<message:yengine.1.4:true:app.job:Restart required:path=/bin%Z/usr/bin%Z/usr/local/bin",
    );
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
