use facet::{Def, Facet, HasFields, Peek, PeekOption};

#[derive(Default)]
struct Serializer {
    parts: Vec<String>,
}

impl Serializer {
    fn serialize_tag(&mut self, tag: &str) {
        self.parts.push(tag.to_string());
    }

    fn serialize_scalar(&mut self, peek: Peek<'_, 'static>) {
        self.parts.push(peek.to_string());
    }

    fn serialize_option(&mut self, peek: PeekOption<'_, 'static>, has_default: bool) {
        match peek.value() {
            None if has_default => (),
            None => self.parts.push(Default::default()),
            Some(peek) => self.serialize_value(peek, has_default),
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
