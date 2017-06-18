use ty::WorldRect;

pub struct Sprite {
    pub rect: WorldRect
}

impl Sprite {
    pub fn new(rect: WorldRect) -> Sprite {
        Sprite {
            rect: rect
        }
    }
}
