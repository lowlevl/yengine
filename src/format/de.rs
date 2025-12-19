use std::{iter::Peekable, str::Split};

use facet::{Def, Facet, Partial, Type, UserType};

use super::{Error, Result};

struct Deserializer<'de> {
    parts: Peekable<Split<'de, &'static str>>,
}

impl<'de> Deserializer<'de> {
    fn deserialize_tag(&mut self) -> Result<&str> {
        self.parts.next().ok_or(Error::MissingTag)
    }

    // fn deserialize_map(&mut self, mut partial: Partial<'static>) -> Result<Partial<'static>> {
    //     partial = partial.begin_custom_deserialization()?.begin_map()?;
    //
    //     for value in self.parts.by_ref() {
    //         let Some((name, value)) = value.split_once('=') else {
    //             return Err(Error::MisformatedMap);
    //         };
    //
    //         // NOTE: may be a recursive deserialization of k,v too
    //         partial = partial
    //             .begin_key()?
    //             .parse_from_str(name)?
    //             .end()?
    //             .begin_value()?
    //             .parse_from_str(value)?
    //             .end()?;
    //     }
    //
    //     Ok(partial)
    // }

    fn deserialize_scalar(&mut self, partial: Partial<'static>) -> Result<Partial<'static>> {
        match self.parts.next() {
            Some(value) => Ok(partial.parse_from_str(value)?),
            None => Err(Error::MissingValue),
        }
    }

    fn deserialize_option(
        &mut self,
        mut partial: Partial<'static>,
        has_default: bool,
    ) -> Result<Partial<'static>> {
        match self.parts.peek() {
            Some(&"") => {
                self.parts.next();

                Ok(partial.set_default()?)
            }
            Some(_) => {
                partial = partial.begin_some()?;
                partial = self.deserialize_value(partial, has_default)?;

                Ok(partial.end()?)
            }
            None => Err(Error::MissingValue),
        }
    }

    fn deserialize_value(
        &mut self,
        mut partial: Partial<'static>,
        has_default: bool,
    ) -> Result<Partial<'static>> {
        if let Some(tag) = partial.shape().type_tag
            && tag != self.deserialize_tag()?
        {
            return Err(Error::MismatchedTag);
        }

        if self.parts.peek().is_none() && has_default {
            return Ok(partial.set_default()?);
        }

        match partial.shape().ty {
            Type::User(UserType::Struct(st)) => {
                for (idx, field) in st.fields.iter().enumerate() {
                    partial = partial.begin_nth_field(idx)?;
                    partial =
                        self.deserialize_value(partial, has_default || field.has_default())?;
                    partial = partial.end()?;
                }

                Ok(partial)
            }
            _ => match partial.shape().def {
                Def::Scalar => self.deserialize_scalar(partial),
                Def::Option(_) => self.deserialize_option(partial, has_default),

                _ => panic!(
                    "unable to deserialize type `{}`",
                    partial.shape().type_identifier
                ),
            },
        }
    }

    fn deserialize<T: facet::Facet<'static>>(mut self, partial: Partial<'static>) -> Result<T> {
        self.deserialize_value(partial, false)?
            .build()?
            .materialize()
            .map_err(Into::into)
    }
}

/// Deserialize an instance of `T` from it's textual representation.
pub fn from_str<T: Facet<'static>>(input: &str) -> Result<T> {
    let partial = Partial::alloc::<T>()?;
    let de = Deserializer {
        parts: input.split(":").peekable(),
    };

    de.deserialize(partial)
}
