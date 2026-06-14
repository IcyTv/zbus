use crate::{Error, Result, utils::define_name_type_impls};
use serde::Serialize;
use zvariant::{OwnedValue, Str, Type, Value};

/// String that identifies a [well-known bus name][wbn].
///
/// # Examples
///
/// ```
/// use zbus_names::WellKnownName;
///
/// // Valid well-known names.
/// let name = WellKnownName::try_from("org.gnome.Service-for_you").unwrap();
/// assert_eq!(name, "org.gnome.Service-for_you");
/// let name = WellKnownName::try_from("a.very.loooooooooooooooooo-ooooooo_0000o0ng.Name").unwrap();
/// assert_eq!(name, "a.very.loooooooooooooooooo-ooooooo_0000o0ng.Name");
///
/// // Invalid well-known names
/// WellKnownName::try_from("").unwrap_err();
/// WellKnownName::try_from("double..dots").unwrap_err();
/// WellKnownName::try_from(".").unwrap_err();
/// WellKnownName::try_from(".start.with.dot").unwrap_err();
/// WellKnownName::try_from("1st.element.starts.with.digit").unwrap_err();
/// WellKnownName::try_from("the.2nd.element.starts.with.digit").unwrap_err();
/// WellKnownName::try_from("no-dots").unwrap_err();
/// ```
///
/// [wbn]: https://dbus.freedesktop.org/doc/dbus-specification.html#message-protocol-names-bus
#[derive(
    Clone, Debug, Hash, PartialEq, Eq, Serialize, Type, Value, PartialOrd, Ord, OwnedValue,
)]
pub struct WellKnownName<'name>(pub(crate) Str<'name>);

/// Owned sibling of [`WellKnownName`].
#[derive(Clone, Hash, PartialEq, Eq, Serialize, Type, Value, PartialOrd, Ord, OwnedValue)]
pub struct OwnedWellKnownName(#[serde(borrow)] WellKnownName<'static>);

define_name_type_impls! {
    name: WellKnownName,
    owned: OwnedWellKnownName,
    validate: validate,
}

impl WellKnownName<'static> {
    /// Create a new static well-known bus name, validating it at compile time in const contexts.
    ///
    /// # Panics
    ///
    /// Panics if `name` is not a valid D-Bus well-known bus name.
    pub const fn from_static_str_checked(name: &'static str) -> Self {
        if !validate_bytes_const(name.as_bytes()) {
            panic!("invalid D-Bus well-known name");
        }

        Self::from_static_str_unchecked(name)
    }
}

fn validate(name: &str) -> Result<()> {
    validate_bytes(name.as_bytes()).map_err(|_| {
        Error::InvalidName(
            "Invalid well-known name. \
            See https://dbus.freedesktop.org/doc/dbus-specification.html#message-protocol-names-bus"
        )
    })
}

pub(crate) fn validate_bytes(bytes: &[u8]) -> std::result::Result<(), ()> {
    use winnow::{
        Parser,
        combinator::separated,
        stream::AsChar,
        token::{one_of, take_while},
    };
    // Rules
    //
    // * Only ASCII alphanumeric, `_` or '-'.
    // * Must not begin with a `.`.
    // * Must contain at least one `.`.
    // * Each element must:
    //  * not begin with a digit.
    //  * be 1 character (so name must be minimum 3 characters long).
    // * <= 255 characters.
    let first_element_char = one_of((AsChar::is_alpha, b'_', b'-'));
    let subsequent_element_chars = take_while::<_, _, ()>(0.., (AsChar::is_alphanum, b'_', b'-'));
    let element = (first_element_char, subsequent_element_chars);
    let mut well_known_name = separated(2.., element, b'.');

    well_known_name
        .parse(bytes)
        .map_err(|_| ())
        .and_then(|_: ()| {
            // Least likely scenario so we check this last.
            if bytes.len() > 255 {
                return Err(());
            }

            Ok(())
        })
}

pub(crate) const fn validate_bytes_const(bytes: &[u8]) -> bool {
    if bytes.len() > 255 || bytes.len() < 3 {
        return false;
    }

    let mut idx = 0;
    let mut element_len = 0;
    let mut has_dot = false;
    let mut first = true;

    while idx < bytes.len() {
        let byte = bytes[idx];
        if byte == b'.' {
            if first || element_len == 0 || idx + 1 == bytes.len() {
                return false;
            }
            has_dot = true;
            element_len = 0;
            first = true;
        } else if first {
            if !byte.is_ascii_alphabetic() && byte != b'_' && byte != b'-' {
                return false;
            }
            element_len = 1;
            first = false;
        } else {
            if !byte.is_ascii_alphanumeric() && byte != b'_' && byte != b'-' {
                return false;
            }
            element_len += 1;
        }
        idx += 1;
    }

    has_dot && element_len > 0
}
