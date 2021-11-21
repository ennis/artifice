use std::u8;

#[repr(transparent)]
#[derive(Copy,Clone,Debug,Eq,PartialEq,Ord,PartialOrd,Hash)]
pub struct Norm<T>(pub T);

pub type f32x2 = [f32;2];
pub type f32x3 = [f32;4];
pub type f32x4 = [f32;4];

pub type u8x2 = [u8;2];
pub type u8x3 = [u8;3];
pub type u8x4 = [u8;4];

pub type unorm8x2 = [Norm<u8>;2];
pub type unorm8x3 = [Norm<u8>;3];
pub type unorm8x4 = [Norm<u8>;4];

pub type u16x2 = [u16;2];
pub type u16x3 = [u16;3];
pub type u16x4 = [u16;4];

pub type i32x2 = [i32;2];
pub type i32x3 = [i32;3];
pub type i32x4 = [i32;4];

pub type u32x2 = [u32;2];
pub type u32x3 = [u32;3];
pub type u32x4 = [u32;4];

pub type unorm16x2 = [Norm<u16>;2];
pub type unorm16x3 = [Norm<u16>;3];
pub type unorm16x4 = [Norm<u16>;4];

pub type unorm32x2 = [Norm<u32>;2];
pub type unorm32x3 = [Norm<u32>;3];
pub type unorm32x4 = [Norm<u32>;4];


#[cfg(test)]
mod tests {
    use super::*;

    struct Vertex {
        position: f32x2,
        texcoords: unorm16x2,
    }



    // static SCREEN_VERTICES: mlr::StaticBufferProxy<[Vertex; 6]>;
    // -> Deref<Target=[Vertex;6]>
    const SCREEN_VERTICES: [Vertex; 6] = {
        let (left, top, right, bottom) = (-1.0, -1.0, 1.0, 1.0);
        [
            Vertex { position: [left, top], texcoords: [0.0, 0.0] },
            Vertex { position: [right, top], texcoords: [1.0, 0.0] },
            Vertex { position: [left, bottom], texcoords: [0.0, 1.0] },
            Vertex { position: [left, bottom], texcoords: [0.0, 1.0] },
            Vertex { position: [right, top], texcoords: [1.0, 0.0] },
            Vertex { position: [right, bottom], texcoords: [1.0, 1.0] },
        ]
    };

    #[derive(Copy,Clone,Debug,Eq,PartialEq)]
    struct DrawIndexedParams {
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: u32,
        index_count: u32,
    }

    // abstracted storage and number of channels
    // -> only type (float or integer), compressed, multisampled, dimension

    struct ImageT<T, const Multisampled: bool> {

    }

    impl<T, const Multisampled: bool> ImageT<T, Multisampled> {

        pub fn width(&self) -> usize {
            unimplemented!()
        }

        pub fn height(&self) -> usize {
            unimplemented!()
        }

        pub fn depth(&self) -> usize {
            unimplemented!()
        }

    }

    type Image = ImageT<f32, false>;

    pub trait ColorAttachments {}

    pub trait VertexSource {}

    struct RasterizerState {

    }

    struct BlendState {

    }

    extern "mlr-intrinsic" fn draw_indexed
    <V: VertexSource,
        VertexInput, VertexOutput,
        FragmentInput, FragmentOutput,
        C: ColorAttachments>
    (
        params: DrawIndexedParams,
        // mlr::Buffer<T>
        vertices: &V,
        // mlr::ShaderFunction
        vertex_shader: fn(VertexInput) -> VertexOutput,
        // mlr::ShaderFunction
        fragment_shader: fn(FragmentInput) -> FragmentOutput,
        rasterizer_state: RasterizerState,
        blend_state: BlendState,
        // mlr::DrawTargets
        targets: &mut C
    )
    {
        //

    }

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
