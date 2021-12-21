use std::ops::Deref;
use string_cache::DefaultAtom;
use std::fmt;
use lazy_static::lazy_static;

/// Atom (interned string used for names)
#[derive(Clone, Debug, Eq, PartialEq, Hash, Default, serde::Serialize)]
pub struct Atom(DefaultAtom);

impl Deref for Atom {
    type Target = DefaultAtom;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl druid::Data for Atom {
    fn same(&self, other: &Self) -> bool {
        &self.0 == &other.0
    }
}

impl<T> From<T> for Atom
where
    DefaultAtom: From<T>,
{
    fn from(value: T) -> Self {
        Atom(DefaultAtom::from(value))
    }
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Helper function to adjust a name so that it doesn't clash with existing names.
pub fn make_unique_name<'a>(
    base_name: Atom,
    existing: impl Iterator<Item = &'a Atom> + Clone,
) -> Atom {
    let mut counter = 0;
    let mut disambiguated_name = base_name.clone();

    'check: loop {
        let existing = existing.clone();
        // check for property with the same name
        for name in existing {
            if name == &disambiguated_name {
                disambiguated_name = Atom::from(format!("{}_{}", base_name, counter));
                counter += 1;
                // restart check
                continue 'check;
            }
        }
        break;
    }

    disambiguated_name
}
