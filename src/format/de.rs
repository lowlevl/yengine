use std::collections::VecDeque;

use facet::{Def, Facet, Partial, Type, UserType};

use super::{Error, Result};

struct Deserializer<'de> {
    parts: VecDeque<&'de str>,
}

impl<'de> Deserializer<'de> {
    fn deserialize_tag(&mut self) -> Result<&str> {
        self.parts.pop_front().ok_or(Error::MissingTag)
    }

    fn deserialize_scalar(&mut self, partial: Partial<'static>) -> Result<Partial<'static>> {
        match self.parts.pop_front() {
            Some(value) => Ok(partial.parse_from_str(value)?),
            None => Err(Error::MissingValue),
        }
    }

    fn deserialize_option(
        &mut self,
        mut partial: Partial<'static>,
        has_default: bool,
    ) -> Result<Partial<'static>> {
        match self.parts.front() {
            Some(&"") => {
                self.parts.pop_front();

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

    fn deserialize_map(
        &mut self,
        mut partial: Partial<'static>,
        has_default: bool,
    ) -> Result<Partial<'static>> {
        partial = partial.begin_map()?;

        self.parts = self
            .parts
            .drain(..)
            .map(|kv| kv.split_once('=').ok_or(Error::MisformatedMap))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flat_map(|(k, v)| [k, v])
            .collect();

        while self.parts.front().is_some() {
            partial = partial.begin_key()?;
            partial = self.deserialize_value(partial, has_default)?;
            partial = partial.end()?;

            partial = partial.begin_value()?;
            partial = self.deserialize_value(partial, has_default)?;
            partial = partial.end()?;
        }

        Ok(partial)
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

        if self.parts.front().is_none() && has_default {
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
                Def::Map(_) => self.deserialize_map(partial, has_default),

                _ => panic!(
                    "unable to deserialize type `{}`",
                    partial.shape().type_identifier
                ),
            },
        }
    }

    fn deserialize<T: facet::Facet<'static>>(mut self, partial: Partial<'static>) -> Result<T> {
        self.deserialize_value(partial, Default::default())?
            .build()?
            .materialize()
            .map_err(Into::into)
    }
}

/// Deserialize an instance of `T` from it's textual representation.
pub fn from_str<T: Facet<'static>>(input: &str) -> Result<T> {
    let partial = Partial::alloc::<T>()?;
    let de = Deserializer {
        parts: input.split(":").collect(),
    };

    de.deserialize(partial)
}
