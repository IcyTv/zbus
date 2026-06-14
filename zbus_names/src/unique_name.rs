use crate::{Error, Result, utils::define_name_type_impls};
use serde::Serialize;
use zvariant::{OwnedValue, Str, Type, Value};

/// String that identifies a [unique bus name][ubn].
///
/// # Examples
///
/// ```
/// use zbus_names::UniqueName;
///
/// // Valid unique names.
/// let name = UniqueName::try_from(":org.gnome.Service-for_you").unwrap();
/// assert_eq!(name, ":org.gnome.Service-for_you");
/// let name = UniqueName::try_from(":a.very.loooooooooooooooooo-ooooooo_0000o0ng.Name").unwrap();
/// assert_eq!(name, ":a.very.loooooooooooooooooo-ooooooo_0000o0ng.Name");
///
/// // Invalid unique names
/// UniqueName::try_from("").unwrap_err();
/// UniqueName::try_from("dont.start.with.a.colon").unwrap_err();
/// UniqueName::try_from(":double..dots").unwrap_err();
/// UniqueName::try_from(".").unwrap_err();
/// UniqueName::try_from(".start.with.dot").unwrap_err();
/// UniqueName::try_from(":no-dots").unwrap_err();
/// ```
///
/// [ubn]: https://dbus.freedesktop.org/doc/dbus-specification.html#message-protocol-names-bus
#[derive(
    Clone, Debug, Hash, PartialEq, Eq, Serialize, Type, Value, PartialOrd, Ord, OwnedValue,
)]
pub struct UniqueName<'name>(pub(crate) Str<'name>);

/// Owned sibling of [`UniqueName`].
#[derive(Clone, Hash, PartialEq, Eq, Serialize, Type, Value, PartialOrd, Ord, OwnedValue)]
pub struct OwnedUniqueName(#[serde(borrow)] UniqueName<'static>);

define_name_type_impls! {
    name: UniqueName,
    owned: OwnedUniqueName,
    validate: validate,
}

impl UniqueName<'static> {
    /// Create a new static unique bus name, validating it at compile time in const contexts.
    ///
    /// # Panics
    ///
    /// Panics if `name` is not a valid D-Bus unique bus name.
    pub const fn from_static_str_checked(name: &'static str) -> Self {
        if !validate_bytes_const(name.as_bytes()) {
            panic!("invalid D-Bus unique name");
        }

        Self::from_static_str_unchecked(name)
    }
}

fn validate(name: &str) -> Result<()> {
    validate_bytes(name.as_bytes()).map_err(|_| {
        Error::InvalidName(
            "Invalid unique name. \
            See https://dbus.freedesktop.org/doc/dbus-specification.html#message-protocol-names-bus"
        )
    })
}

pub(crate) fn validate_bytes(bytes: &[u8]) -> std::result::Result<(), ()> {
    use winnow::{
        Parser,
        combinator::{alt, separated},
        stream::AsChar,
        token::take_while,
    };
    // Rules
    //
    // * Only ASCII alphanumeric, `_` or '-'
    // * Must begin with a `:`.
    // * Must contain at least one `.`.
    // * Each element must be 1 character (so name must be minimum 4 characters long).
    // * <= 255 characters.
    let element = take_while::<_, _, ()>(1.., (AsChar::is_alphanum, b'_', b'-'));
    let peer_name = (b':', (separated(2.., element, b'.'))).map(|_: (_, ())| ());
    let bus_name = b"org.freedesktop.DBus".map(|_| ());
    let mut unique_name = alt((bus_name, peer_name));

    unique_name.parse(bytes).map_err(|_| ()).and_then(|_: ()| {
        // Least likely scenario so we check this last.
        if bytes.len() > 255 {
            return Err(());
        }

        Ok(())
    })
}

pub(crate) const fn validate_bytes_const(bytes: &[u8]) -> bool {
    if bytes_eq(bytes, b"org.freedesktop.DBus") {
        return true;
    }
    if bytes.len() > 255 || bytes.len() < 4 || bytes[0] != b':' {
        return false;
    }

    let mut idx = 1;
    let mut element_len = 0;
    let mut has_dot = false;

    while idx < bytes.len() {
        let byte = bytes[idx];
        if byte == b'.' {
            if element_len == 0 || idx + 1 == bytes.len() {
                return false;
            }
            has_dot = true;
            element_len = 0;
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

const fn bytes_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let mut idx = 0;
    while idx < left.len() {
        if left[idx] != right[idx] {
            return false;
        }
        idx += 1;
    }

    true
}
