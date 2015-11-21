use std::sync;
use std::sync::mpsc;
use std::thread;
use std::time;

use glium;
use glium::DisplayBuild;
use glium::Surface;

use error;
use world;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}

implement_vertex!(Vertex, position);

impl Vertex {
    pub fn new(x: f32, y: f32) -> Vertex {
        Vertex { position: [x, y] }
    }
}

pub fn draw_loop(rate: u64,
                 incoming: mpsc::Receiver<sync::Arc<sync::Mutex<world::World>>>,
                 outgoing: mpsc::Sender<sync::Arc<sync::Mutex<world::World>>>) {
    let monitor = glium::glutin::get_primary_monitor();
    let (w, h) = monitor.get_dimensions();

    let display = glium::glutin::WindowBuilder::new()
                      .with_dimensions(w, h)
                      .with_title("Life".to_owned())
                      .build_glium()
                      .expect("Could not build glium window!");

    let vertex_shader_source = r#"
#version 140

in vec2 position;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
}
"#;

    let fragment_shader_source = r#"
#version 140

out vec4 color;

void main() {
    color = vec4(0.0, 0.0, 0.0, 1.0);
}
"#;

    let program = glium::Program::from_source(&display,
                                              vertex_shader_source,
                                              fragment_shader_source,
                                              None)
                      .expect("Could not create program!");

    let mut triangles = Vec::with_capacity(10000);

    for world in incoming {
        unsafe {
            triangles.set_len(0);
        }

        let mut target = display.draw();
        target.clear_color(1.0, 1.0, 1.0, 1.0);
        {
            let w = world.lock().unwrap();
            draw_world(&display, &program, &mut target, &*w, &mut triangles)
                .expect("Could not draw world!");
        }

        if let Err(_) = outgoing.send(world) {
            return;
        }

        target.finish().expect("Could not finish drawing!");

        for ev in display.poll_events() {
            match ev {
                glium::glutin::Event::Closed |
                glium::glutin::Event::KeyboardInput(_,
                                                    _,
                                                    Some(glium::glutin::VirtualKeyCode::Escape)) =>
                    return,

                _ => {}
            }
        }

        if rate > 0 {
            thread::sleep(time::Duration::from_millis(rate));
        }
    }
}

fn draw_world<F, S>(display: &F,
                    program: &glium::Program,
                    target: &mut S,
                    world: &world::World,
                    triangles: &mut Vec<Vertex>)
                    -> Result<(), error::Error>
    where F: glium::backend::Facade,
          S: glium::Surface
{
    let cell_height = 2f32 / world.height() as f32;
    let cell_width = 2f32 / world.width() as f32;

    for (i, row) in &world.rows() {
        for (j, cell) in row.iter().enumerate() {
            if !cell {
                continue;
            }

            let v1 = Vertex::new((j as f32 * cell_width) - 1f32,
                                 (i as f32 * cell_height) - 1f32);
            let v2 = Vertex::new(((j as f32 + 1f32) * cell_width) - 1f32,
                                 (i as f32 * cell_height) - 1f32);
            let v3 = Vertex::new(((j as f32 + 1f32) * cell_width) - 1f32,
                                 ((i as f32 + 1f32) * cell_height) - 1f32);
            let v4 = Vertex::new((j as f32 * cell_width) - 1f32,
                                 ((i as f32 + 1f32) * cell_height) - 1f32);

            triangles.push(v1);
            triangles.push(v2);
            triangles.push(v3);

            triangles.push(v1);
            triangles.push(v3);
            triangles.push(v4);
        }
    }

    let vertex_buffer = try!(glium::VertexBuffer::new(display, triangles));
    let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

    try!(target.draw(&vertex_buffer,
                     &indices,
                     program,
                     &glium::uniforms::EmptyUniforms,
                     &Default::default()));

    Ok(())
}
