use facet::{Def, Facet, HasFields, Peek, PeekMap, PeekOption};

#[derive(Default)]
struct Serializer {
    parts: Vec<String>,
}

impl Serializer {
    fn serialize_tag(&mut self, tag: &str) {
        self.parts.push(tag.to_string());
    }

    fn serialize_scalar(&mut self, peek: Peek<'_, 'static>) {
        self.parts
            .push(super::upcode::encode(&peek.to_string()).into_owned());
    }

    fn serialize_option(&mut self, peek: PeekOption<'_, 'static>, has_default: bool) {
        match peek.value() {
            None if has_default => (),
            None => self.parts.push(Default::default()),
            Some(peek) => self.serialize_value(peek, has_default),
        }
    }

    fn serialize_map(&mut self, peek: PeekMap<'_, 'static>) {
        for (k, v) in peek.iter() {
            self.serialize_value(k, false);
            let k = self.parts.pop().expect("key not serialized");

            self.serialize_value(v, false);
            let v = self.parts.pop().expect("value not serialized");

            self.parts.push(format!("{k}={v}"));
        }
    }

    fn serialize_value(&mut self, peek: Peek<'_, 'static>, has_default: bool) {
        if let Some(tag) = peek.shape().type_tag {
            self.serialize_tag(tag);
        };

        if let Ok(peek) = peek.into_struct() {
            for (item, peek) in peek.fields_for_serialize() {
                self.serialize_value(peek, has_default || item.field.has_default());
            }
        } else if let Ok(peek) = peek.into_option() {
            self.serialize_option(peek, has_default);
        } else if let Ok(peek) = peek.into_map() {
            self.serialize_map(peek);
        } else {
            match peek.shape().def {
                Def::Scalar => self.serialize_scalar(peek),

                _ => panic!(
                    "unable to serialize type `{}`, stopped at {:?}",
                    peek.shape().type_identifier,
                    self.parts
                ),
            }
        }
    }

    pub fn serialize(mut self, peek: Peek<'_, 'static>) -> String {
        self.serialize_value(peek, false);

        self.parts.join(":")
    }
}

/// Serialize an instance of `T` to it's textual representation.
pub fn to_string<T: Facet<'static>>(value: &T) -> String {
    let peek = Peek::new(value);
    let ser = Serializer::default();

    ser.serialize(peek)
}
