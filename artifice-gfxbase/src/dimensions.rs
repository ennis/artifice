use std::fmt;

/// Dimensions of an image.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum Dimensions {
    /// 1D image
    Dim1d { width: u32, array_layers: u32 },
    /// 2D image
    Dim2d {
        width: u32,
        height: u32,
        array_layers: u32,
    },
    /// 3D image
    Dim3d { width: u32, height: u32, depth: u32 },
    /// Cubemap image (6 2D images)
    Cubemap { size: u32, array_layers: u32 },
}

impl Dimensions {
    /// Returns the width in pixels.
    #[inline]
    pub fn width(&self) -> u32 {
        match *self {
            Dimensions::Dim1d { width, .. } => width,
            Dimensions::Dim2d { width, .. } => width,
            Dimensions::Dim3d { width, .. } => width,
            Dimensions::Cubemap { size, .. } => size,
        }
    }

    /// Returns the height in pixels.
    ///
    /// Returns 1 for 1D images.
    #[inline]
    pub fn height(&self) -> u32 {
        match *self {
            Dimensions::Dim1d { .. } => 1,
            Dimensions::Dim2d { height, .. } => height,
            Dimensions::Dim3d { height, .. } => height,
            Dimensions::Cubemap { size, .. } => size,
        }
    }

    /// Returns the (width,height) pair.
    ///
    /// Equivalent to `(self.width(), self.height())`
    #[inline]
    pub fn width_height(&self) -> (u32, u32) {
        (self.width(), self.height())
    }

    /// Returns the depth (third dimension) of the image.
    ///
    /// Returns 1 for 1D, 2D or cubemap images.
    #[inline]
    pub fn depth(&self) -> u32 {
        match *self {
            Dimensions::Dim1d { .. } => 1,
            Dimensions::Dim2d { .. } => 1,
            Dimensions::Dim3d { depth, .. } => depth,
            Dimensions::Cubemap { .. } => 1,
        }
    }

    /// Returns the (width,height,depth) triplet.
    ///
    /// Equivalent to `(self.width(), self.height(), self.depth())`
    #[inline]
    pub fn width_height_depth(&self) -> (u32, u32, u32) {
        (self.width(), self.height(), self.depth())
    }

    #[inline]
    pub fn array_layers(&self) -> u32 {
        match *self {
            Dimensions::Dim1d { array_layers, .. } => array_layers,
            Dimensions::Dim2d { array_layers, .. } => array_layers,
            Dimensions::Dim3d { .. } => 1,
            Dimensions::Cubemap { array_layers, .. } => array_layers,
        }
    }

    #[inline]
    pub fn array_layers_with_cube(&self) -> u32 {
        match *self {
            Dimensions::Dim1d { array_layers, .. } => array_layers,
            Dimensions::Dim2d { array_layers, .. } => array_layers,
            Dimensions::Dim3d { .. } => 1,
            Dimensions::Cubemap { array_layers, .. } => array_layers * 6,
        }
    }
}

impl From<(u32, u32)> for Dimensions {
    fn from((width, height): (u32, u32)) -> Dimensions {
        Dimensions::Dim2d {
            width,
            height,
            array_layers: 1,
        }
    }
}

impl fmt::Debug for Dimensions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Dimensions::Dim1d {
                width,
                array_layers,
            } => {
                if *array_layers == 1 {
                    write!(f, "[1D {}]", width)
                } else {
                    write!(f, "[1D Array {}(x{})]", width, array_layers)
                }
            }
            Dimensions::Dim2d {
                width,
                height,
                array_layers,
            } => {
                if *array_layers == 1 {
                    write!(f, "[2D {}x{}]", width, height)
                } else {
                    write!(f, "[2D Array {}x{}(x{})]", width, height, array_layers)
                }
            }
            Dimensions::Dim3d {
                width,
                height,
                depth,
            } => write!(f, "[3D {}x{}x{}]", width, height, depth),
            Dimensions::Cubemap { size, array_layers } => {
                if *array_layers == 1 {
                    write!(f, "[Cubemap {}x{}]", size, size)
                } else {
                    write!(f, "[Cubemap Array {}x{}(x{})]", size, size, array_layers)
                }
            }
        }
    }
}
