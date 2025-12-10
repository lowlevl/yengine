use facet::Facet;

/// Serialize an instance of `T` to it's textual representation.
pub fn to_string<T: Facet<'static>>(value: &T) -> String {
    Default::default()
}
