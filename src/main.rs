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


    frame: u32,
    start_time: Instant
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

        d.frame += 1;
        let time = Instant::now();
        let duration = time - d.start_time;
        println!("{} ({})", 1000000000 / (duration / d.frame).subsec_nanos(), (duration / d.frame).subsec_nanos());

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

        let diff = 1.0f64.to_radians() / 5.0;

        let mut coords = vec![TerrainVertex::Inner(WorldCoords(800, 700))];
        for deg in 0 .. 271 {
            let rad = (deg as u32 as f64 + (diff * d.frame as f64 % 5.0)).to_radians() * 4.0 / 3.0;
            coords.push(TerrainVertex::Surface(WorldCoords(800 + (600.0 * rad.sin()) as i32, 700 + (600.0 * rad.cos()) as i32)));
        }

        d.terrain = TerrainMesh::new(coords);
        d.renderer.load_terrain(&d.terrain);
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
            let mut coords = vec![TerrainVertex::Inner(WorldCoords(800, 700))];
            for deg in 0 .. 360 {
                let rad = (deg as f64).to_radians();
                coords.push(TerrainVertex::Surface(WorldCoords(800 + (600.0 * rad.sin()) as i32, 700 + (600.0 * rad.cos()) as i32)));
            }

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

                terrain: TerrainMesh::new(coords),

                frame: 0,
                start_time: Instant::now()
            };

            d.renderer.load_terrain(&d.terrain);
            d
        }
    )
        .unwrap()
        .run(60, State::Main());
}
