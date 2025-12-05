use facet::{Def, Facet, FieldFlags, Partial, Type, UserType};

use super::{Error, Result};

pub fn from_str<T: Facet<'static>>(input: &str) -> Result<T> {
    let mut partial = Partial::alloc::<T>()?;

    let Some(tag) = partial.shape().type_tag else {
        panic!("unable to deserialize untagged types")
    };

    let mut parts = input.split(":");
    match parts.next() {
        None => Err(Error::MissingTag),
        Some(first) if first != tag => Err(Error::MismatchedTag(first.into(), tag)),

        Some(_) => {
            let Type::User(UserType::Struct(ty)) = partial.shape().ty else {
                panic!("unable to deserialize non-structs")
            };

            for (idx, field) in ty.fields.iter().enumerate() {
                let partial = partial.begin_nth_field(idx)?;

                match field.shape().def {
                    Def::Scalar => {
                        let Some(value) = parts.next() else {
                            return Err(Error::MissingField(field.shape()));
                        };

                        partial.parse_from_str(value)?;
                    }

                    Def::Map(_) if field.flags.contains(FieldFlags::FLATTEN) => {
                        let partial = partial.begin_map()?;

                        for value in parts.by_ref() {
                            let Some((name, value)) = value.split_once('=') else {
                                return Err(Error::MisformatedMap);
                            };

                            {
                                let partial = partial.begin_key()?;
                                partial.parse_from_str(name)?;
                                partial.end()?;
                            }

                            {
                                let partial = partial.begin_value()?;
                                partial.parse_from_str(value)?;
                                partial.end()?;
                            }
                        }
                    }
                    _ => panic!("unable to deserialize {:?}", field.shape()),
                };

                partial.end()?;
            }

            let boxed = partial.build()?;
            Ok(*boxed)
        }
    }
}
