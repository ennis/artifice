use kyute::Data;
use std::cmp::Ordering;

/// Value variability.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Data)]
pub enum Variability {
    /// Vertex-varying (in vertex shaders)
    Vertex,
    /// Fragment-varying (in fragment shaders)
    Fragment,
    /// Per-instance value.
    DrawInstance,
    /// Per-object value.
    Object,
    /// Per-material value.
    Material,
    /// Time-varying
    TimeVarying,
    /// Constant (until UI changes)
    Constant,
}

fn lesser_variability(a: Variability, b: Variability) -> bool {
    // ordering relations:
    // Constant > TimeVarying > |Material             | > |Vertex   |
    //                          |Object > DrawInstance|   |Fragment |
    match a {
        Variability::Vertex => false,
        Variability::Fragment => false,
        Variability::DrawInstance => b == Variability::Fragment || b == Variability::Vertex,
        Variability::Object => b == Variability::DrawInstance || b == Variability::Fragment || b == Variability::Vertex,
        Variability::Material => b == Variability::Fragment || b == Variability::Vertex,
        Variability::TimeVarying => {
            b == Variability::Fragment
                || b == Variability::Vertex
                || b == Variability::Fragment
                || b == Variability::Object
                || b == Variability::DrawInstance
                || b == Variability::Material
        }
        Variability::Constant => b != Variability::Constant,
    }
}

impl PartialOrd for Variability {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            Some(Ordering::Equal)
        } else if lesser_variability(*self, *other) {
            Some(Ordering::Less)
        } else if lesser_variability(*other, *self) {
            Some(Ordering::Greater)
        } else {
            None
        }
    }
}
