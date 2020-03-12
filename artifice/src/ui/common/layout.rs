use euclid::default::Vector2D;

pub type Size = Vector2D<f64>;

#[derive(Copy,Clone,Debug)]
pub struct BoxConstraints {
    pub min: Size,
    pub max: Size,
}

impl BoxConstraints {
    pub fn new(min: Size, max: Size) -> BoxConstraints {
        BoxConstraints {
            min,
            max
        }
    }

    pub fn tight(size: Size) -> BoxConstraints {
        BoxConstraints {
            min: size,
            max: size,
        }
    }

    pub fn contract(&self, amount: f64) -> BoxConstraints {
        let new_max = self.max - Size::new(amount, amount);
        let new_min = self.min.min(new_max);
        BoxConstraints {
            min: new_min,
            max: new_max,
        }
    }
}


// Layout in a vbox:
// call layout
//