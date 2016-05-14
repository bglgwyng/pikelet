extern crate cgmath;
extern crate find_folder;
#[macro_use] extern crate glium;
#[macro_use] extern crate imgui;
extern crate num_traits;
#[macro_use] extern crate quick_error;
extern crate rand;
extern crate rayon;
extern crate rusttype;
extern crate time;

use cgmath::conv::*;
use cgmath::prelude::*;
use cgmath::{Matrix4, PerspectiveFov, Point2, Point3, Rad, Vector3};
use find_folder::Search as FolderSearch;
use glium::{DisplayBuild, Frame, IndexBuffer, Program, Surface, VertexBuffer, BackfaceCullingMode};
use glium::index::{PrimitiveType, NoIndices};
use imgui::Ui;
use rand::{Rand, Rng};
use rayon::prelude::*;
use std::mem;
use std::thread;
use std::time::Duration;

use camera::{Camera, ComputedCamera};
use input::Event;
use math::GeoPoint;
use render::{Resources, RenderTarget, Vertex};
use ui::Context as UiContext;

pub mod camera;
pub mod color;
pub mod geom;
pub mod input;
pub mod math;
pub mod text;
pub mod times;
pub mod render;
pub mod ui;

const WINDOW_TITLE: &'static str = "Geodesic Test";
const WINDOW_WIDTH: u32 = 1000;
const WINDOW_HEIGHT: u32 = 500;

const CAMERA_XZ_RADIUS: f32 = 2.0;
const CAMERA_Y_HEIGHT: f32 = 1.0;
const CAMERA_NEAR: f32 = 0.1;
const CAMERA_FAR: f32 = 1000.0;
const CAMERA_ZOOM_FACTOR: f32 = 10.0;
const CAMERA_DRAG_FACTOR: f32 = 10.0;

const POLYHEDRON_SUBDIVS: usize = 3;

const LIGHT_DIR: Vector3<f32> = Vector3 { x: 0.0, y: 1.0, z: 0.2 };

const STAR_FIELD_RADIUS: f32 = 20.0;

const STAR0_SIZE: f32 = 1.0;
const STAR1_SIZE: f32 = 2.5;
const STAR2_SIZE: f32 = 5.0;

const STARS0_COUNT: usize = 100000;
const STARS1_COUNT: usize = 10000;
const STARS2_COUNT: usize = 1000;

pub fn create_vertices(mesh: &geom::half_edge::Mesh) -> Vec<Vertex> {
    const VERTICES_PER_FACE: usize = 3;

    println!("--------------------");

    let mut vertices = Vec::with_capacity(mesh.faces.len() * VERTICES_PER_FACE);
    for (fi, face) in mesh.faces.iter().enumerate() {
        let e0 = face.root.clone();
        let e1 = mesh.edges[e0].next.clone();
        let e2 = mesh.edges[e1].next.clone();

        let p0 = mesh.edges[e0].position.clone();
        let p1 = mesh.edges[e1].position.clone();
        let p2 = mesh.edges[e2].position.clone();

        vertices.push( Vertex { position: mesh.positions[p0].into() } );
        vertices.push( Vertex { position: mesh.positions[p1].into() } );
        vertices.push( Vertex { position: mesh.positions[p2].into() } );

        println!("Face: {}", fi);
        println!("\tEdge indexes: {} -> {} -> {}", e0, e1, e2);
        println!("\tPosition indexes: {} -> {} -> {}", p0, p1, p2);
    }

    vertices
}

struct Star {
    pub position: GeoPoint<f32>,
}

impl Rand for Star {
    fn rand<R: Rng>(rng: &mut R) -> Self {
        Star { position: rng.gen() }
    }
}

struct StarField {
    stars0: Vec<Star>,
    stars1: Vec<Star>,
    stars2: Vec<Star>,
}

impl StarField {
    fn generate() -> StarField {
        let mut rng = rand::weak_rng();
        StarField {
            stars0: (0..STARS0_COUNT).map(|_| rng.gen()).collect(),
            stars1: (0..STARS1_COUNT).map(|_| rng.gen()).collect(),
            stars2: (0..STARS2_COUNT).map(|_| rng.gen()).collect(),
        }
    }
}

fn create_star_vertices(stars: &[Star]) -> Vec<Vertex> {
    let mut star_vertices = Vec::with_capacity(stars.len());
    stars.par_iter()
        .map(|star| Vertex { position: array3(star.position.to_point(STAR_FIELD_RADIUS)) })
        .collect_into(&mut star_vertices);

    star_vertices
}

#[derive(Copy, Clone, Debug)]
#[derive(PartialEq, Eq)]
enum Loop {
    Continue,
    Break,
}

struct State {
    delta_time: f32,
    frames_per_second: f32,

    is_wireframe: bool,
    is_showing_mesh: bool,
    is_showing_star_field: bool,
    is_showing_ui: bool,
    is_dragging: bool,
    is_ui_capturing_mouse: bool,
    is_zooming: bool,

    culling_mode: BackfaceCullingMode,

    mouse_position: Point2<i32>,
    window_dimensions: (u32, u32),

    light_dir: Vector3<f32>,

    camera_rotation: Rad<f32>,
    camera_rotation_delta: Rad<f32>,
    camera_distance: f32,

    subdivisions: usize,
}

impl State {
    fn init() -> State {
        State {
            delta_time: 0.0,
            frames_per_second: 0.0,

            is_wireframe: false,
            is_showing_mesh: true,
            is_showing_star_field: true,
            is_showing_ui: true,
            is_dragging: false,
            is_ui_capturing_mouse: false,
            is_zooming: false,

            culling_mode: BackfaceCullingMode::CullClockwise,

            light_dir: LIGHT_DIR,

            mouse_position: Point2::origin(),
            window_dimensions: (WINDOW_WIDTH, WINDOW_HEIGHT),

            camera_rotation: Rad::new(0.0),
            camera_rotation_delta: Rad::new(0.0),
            camera_distance: CAMERA_XZ_RADIUS,

            subdivisions: POLYHEDRON_SUBDIVS,
        }
    }

    fn apply_mouse_update(&mut self, new_position: Point2<i32>) {
        let mouse_position_delta = {
            let old_position = mem::replace(&mut self.mouse_position, new_position);
            new_position - old_position
        };

        if !self.is_ui_capturing_mouse {
            if self.is_dragging {
                let (window_width, _) = self.window_dimensions;
                let rotations_per_second = (mouse_position_delta.x as f32 / window_width as f32) * CAMERA_DRAG_FACTOR;
                self.camera_rotation_delta = Rad::full_turn() * rotations_per_second * self.delta_time;
            }

            if self.is_zooming {
                let zoom_delta = mouse_position_delta.x as f32 * self.delta_time;
                self.camera_distance = self.camera_distance - (zoom_delta * CAMERA_ZOOM_FACTOR);
            }
        }
    }

    fn apply_event_update(&mut self, event: Event) -> Loop {
        use input::Event::*;

        match event {
            CloseApp => return Loop::Break,
            SetShowingMesh(value) => self.is_showing_mesh = value,
            SetShowingStarField(value) => self.is_showing_star_field = value,
            SetUiCapturingMouse(value) => self.is_ui_capturing_mouse = value,
            SetWireframe(value) => self.is_wireframe = value,
            ToggleUi => self.is_showing_ui = !self.is_showing_ui,
            ResetState => *self = State::init(),
            DragStart => if !self.is_ui_capturing_mouse { self.is_dragging = true },
            DragEnd => self.is_dragging = false,
            ZoomStart => self.is_zooming = true,
            ZoomEnd => self.is_zooming = false,
            MousePosition(position) => self.apply_mouse_update(position),
            UpdateSubdivisions(subdivisions) => self.subdivisions = subdivisions,
            NoOp => {},
        }

        Loop::Continue
    }

    fn update<Events>(&mut self, events: Events, window_dimensions: (u32, u32), delta_time: f32) -> Loop where
        Events: IntoIterator<Item = Event>,
    {
        self.delta_time = delta_time;
        self.window_dimensions = window_dimensions;
        self.frames_per_second = 1.0 / delta_time;

        if self.is_dragging {
            self.camera_rotation_delta = Rad::new(0.0);
        }

        for event in events {
            if self.apply_event_update(event) == Loop::Break {
                return Loop::Break;
            }
        }

        self.camera_rotation = self.camera_rotation - self.camera_rotation_delta;

        Loop::Continue
    }

    fn create_scene_camera(&self, (frame_width, frame_height): (u32, u32)) -> ComputedCamera {
        Camera {
            position: Point3 {
                x: Rad::sin(self.camera_rotation) * self.camera_distance,
                y: CAMERA_Y_HEIGHT,
                z: Rad::cos(self.camera_rotation) * self.camera_distance,
            },
            target: Point3::origin(),
            projection: PerspectiveFov {
                aspect: frame_width as f32 / frame_height as f32,
                fovy: Rad::full_turn() / 6.0,
                near: CAMERA_NEAR,
                far: CAMERA_FAR,
            },
        }.compute()
    }

    fn create_hud_camera(&self, (frame_width, frame_height): (u32, u32)) -> Matrix4<f32> {
        cgmath::ortho(0.0, frame_width as f32, frame_height as f32, 0.0, -1.0, 1.0)
    }
}

fn render_scene(frame: &mut Frame, state: &State, resources: &Resources, hidpi_factor: f32) {
    let frame_dimensions = frame.get_dimensions();

    let mut target = RenderTarget {
        frame: frame,
        hidpi_factor: hidpi_factor,
        resources: resources,
        camera: state.create_scene_camera(frame_dimensions),
        hud_matrix: state.create_hud_camera(frame_dimensions),
        culling_mode: state.culling_mode,
    };

    target.clear(color::BLUE);

    if state.is_showing_star_field {
        // TODO: Render centered at eye position
        target.render_points(&resources.stars0_vertex_buffer, STAR0_SIZE, color::WHITE).unwrap();
        target.render_points(&resources.stars1_vertex_buffer, STAR1_SIZE, color::WHITE).unwrap();
        target.render_points(&resources.stars2_vertex_buffer, STAR2_SIZE, color::WHITE).unwrap();
    }

    if state.is_showing_mesh {
        target.render_points(&resources.delaunay_vertex_buffer, 5.0, color::RED).unwrap();
        target.render_points(&resources.voronoi_vertex_buffer, 5.0, color::YELLOW).unwrap();
        target.render_lines(&resources.voronoi_vertex_buffer, 0.5, color::WHITE).unwrap();
    }

    if state.is_wireframe {
        target.render_lines(&resources.delaunay_vertex_buffer, 0.5, color::BLACK).unwrap();
    } else {
        target.render_solid(&resources.delaunay_vertex_buffer, state.light_dir, color::GREEN).unwrap();
    }

    // FIXME: https://github.com/Gekkio/imgui-rs/issues/17
    // target.render_hud_text(&state.frames_per_second.to_string(), 12.0, Point2::new(2.0, 2.0), color::BLACK).unwrap();
}

fn build_ui<'a>(ui_context: &'a mut UiContext, state: &State) -> (Option<Ui<'a>>, Vec<Event>) {
    use input::Event::*;

    fn checkbox(ui: &Ui, text: imgui::ImStr, initial_value: bool) -> Option<bool> {
        let mut value = initial_value;
        ui.checkbox(text, &mut value);

        if value != initial_value { Some(value) } else { None }
    }

    fn slider_i32(ui: &Ui, text: imgui::ImStr, initial_value: i32, min: i32, max: i32) -> Option<i32> {
        let mut value = initial_value;
        ui.slider_i32(text, &mut value, min, max).build();

        if value != initial_value { Some(value) } else { None }
    }

    if !state.is_showing_ui {
        return (None, vec![]);
    }

    let ui = ui_context.frame(state.window_dimensions, state.delta_time);
    let mut events = vec![];

    ui.window(im_str!("State"))
        .position((10.0, 10.0), imgui::ImGuiSetCond_FirstUseEver)
        .size((250.0, 350.0), imgui::ImGuiSetCond_FirstUseEver)
        .build(|| {
            ui.tree_node(im_str!("Render options")).build(|| {
                checkbox(&ui, im_str!("Wireframe"), state.is_wireframe)
                    .map(|v| events.push(SetWireframe(v)));
                checkbox(&ui, im_str!("Show mesh"), state.is_showing_mesh)
                    .map(|v| events.push(SetShowingMesh(v)));
                checkbox(&ui, im_str!("Show starfield"), state.is_showing_star_field)
                    .map(|v| events.push(SetShowingStarField(v)));
            });

            ui.tree_node(im_str!("State")).build(|| {
                ui.text(im_str!("delta_time: {:?}", state.delta_time));
                ui.text(im_str!("frames_per_second: {:?}", state.frames_per_second));

                ui.separator();

                ui.text(im_str!("is_wireframe: {:?}", state.is_wireframe));
                ui.text(im_str!("is_showing_mesh: {:?}", state.is_showing_mesh));
                ui.text(im_str!("is_showing_star_field: {:?}", state.is_showing_star_field));
                ui.text(im_str!("is_showing_ui: {:?}", state.is_showing_ui));
                ui.text(im_str!("is_dragging: {:?}", state.is_dragging));
                ui.text(im_str!("is_ui_capturing_mouse: {:?}", state.is_ui_capturing_mouse));
                ui.text(im_str!("is_zooming: {:?}", state.is_zooming));

                ui.text(im_str!("culling_mode: {:?}", state.culling_mode));

                ui.separator();

                ui.text(im_str!("light_dir: {:?}", state.light_dir));

                ui.separator();

                ui.text(im_str!("mouse_position: {:?}", state.mouse_position));
                ui.text(im_str!("window_dimensions: {:?}", state.window_dimensions));

                ui.separator();

                ui.text(im_str!("camera_rotation: {:?}", state.camera_rotation));
                ui.text(im_str!("camera_rotation_delta: {:?}", state.camera_rotation_delta));
                ui.text(im_str!("camera_distance: {:?}", state.camera_distance));

                ui.separator();

                ui.text(im_str!("subdivisions: {:?}", state.subdivisions));
            });

            slider_i32(&ui, im_str!("subdivisions"), state.subdivisions as i32, 1, 8)
                .map(|v| events.push(UpdateSubdivisions(v as usize)));

            if ui.small_button(im_str!("Reset state")) {
                events.push(ResetState);
            }
        });

    if ui.want_capture_mouse() != state.is_ui_capturing_mouse {
        events.push(SetUiCapturingMouse(ui.want_capture_mouse()));
    }

    (Some(ui), events)
}

fn main() {
    use glium::backend::Facade;
    use glium::glutin::WindowBuilder;

    let display = WindowBuilder::new()
        .with_title(WINDOW_TITLE.to_string())
        .with_dimensions(WINDOW_WIDTH, WINDOW_HEIGHT)
        .with_depth_buffer(24)
        .build_glium()
        .unwrap();

    let mut state = State::init();

    let resources = {
        use rusttype::FontCollection;
        use std::fs::File;
        use std::io;
        use std::io::prelude::*;
        use std::path::Path;

        let radius = 1.0;
        let geometry = geom::primitives::icosahedron(radius);
        let subdivided = geometry.subdivide(POLYHEDRON_SUBDIVS, &|a, b| math::midpoint_arc(radius, a, b));
        let star_field = StarField::generate();

        let assets = FolderSearch::ParentsThenKids(3, 3)
                .for_folder("resources")
                .expect("Could not locate `resources` folder");

        let load_shader = |assets: &Path, path| -> io::Result<String> {
            let mut file = try!(File::open(assets.join(path)));
            let mut buffer = String::new();
            try!(file.read_to_string(&mut buffer));

            Ok(buffer)
        };

        let flat_shaded_vert    = load_shader(&assets, "shaders/flat_shaded.v.glsl").unwrap();
        let flat_shaded_frag    = load_shader(&assets, "shaders/flat_shaded.f.glsl").unwrap();
        let text_vert           = load_shader(&assets, "shaders/text.v.glsl").unwrap();
        let text_frag           = load_shader(&assets, "shaders/text.f.glsl").unwrap();
        let unshaded_vert       = load_shader(&assets, "shaders/unshaded.v.glsl").unwrap();
        let unshaded_frag       = load_shader(&assets, "shaders/unshaded.f.glsl").unwrap();

        let flat_shaded_program = Program::from_source(&display, &flat_shaded_vert, &flat_shaded_frag, None).unwrap();
        let text_program        = Program::from_source(&display, &text_vert, &text_frag, None).unwrap();
        let unshaded_program    = Program::from_source(&display, &unshaded_vert, &unshaded_frag, None).unwrap();

        let blogger_sans_font = {
            let mut file = File::open(assets.join("fonts/blogger/Blogger Sans.ttf")).unwrap();
            let mut buffer = vec![];
            file.read_to_end(&mut buffer).unwrap();

            let font_collection = FontCollection::from_bytes(buffer);
            font_collection.into_font().unwrap()
        };

        Resources {
            context: display.get_context().clone(),

            delaunay_vertex_buffer: VertexBuffer::new(&display, &create_vertices(&subdivided)).unwrap(),
            voronoi_vertex_buffer: VertexBuffer::new(&display, &create_vertices(&geometry)).unwrap(),
            index_buffer: NoIndices(PrimitiveType::TrianglesList),

            text_vertex_buffer: VertexBuffer::new(&display, &text::TEXTURE_VERTICES).unwrap(),
            text_index_buffer: IndexBuffer::new(&display, PrimitiveType::TrianglesList, &text::TEXTURE_INDICES).unwrap(),

            stars0_vertex_buffer: VertexBuffer::new(&display, &create_star_vertices(&star_field.stars0)).unwrap(),
            stars1_vertex_buffer: VertexBuffer::new(&display, &create_star_vertices(&star_field.stars1)).unwrap(),
            stars2_vertex_buffer: VertexBuffer::new(&display, &create_star_vertices(&star_field.stars2)).unwrap(),

            flat_shaded_program: flat_shaded_program,
            text_program: text_program,
            unshaded_program: unshaded_program,

            blogger_sans_font: blogger_sans_font,
        }
    };

    let mut ui_context = UiContext::new();
    let mut ui_renderer = ui_context.init_renderer(&display).unwrap();

    for time in times::in_seconds() {
        // FIXME: lots of confusing mutations if the event buffer...

        let display_events: Vec<_> = display.poll_events().collect();

        let window = display.get_window().unwrap();
        let window_dimensions = window.get_inner_size_points().unwrap();
        let hidpi_factor = window.hidpi_factor();
        let delta_time = time.delta() as f32;

        ui_context.update(display_events.iter(), hidpi_factor);
        let (ui, ui_events) = build_ui(&mut ui_context, &mut state);

        let events = display_events.into_iter()
            .map(Event::from)
            .chain(ui_events);

        match state.update(events, window_dimensions, delta_time) {
            Loop::Break => break,
            Loop::Continue => {
                let mut frame = display.draw();

                render_scene(&mut frame, &state, &resources, hidpi_factor);

                if let Some(ui) = ui {
                    ui_renderer.render(&mut frame, ui, hidpi_factor).unwrap();
                }

                frame.finish().unwrap()
            }
        }

        thread::sleep(Duration::from_millis(10)); // battery saver ;)
    }
}
