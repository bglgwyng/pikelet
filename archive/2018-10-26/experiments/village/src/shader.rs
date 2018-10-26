// Copyright The Voyager Developers 2014

#[shader_param(Batch)]
pub struct Params {
    #[name="u_SunDir"]
    pub sun_dir: [f32; 3],
    #[name = "u_Model"]
    pub model: [[f32; 4]; 4],
    #[name = "u_ViewProj"]
    pub view_proj: [[f32; 4]; 4],
}

pub mod color {
    use gfx;
    use std::fmt;

    #[derive(Copy)]
    #[vertex_format]
    pub struct Vertex {
        #[name = "a_Pos"]
        pub pos: [f32; 3],
        #[name = "a_Color"]
        pub color: [f32; 3],
    }

    impl Clone for Vertex {
        fn clone(&self) -> Vertex {
            Vertex { pos: self.pos, color: self.color }
        }
    }

    impl fmt::Show for Vertex {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{{ pos: {:?}, color: {:?} }}", self.pos, self.color)
        }
    }

    pub static VERTEX_SRC: gfx::ShaderSource<'static> = shaders! {
    glsl_120: b"
        #version 120

        attribute vec3 a_Pos;
        attribute vec3 a_Color;
        varying vec3 v_Color;

        uniform mat4 u_Model;
        uniform mat4 u_ViewProj;

        void main() {
            v_Color = a_Color;
            gl_Position = u_ViewProj * u_Model * vec4(a_Pos, 1.0);
        }
    ",
    glsl_150: b"
        #version 150 core

        in vec3 a_Pos;
        in vec3 a_Color;
        out vec3 v_Color;

        uniform mat4 u_Model;
        uniform mat4 u_ViewProj;

        void main() {
            v_Color = a_Color;
            gl_Position = u_ViewProj * u_Model * vec4(a_Pos, 1.0);
        }
    "
    };

    pub static FRAGMENT_SRC: gfx::ShaderSource<'static> = shaders! {
    glsl_120: b"
        #version 120

        varying vec3 v_Color;
        out vec4 o_Color;

        void main() {
            o_Color = vec4(v_Color, 1.0);
        }
    ",
    glsl_150: b"
        #version 150 core

        in vec3 v_Color;
        out vec4 o_Color;

        void main() {
            o_Color = vec4(v_Color, 1.0);
        }
    "
    };
}

pub mod flat {
    use gfx;
    use std::fmt;

    #[derive(Copy)]
    #[vertex_format]
    pub struct Vertex {
        #[name = "a_Pos"]
        pub pos: [f32; 3],
        #[name = "a_Norm"]
        pub norm: [f32; 3],
        #[name = "a_Color"]
        pub color: [f32; 3],
    }

    impl Clone for Vertex {
        fn clone(&self) -> Vertex {
            Vertex {
                pos: self.pos,
                norm: self.norm,
                color: self.color,
            }
        }
    }

    impl fmt::Show for Vertex {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{{ pos: {:?}, norm: {:?}, color: {:?} }}", self.pos, self.norm, self.color)
        }
    }

    pub static VERTEX_SRC: gfx::ShaderSource<'static> = shaders! {
    glsl_120: b"
        #version 120

        attribute vec3 a_Pos;
        attribute vec3 a_Color;
        attribute vec3 a_Norm;

        varying vec3 v_Pos;
        varying vec3 v_Color;
        varying vec3 v_Norm;

        uniform mat4 u_Model;
        uniform mat4 u_ViewProj;

        void main() {
            v_Pos = a_Pos;
            v_Color = a_Color;
            v_Norm = a_Norm;

            gl_Position = u_ViewProj * u_Model * vec4(a_Pos, 1.0);
        }
    ",
    glsl_150: b"
        #version 150 core

        in vec3 a_Pos;
        in vec3 a_Color;
        in vec3 a_Norm;

        out vec3 v_Pos;
        out vec3 v_Color;
        out vec3 v_Norm;

        uniform mat4 u_Model;
        uniform mat4 u_ViewProj;

        void main() {
            v_Pos = a_Pos;
            v_Color = a_Color;
            v_Norm = a_Norm;

            gl_Position = u_ViewProj * u_Model * vec4(a_Pos, 1.0);
        }
    "
    };

    pub static FRAGMENT_SRC: gfx::ShaderSource<'static> = shaders! {
    glsl_120: b"
        #version 120

        varying vec3 v_Pos;
        varying vec3 v_Color;
        varying vec3 v_Norm;

        out vec4 o_Color;

        uniform vec3 u_SunDir;

        void main() {
            float sunIntensity = max(dot(u_SunDir, normalize(v_Norm)), 0.0);
            o_Color = vec4(sunIntensity * v_Color, 1.0);
        }
    ",
    glsl_150: b"
        #version 150 core

        in vec3 v_Pos;
        in vec3 v_Color;
        in vec3 v_Norm;

        out vec4 o_Color;

        uniform vec3 u_SunDir;

        void main() {
            float sunIntensity = max(dot(u_SunDir, normalize(v_Norm)), 0.0);
            o_Color = vec4(sunIntensity * v_Color, 1.0);
        }
    "
    };
}