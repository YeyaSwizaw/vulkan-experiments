#[macro_use] extern crate stateloop;
#[macro_use] extern crate vulkano;
#[macro_use] extern crate vulkano_shader_derive;
extern crate vulkano_win;

use std::time::{Duration, Instant};
use stateloop::app::{App, Data, Event, Window};
use stateloop::state::Action;

use vulkano::instance::Instance;

use renderer::Renderer;

mod shaders;
mod renderer;
mod ty;
mod sprite;

states! {
    State {
        MainHandler Main()
    }
}

pub struct D {
    renderer: Renderer,
    last_frame_time: Instant
}

impl MainHandler for Data<D> {
    fn handle_event(&mut self, event: Event) -> Action<State> {
        match event {
            Event::Closed => Action::Quit,
            _ => Action::Continue
        }
    }

    fn handle_tick(&mut self) {
        let next = Instant::now();
        println!("Frame Time: {:?}", next - self.data().last_frame_time);
        self.data_mut().last_frame_time = next;
    }

    fn handle_render(&self) {
        self.data().renderer.render();
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

        |window| D {
            renderer: Renderer::new(instance, window),
            last_frame_time: Instant::now()
        }
    )
        .unwrap()
        .run(30, State::Main());
}
