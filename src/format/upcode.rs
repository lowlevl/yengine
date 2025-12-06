//! Upcodes encoding & decoding for the Yate Engine external module protocol.
//!
//! ## Format of strings and `%`-based encoding
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
//! _see <https://docs.yate.ro/wiki/External_module_command_flow#Format_of_commands_and_notifications>_.

use std::borrow::Cow;

use thiserror::Error;

/// An error that may occur while decoding `%`-encoded strings.
#[derive(Debug, Error)]
#[error("invalid upcode `{0}`, not in 64..=127 range")]
pub struct DecodeError(char);

fn updecode(ch: char) -> Result<char, DecodeError> {
    if ch == '%' {
        Ok(ch)
    } else {
        match u8::try_from(ch) {
            Ok(code @ 64..=127) => Ok(char::from(code - 64)),
            _ => Err(DecodeError(ch)),
        }
    }
}

/// Decode a `%`-encoded string in the context of value parsing.
pub fn decode(value: &str) -> Result<Cow<'_, str>, DecodeError> {
    if !value.contains('%') {
        return Ok(value.into());
    }

    let mut decoded = String::with_capacity(value.len());
    let mut decoding = false;
    for ch in value.chars() {
        if decoding {
            decoding = false;
            decoded.push(updecode(ch)?);
        } else if ch == '%' {
            decoding = true;
        } else {
            decoded.push(ch);
        }
    }

    Ok(decoded.into())
}

fn upencode(ch: char) -> char {
    if ch == '%' {
        ch
    } else {
        char::from(ch as u8 + 64)
    }
}

/// Encode a `%`-encoded string in the context of value encoding.
pub fn encode(value: &str) -> Cow<'_, str> {
    let pred = |ch: &char| ch.is_ascii_control() || matches!(ch, '%' | ':');
    let encodable = value.chars().filter(pred).count();

    if encodable == 0 {
        return value.into();
    }

    let mut encoded = String::with_capacity(value.len() + encodable);
    for ch in value.chars() {
        if pred(&ch) {
            encoded.push('%');
            encoded.push(upencode(ch))
        } else {
            encoded.push(ch);
        }
    }

    encoded.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zc_decoding() {
        assert!(matches!(decode("123456"), Ok(Cow::Borrowed(_))));
        assert!(matches!(decode("engine.timer"), Ok(Cow::Borrowed(_))));
    }

    #[test]
    fn zc_encoding() {
        assert!(matches!(encode("123456"), Cow::Borrowed(_)));
        assert!(matches!(encode("engine.timer"), Cow::Borrowed(_)));
    }

    #[test]
    fn it_decodes() {
        assert_eq!(
            decode("a%%null%%separated%%string").unwrap(),
            "a%null%separated%string"
        );

        assert_eq!(
            decode("a%@null%@separated%@string").unwrap(),
            "a\0null\0separated\0string"
        );

        assert_eq!(
            decode("a%znull%zseparated%zstring").unwrap(),
            "a:null:separated:string"
        );

        assert_eq!(
            decode("a%\x7fnull%\x7fseparated%\x7fstring").unwrap(),
            "a?null?separated?string"
        );
    }

    #[test]
    fn it_encodes() {
        assert_eq!(
            encode("a%null%separated%string"),
            "a%%null%%separated%%string"
        );

        assert_eq!(
            encode("a\0null\0separated\0string"),
            "a%@null%@separated%@string"
        );

        assert_eq!(
            encode("a:null:separated:string"),
            "a%znull%zseparated%zstring"
        );
    }

    #[test]
    fn its_consistent() {
        assert_eq!(encode(&decode("engine.timer").unwrap()), "engine.timer");
        assert_eq!(decode(&encode("engine.timer")).unwrap(), "engine.timer");

        assert_eq!(encode(&decode("some Ùtf̵-8").unwrap()), "some Ùtf̵-8");
        assert_eq!(decode(&encode("some Ùtf̵-8")).unwrap(), "some Ùtf̵-8");

        assert_eq!(encode(&decode("%@%%%z%\\?").unwrap()), "%@%%%z%\\?");
        assert_eq!(decode(&encode("\0%:\\?")).unwrap(), "\0%:\\?");
    }

    #[test]
    fn it_rejects_bad_upcodes() {
        assert!(decode("%\n").is_err());
        assert!(decode("%\0").is_err());
        assert!(decode("%:").is_err());
        assert!(decode("%0").is_err());
        assert!(decode("%™").is_err());
        assert!(decode("% ").is_err());
    }
}
