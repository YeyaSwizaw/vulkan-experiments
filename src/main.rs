#[macro_use] extern crate stateloop;
#[macro_use] extern crate vulkano;
#[macro_use] extern crate vulkano_shader_derive;
extern crate vulkano_win;

use stateloop::app::{App, Data, Event, Window};
use stateloop::state::Action;

use vulkano::instance::Instance;

use renderer::Renderer;

mod shaders;
mod renderer;

states! {
    State {
        MainHandler Main()
    }
}

impl MainHandler for Data<Renderer> {
    fn handle_event(&mut self, event: Event) -> Action<State> {
        match event {
            Event::Closed => Action::Quit,
            _ => Action::Continue
        }
    }

    fn handle_tick(&mut self) {}

    fn handle_render(&self) {
        self.data().render();
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

        |window| Renderer::new(instance, window)
    )
        .unwrap()
        .run(30, State::Main());
}
