use std::str::Split;

use facet::{Def, Facet, FieldFlags, Partial, Type, TypedPartial, UserType};

use super::{Error, Result};

struct Deserializer<'de, T> {
    parts: Split<'de, &'static str>,
    partial: TypedPartial<'de, T>,
}

impl<'de, T: Facet<'de>> Deserializer<'de, T> {
    fn deserialize_tag(&mut self) -> Result<&str> {
        self.parts.next().ok_or(Error::MissingTag)
    }

    fn deserialize_scalar(&mut self, idx: usize) -> Result<()> {
        let Some(value) = self.parts.next() else {
            return Err(Error::MissingValue);
        };

        self.partial
            .begin_nth_field(idx)?
            .begin_custom_deserialization()?
            .parse_from_str(value)?
            .end()?;

        Ok(())
    }

    fn deserialize_map(&mut self, idx: usize) -> Result<()> {
        self.partial
            .begin_nth_field(idx)?
            .begin_custom_deserialization()?
            .begin_map()?;

        for value in self.parts.by_ref() {
            let Some((name, value)) = value.split_once('=') else {
                return Err(Error::MisformatedMap);
            };

            // NOTE: could be a recursive deserialization of k,v too
            self.partial
                .begin_key()?
                .parse_from_str(name)?
                .end()?
                .begin_value()?
                .parse_from_str(value)?
                .end()?;
        }

        self.partial.end()?;

        Ok(())
    }

    fn deserialize(mut self) -> Result<T> {
        if self.partial.shape().type_tag != Some(self.deserialize_tag()?) {
            return Err(Error::MismatchedTag);
        }

        let Type::User(UserType::Struct(st)) = self.partial.shape().ty else {
            panic!(
                "type `{}` is not a struct",
                self.partial.shape().type_identifier
            )
        };

        for (idx, field) in st.fields.iter().enumerate() {
            match field.shape().def {
                Def::Scalar => self.deserialize_scalar(idx)?,
                Def::Map(_) if field.flags.contains(FieldFlags::FLATTEN) => {
                    self.deserialize_map(idx)?
                }

                _ => panic!(
                    "unable to deserialize type `{}`",
                    field.shape().type_identifier
                ),
            }
        }

        let boxed = self.partial.build()?;
        Ok(*boxed)
    }
}

/// Deserialize an instance of `T` from it's textual representation.
pub fn from_str<'de, T: Facet<'de>>(input: &'de str) -> Result<T> {
    let partial = Partial::alloc::<T>()?;
    let de = Deserializer {
        parts: input.split(":"),
        partial,
    };

    de.deserialize()
}
