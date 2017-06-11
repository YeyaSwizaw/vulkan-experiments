#[derive(Copy, Clone, Debug)]
pub struct WorldCoords(pub i32, pub i32);

#[derive(Copy, Clone, Debug)]
pub struct WorldBounds(pub u32, pub u32);

#[derive(Clone, Debug)]
pub struct WorldRect {
    pub position: WorldCoords,
    pub bounds: WorldBounds,
}

