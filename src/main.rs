#![feature(conservative_impl_trait)]

#[macro_use] extern crate stateloop;
#[macro_use] extern crate vulkano;
#[macro_use] extern crate vulkano_shader_derive;

extern crate vulkano_win;
extern crate winit;

use std::time::{Duration, Instant};
use stateloop::app::{App, Data, Event, Window};
use stateloop::state::Action;

use vulkano::instance::Instance;

use winit::{VirtualKeyCode, ElementState};

use renderer::Renderer;
use ty::{WorldCoords, WorldBounds, WorldRect};
use sprite::Sprite;
use terrain::{TerrainMesh, TerrainVertex};

mod shaders;
mod renderer;
mod ty;
mod sprite;
mod terrain;

states! {
    State {
        MainHandler Main()
    }
}

pub struct D {
    renderer: Renderer,

    key_states: Vec<ElementState>,
    sprites: Vec<Sprite>,
    terrain: TerrainMesh,

    last_frame_time: Instant
}

impl MainHandler for Data<D> {
    fn handle_event(&mut self, event: Event) -> Action<State> {
        let mut d = self.data_mut();

        match event {
            Event::Closed => return Action::Quit,

            Event::Resized(w, h) => {
                d.renderer.update_display_uniforms(w, h);
                Action::Continue
            },

            Event::KeyboardInput(state, _, Some(key), _) => {
                d.key_states[key as usize] = state;
                Action::Continue
            },

            _ => Action::Continue
        }
    }

    fn handle_tick(&mut self) {
        let mut d = self.data_mut();

        let next = Instant::now();
        // println!("Frame Time: {:?}", next - d.last_frame_time);
        d.last_frame_time = next;

        if d.key_states[VirtualKeyCode::W as usize] == ElementState::Pressed {
            d.sprites[0].rect.position.1 -= 5;
        }

        if d.key_states[VirtualKeyCode::S as usize] == ElementState::Pressed {
            d.sprites[0].rect.position.1 += 5;
        }

        if d.key_states[VirtualKeyCode::A as usize] == ElementState::Pressed {
            d.sprites[0].rect.position.0 -= 5;
        }

        if d.key_states[VirtualKeyCode::D as usize] == ElementState::Pressed {
            d.sprites[0].rect.position.0 += 5;
        }
    }

    fn handle_render(&self) {
        self.data().renderer.render(&self.data().sprites);
    }
}

fn main() {
    let instance = {
        let extensions = vulkano_win::required_extensions();

        Instance::new(None, &extensions, None)
            .unwrap()
    };

    App::new(
        instance.clone(),

        |builder| builder
            .with_title("Platformer")
            .with_dimensions(800, 600),

        |window| {
            let mut d = D {
                renderer: Renderer::new(instance, window),

                key_states: vec![ElementState::Released; VirtualKeyCode::Yen as usize],

                sprites: vec![
                    Sprite::new(WorldRect {
                        position: WorldCoords(600, 200),
                        bounds: WorldBounds(700, 600)
                    }),

                    Sprite::new(WorldRect {
                        position: WorldCoords(300, 800),
                        bounds: WorldBounds(200, 300)
                    }),
                ],

                terrain: TerrainMesh::new(vec![
                    TerrainVertex::Inner(WorldCoords(0, 0)),
                    TerrainVertex::Surface(WorldCoords(0, 100)),
                    TerrainVertex::Surface(WorldCoords(100, 200)),
                    TerrainVertex::Inner(WorldCoords(100, 0)),
                    TerrainVertex::Surface(WorldCoords(150, 450)),
                    TerrainVertex::Surface(WorldCoords(350, 150)),
                    TerrainVertex::Inner(WorldCoords(350, 0)),
                    TerrainVertex::Surface(WorldCoords(450, 550)),
                    TerrainVertex::Inner(WorldCoords(450, 0))
                ]),

                last_frame_time: Instant::now()
            };

            d.renderer.load_terrain(&d.terrain);
            d
        }
    )
        .unwrap()
        .run(30, State::Main());
}
