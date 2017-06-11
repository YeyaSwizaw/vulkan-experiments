use ty::WorldRect;

pub struct Sprite {
    rect: WorldRect
}

impl Sprite {
    pub fn new(rect: WorldRect) -> Sprite {
        Sprite {
            rect: rect
        }
    }

    pub fn rect(&self) -> &WorldRect {
        &self.rect
    }
}
