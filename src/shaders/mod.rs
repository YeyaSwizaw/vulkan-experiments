use std::fmt;

use ty::WorldRect;

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

    pub use self::v::ty::{DisplayUniforms, SpriteUniforms};
}

pub mod terrain {
    mod v {
        #[derive(VulkanoShader)]
        #[ty = "vertex"]
        #[path = "src/shaders/terrain_vertex.glsl"]
        struct Dummy;
    }

    mod f {
        #[derive(VulkanoShader)]
        #[ty = "fragment"]
        #[path = "src/shaders/terrain_fragment.glsl"]
        struct Dummy;
    }

    pub use self::v::Shader as vertex;
    pub use self::f::Shader as fragment;
}

impl<'a> From<&'a WorldRect> for sprite::SpriteUniforms {
    fn from(rect: &'a WorldRect) -> sprite::SpriteUniforms {
        sprite::SpriteUniforms {
            pos: [rect.position.0, rect.position.1],
            bounds: [rect.bounds.0, rect.bounds.1],
        }
    }
}

impl From<WorldRect> for sprite::SpriteUniforms {
    fn from(rect: WorldRect) -> sprite::SpriteUniforms {
        sprite::SpriteUniforms::from(&rect)
    }
}
