//! Format of messages in the Yate Engine external module protocol.
//!
//! ## Format of commands and notifications
//!
//! Every command is sent on its own newline (`\n`, `^J`, decimal `10`) delimited line.
//!
//! Any value that contains special characters (ASCII `<32`)
//! MUST have them converted to `%<upcode>` where `<upcode>` is the character
//! with a numeric value equal with `64 + original ASCII code`.
//!
//! The `%` character itself MUST be converted to a special `%%` representation.
//! Characters with codes `>=32` (except `%`) SHOULD not be escaped but may be so.
//!
//! A `%`-escaped code may be received instead of an unescaped character anywhere
//! except in the initial keyword or the delimiting colon (`:`) characters.
//!
//! Anywhere in the line except the initial keyword,
//! a `%` character not followed by a character with
//! a numeric value `>64` (`40H`, `0x40`, `'@'`)
//! or another `%` is an error.
//!
//! ## Command direction
//! Command direction is anotated by the following prefixes in the
//! structures documentations:
//! - **(>)**: _Application_ to _Engine_
//! - **(<)**: _Engine_ to _Application_
//! - **(~)**: _Bi_-directional

use std::collections::HashMap;

mod error;
pub use error::{Error, Result};

mod encoding;

mod de;
pub use de::*;

mod ser;
pub use ser::*;

/// **(<)** The engine sends this notification as answer to a syntactically
/// incorrect line it received from the application.
///
/// Note: _The external module SHOULD NOT send anything back to Yate
/// in response to such a notification as it can result in an infinite loop._
#[derive(Debug, facet::Facet)]
#[facet(type_tag = "Error in")]
pub struct ErrorIn {
    /// The original line exactly as received (not escaped or something).
    original: String,
}

/// **(~)**
#[derive(Debug, facet::Facet)]
#[facet(type_tag = "%%>message")]
pub struct MessageReq {
    id: String,
    time: u64,
    name: String,
    retvalue: String,

    #[facet(flatten)]
    kv: HashMap<String, String>,
}

/// **(~)**
#[derive(Debug, facet::Facet)]
#[facet(type_tag = "%%<message")]
pub struct MessageAck {
    id: String,
    time: u64,
    name: String,
    retvalue: String,

    #[facet(flatten)]
    kv: HashMap<String, String>,
}

/// **(>)** Requests the installing of a message **handler**.
#[derive(Debug, facet::Facet)]
#[facet(type_tag = "%%>install")]
pub struct InstallReq {
    /// Priority in chain, use default (`100`) if `None`.
    priority: Option<u64>,

    /// Name of the messages for that a handler should be installed.
    name: String,

    /// Filter for the installed handler;
    /// - name of a variable the handler will filter,
    /// - matching value for the filtered variable.
    #[facet(flatten)]
    filter: Option<(String, Option<String>)>,
}

/// **(<)** Confirmation that the **handler**
/// has been installed properly or not.
#[derive(Debug, facet::Facet)]
#[facet(type_tag = "%%<install")]
pub struct InstallAck {
    /// Priority of the installed handler.
    priority: u64,

    /// Name of the messages asked to handle.
    name: String,

    /// Success of operation.
    success: bool,
}

/// **(>)** Requests uninstalling a previously installed message **handler**.
#[derive(Debug, facet::Facet)]
#[facet(type_tag = "%%>uninstall")]
pub struct UninstallReq {
    /// Name of the message handler thst should be uninstalled.
    name: String,
}

/// **(<)** Confirmation that the **handler**
/// has been uninstalled properly or not.
#[derive(Debug, facet::Facet)]
#[facet(type_tag = "%%<uninstall")]
pub struct UninstallAck {
    /// Priority of the previously installed handler.
    priority: u64,

    /// Name of the message handler asked to uninstall.
    name: String,

    /// Success of operation.
    success: bool,
}

/// **(>)** Requests the installing of a message **watcher**
/// (post-dispatching notifier).
#[derive(Debug, facet::Facet)]
#[facet(type_tag = "%%>watch")]
pub struct WatchReq {
    /// Name of the messages for that a watcher should be installed.
    name: String,
}

/// **(<)** Confirmation that the **watcher**
/// has been installed properly or not.
#[derive(Debug, facet::Facet)]
#[facet(type_tag = "%%<watch")]
pub struct WatchAck {
    /// Name of the messages asked to watch.
    name: String,

    /// Success of operation.
    success: bool,
}

/// **(>)** Requests uninstalling a previously installed message **watcher**.
#[derive(Debug, facet::Facet)]
#[facet(type_tag = "%%>unwatch")]
pub struct UnwatchReq {
    /// Name of the message watcher thst should be uninstalled.
    name: String,
}

/// **(<)** Confirmation that the **watcher**
/// has been uninstalled properly or not.
#[derive(Debug, facet::Facet)]
#[facet(type_tag = "%%<unwatch")]
pub struct UnwatchAck {
    /// Name of the message watcher asked to uninstall.
    name: String,

    /// Success of operation.
    success: bool,
}

/// **(>)** Requests the change of a **local parameter**.
///
/// Currently supported parameters:
/// - `id` (string) - Identifier of the associated channel, if any
/// - `disconnected` (bool) - Enable or disable sending "chan.disconnected" messages
/// - `trackparam` (string) - Set the message handler tracking name, cannot be made empty
/// - `reason` (string) - Set the disconnect reason that gets received by the peer channel
/// - `timeout` (int) - Timeout in milliseconds for answering to messages
/// - `timebomb` (bool) - Terminate this module instance if a timeout occured
/// - `bufsize` (int) - Length of the incoming line buffer (default 8192)
/// - `setdata` (bool) - Attach channel pointer as user data to generated messages
/// - `reenter` (bool) - If this module is allowed to handle messages generated by itself
/// - `selfwatch` (bool) - If this module is allowed to watch messages generated by itself
/// - `restart` (bool) - Restart this global module if it terminates unexpectedly. Must be turned off to allow normal termination
///
/// Engine read-only run parameters:
/// - `engine.version` (string,readonly) - Version of the engine, like "2.0.1"
/// - `engine.release` (string,readonly) - Release type and number, like "beta2"
/// - `engine.nodename` (string,readonly) - Server's node name as known by the engine
/// - `engine.runid` (int,readonly) - Engine's run identifier
/// - `engine.configname` (string,readonly) - Name of the master configuration
/// - `engine.sharedpath` (string,readonly) - Path to the shared directory
/// - `engine.configpath` (string,readonly) - Path to the program config files directory
/// - `engine.cfgsuffix` (string,readonly) - Suffix of the config files names, normally ".conf"
/// - `engine.modulepath` (string,readonly) - Path to the main modules directory
/// - `engine.modsuffix` (string,readonly) - Suffix of the loadable modules, normally ".yate"
/// - `engine.logfile` (string,readonly) - Name of the log file if in use, empty if not logging
/// - `engine.clientmode` (bool,readonly) - Check if running as a client
/// - `engine.supervised` (bool,readonly) - Check if running under supervisor
/// - `engine.maxworkers` (int,readonly) - Maximum number of message worker threads
///
/// Engine configuration file parameters:
/// - `config.<section>.<key>` (readonly) - Content of `key=` in `[section]` of main config file (`yate.conf`, `yate-qt4.conf`)
#[derive(Debug, facet::Facet)]
#[facet(type_tag = "%%>setlocal")]
pub struct SetLocalReq {
    /// Name of the parameter to modify.
    name: String,

    /// New value to set in the local module instance,
    /// `None` to just query.
    value: Option<String>,
}

/// **(<)** Confirmation that the **local parameter**
/// has been changed successfully or not.
#[derive(Debug, facet::Facet)]
#[facet(type_tag = "%%<setlocal")]
pub struct SetLocalAck {
    /// Name of the modified parameter.
    name: String,

    /// Value of the local parameter.
    value: String,

    /// Success of operation.
    success: bool,
}

/// **(>)** The [`Output`] message is used to relay arbitrary
/// messages to engine's logging output.
///
/// This is the proper way of logging messages for programs
/// that connect to the socket interface as they may not
/// have the standard error redirected.
#[derive(Debug, facet::Facet)]
#[facet(type_tag = "%%>output")]
pub struct Output {
    /// Arbitrary unescaped string.
    text: String,
}

/// **(>)** The [`Connect`] message is used only by
/// external modules that attach to the socket interface.
///
/// As the conection is initiated from the external module
/// the engine must be informed on the role of the connection.
/// This must be the first request sent over a newly
/// established socket connection.
/// The role and direction of the connection is established
/// and then this keyword cannot be used again on the same connection.
///
/// There is no answer to this request, if it fails
/// the engine will slam the connection shut.
#[derive(Debug, facet::Facet)]
#[facet(type_tag = "%%>connect")]
pub struct Connect {
    /// Role of this connection: `global`, `channel`, `play`, `record` or `playrec`.
    role: String,

    /// Channel id to connect this socket to.
    id: Option<String>,

    /// Type of data channel, assuming `audio` if `None`.
    type_: Option<String>,
}
