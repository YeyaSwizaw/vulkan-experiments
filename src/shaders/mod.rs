use std::fmt;

pub mod sprite {
    mod v {
        #[derive(VulkanoShader)]
        #[ty = "vertex"]
        #[path = "src/shaders/sprite_vertex.glsl"]
        struct Dummy;
    }

    mod f {
        #[derive(VulkanoShader)]
        #[ty = "fragment"]
        #[path = "src/shaders/sprite_fragment.glsl"]
        struct Dummy;
    }

    pub use self::v::Shader as vertex;
    pub use self::f::Shader as fragment;

    pub use self::v::ty::DisplayUniforms;
}

impl fmt::Debug for sprite::DisplayUniforms {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "BOUNDS: {:?} !!!", self.bounds)
    }
}
