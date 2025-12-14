use std::str::Split;

use facet::{Def, Facet, FieldFlags, Partial, Type, UserType};

use super::{Error, Result};

struct Deserializer<'de> {
    parts: Split<'de, &'static str>,
}

impl<'de> Deserializer<'de> {
    fn deserialize_tag(&mut self) -> Result<&str> {
        self.parts.next().ok_or(Error::MissingTag)
    }

    fn deserialize_scalar(&mut self, partial: Partial<'de>, idx: usize) -> Result<Partial<'de>> {
        let Some(value) = self.parts.next() else {
            return Err(Error::MissingValue);
        };

        Ok(partial
            .begin_nth_field(idx)?
            .begin_custom_deserialization()?
            .parse_from_str(value)?
            .end()?)
    }

    fn deserialize_map(&mut self, mut partial: Partial<'de>, idx: usize) -> Result<Partial<'de>> {
        partial = partial
            .begin_nth_field(idx)?
            .begin_custom_deserialization()?
            .begin_map()?;

        for value in self.parts.by_ref() {
            let Some((name, value)) = value.split_once('=') else {
                return Err(Error::MisformatedMap);
            };

            // NOTE: may be a recursive deserialization of k,v too
            partial = partial
                .begin_key()?
                .parse_from_str(name)?
                .end()?
                .begin_value()?
                .parse_from_str(value)?
                .end()?;
        }

        Ok(partial.end()?)
    }

    fn deserialize<T: facet::Facet<'de>>(mut self, mut partial: Partial<'de>) -> Result<T> {
        if partial.shape().type_tag != Some(self.deserialize_tag()?) {
            return Err(Error::MismatchedTag);
        }

        let Type::User(UserType::Struct(st)) = partial.shape().ty else {
            panic!("type `{}` is not a struct", partial.shape().type_identifier)
        };

        for (idx, field) in st.fields.iter().enumerate() {
            match field.shape().def {
                Def::Scalar => partial = self.deserialize_scalar(partial, idx)?,
                Def::Map(_) if field.flags.contains(FieldFlags::FLATTEN) => {
                    partial = self.deserialize_map(partial, idx)?
                }

                _ => panic!(
                    "unable to deserialize type `{}`",
                    field.shape().type_identifier
                ),
            }
        }

        partial.build()?.materialize().map_err(Into::into)
    }
}

/// Deserialize an instance of `T` from it's textual representation.
pub fn from_str<'de, T: Facet<'de>>(input: &'de str) -> Result<T> {
    let partial = Partial::alloc::<T>()?;
    let de = Deserializer {
        parts: input.split(":"),
    };

    de.deserialize(partial)
}
