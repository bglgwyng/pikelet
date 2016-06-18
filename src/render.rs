use cgmath::conv::*;
use cgmath::prelude::*;
use cgmath::{Matrix4, Point2, Vector3};
use glium::{self, index, program, texture, vertex};
use glium::{DrawParameters, Frame, PolygonMode, Surface, BackfaceCullingMode};
use rusttype::Font;

use camera::ComputedCamera;
use color::Color;
use resources::{Resources, Buffer};
use text::TextData;

pub type RenderResult<T> = Result<T, RenderError>;

quick_error! {
    #[derive(Debug)]
    pub enum RenderError {
        Draw(error: glium::DrawError) {
            from()
            description(error.description())
            cause(error)
        }
        Index(error: index::BufferCreationError) {
            from()
            description(error.description())
            cause(error)
        }
        Program(error: program::ProgramChooserCreationError) {
            from()
            description(error.description())
            cause(error)
        }
        Texture(error: texture::TextureCreationError) {
            from()
            description(error.description())
            cause(error)
        }
        Vertex(error: vertex::BufferCreationError) {
            from()
            description(error.description())
            cause(error)
        }
    }
}

fn draw_params<'a>() -> DrawParameters<'a> {
    use glium::{BackfaceCullingMode, Depth, DepthTest};

    DrawParameters {
        backface_culling: BackfaceCullingMode::CullClockwise,
        depth: Depth {
            test: DepthTest::IfLess,
            write: true,
            ..Depth::default()
        },
        ..DrawParameters::default()
    }
}

pub struct RenderTarget<'a> {
    pub frame: &'a mut Frame,
    pub hidpi_factor: f32,
    pub resources: &'a Resources,
    pub camera: ComputedCamera,
    pub hud_matrix: Matrix4<f32>,
    pub culling_mode: BackfaceCullingMode,
}

impl<'a> RenderTarget<'a> {
    pub fn clear(&mut self, color: Color) {
        self.frame.clear_color_and_depth(color, 1.0);
    }

    pub fn render_hud_text(&mut self, font: &Font, text: &str, text_size: f32, position: Point2<f32>, color: Color) -> RenderResult<()> {
        use glium::texture::Texture2d;
        use glium::uniforms::MagnifySamplerFilter;

        let text_data = TextData::new(font, text, text_size * self.hidpi_factor);
        let text_texture = try!(Texture2d::new(&self.resources.context, &text_data));

        let params = {
            use glium::Blend;
            use glium::BlendingFunction::Addition;
            use glium::LinearBlendingFactor::*;

            let blending_function = Addition {
                source: SourceAlpha,
                destination: OneMinusSourceAlpha
            };

            DrawParameters {
                blend: Blend {
                    color: blending_function,
                    alpha: blending_function,
                    constant_value: (1.0, 1.0, 1.0, 1.0),
                },
                ..DrawParameters::default()
            }
        };

        try!(self.frame.draw(
            &self.resources.text_vertex_buffer,
            &self.resources.text_index_buffer,
            &self.resources.programs["text"],
            &uniform! {
                color:    color,
                text:     text_texture.sampled().magnify_filter(MagnifySamplerFilter::Nearest),
                proj:     array4x4(self.hud_matrix),
                model:    array4x4(text_data.matrix(position * self.hidpi_factor)),
            },
            &params,
        ));

        Ok(())
    }

    pub fn render_points(&mut self, &(ref vertex_buffer, ref index_buffer): &Buffer, point_size: f32, color: Color) -> RenderResult<()> {
        try!(self.frame.draw(
            vertex_buffer,
            index_buffer,
            &self.resources.programs["unshaded"],
            &uniform! {
                color:      color,
                model:      array4x4(Matrix4::from_scale(1.025f32)),
                view:       array4x4(self.camera.view),
                proj:       array4x4(self.camera.projection),
            },
            &DrawParameters {
                polygon_mode: PolygonMode::Point,
                point_size: Some(point_size),
                backface_culling: self.culling_mode,
                ..draw_params()
            },
        ));

        Ok(())
    }

    pub fn render_lines(&mut self, &(ref vertex_buffer, ref index_buffer): &Buffer, line_width: f32, color: Color) -> RenderResult<()> {
        try!(self.frame.draw(
            vertex_buffer,
            index_buffer,
            &self.resources.programs["unshaded"],
            &uniform! {
                color:      color,
                model:      array4x4(Matrix4::from_scale(1.025f32)),
                view:       array4x4(self.camera.view),
                proj:       array4x4(self.camera.projection),
            },
            &DrawParameters {
                polygon_mode: PolygonMode::Line,
                line_width: Some(line_width),
                backface_culling: self.culling_mode,
                ..draw_params()
            },
        ));

        Ok(())
    }

    pub fn render_solid(&mut self, &(ref vertex_buffer, ref index_buffer): &Buffer, light_dir: Vector3<f32>, color: Color) -> RenderResult<()> {
        try!(self.frame.draw(
            vertex_buffer,
            index_buffer,
            &self.resources.programs["flat_shaded"],
            &uniform! {
                color:      color,
                light_dir:  array3(light_dir),
                model:      array4x4(Matrix4::<f32>::identity()),
                view:       array4x4(self.camera.view),
                proj:       array4x4(self.camera.projection),
                eye:        array3(self.camera.position),
            },
            &DrawParameters {
                polygon_mode: PolygonMode::Fill,
                backface_culling: self.culling_mode,
                ..draw_params()
            },
        ));

        Ok(())
    }
}
