use std::iter::once;

use ty::WorldCoords;

#[derive(Copy, Clone, Debug)]
pub enum TerrainVertex {
    Surface(WorldCoords),
    Inner(WorldCoords)
}

impl TerrainVertex {
    pub fn coords(&self) -> WorldCoords {
        match *self {
            TerrainVertex::Surface(coord) | TerrainVertex::Inner(coord) => coord
        }
    }
}
    
pub struct TerrainMesh {
    mesh: Vec<TerrainVertex>
}

impl TerrainMesh {
    pub fn new(mesh: Vec<TerrainVertex>) -> TerrainMesh {
        TerrainMesh {
            mesh: mesh
        }
    }

    pub fn mesh_vertices<'a>(&'a self) -> impl Iterator<Item=WorldCoords> + 'a {
        self.mesh.iter().map(|vertex| vertex.coords())
    }

    // Calculate triangle_strip indices
    pub fn mesh_indices(&self, offset: u32) -> impl Iterator<Item=u32> {
        
        let mut indices: Vec<u32> = Vec::new();

        let mut base = 0;
        let mut start = true;
        let mut last = None;

        for (index, vertex) in self.mesh.iter().enumerate().skip(1) {
            let index = index as u32;

            if start {
                indices.extend_from_slice(&[base + offset]);
                start = false;
            } else {
                start = true;
            }

            match *vertex {
                TerrainVertex::Surface(_) => {
                    indices.extend_from_slice(&[index + offset]);
                    last = Some(index + offset);
                },

                TerrainVertex::Inner(_) => {
                    indices.extend_from_slice(&[index + offset, RESTART, last.unwrap()]);
                    base = index;
                    start = true;
                }
            }
        }

        indices.into_iter().chain(once(RESTART))
    }
}

static RESTART: u32 = 0xffffffff;
