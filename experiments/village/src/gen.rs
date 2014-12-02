// Copyright The Voyager Developers 2014

use nalgebra::*;
use noise::source::Source;
use std::rand::Rng;

use World;
use camera::Camera;
use terrain::Terrain;

pub struct Range {
    pub min: f32,
    pub max: f32,
}

impl Range {
    pub fn delta(&self) -> f32 {
        self.max - self.min
    }

    /// Shift a range factor into the range. It is assumed that `factor` is a
    /// number in the range `[0.0, 1.0]`.
    pub fn shift(&self, factor: f32) -> f32 {
        (factor * self.delta()) + self.min
    }
}

/// Construct a model matrix
fn model_mat(scale: Vec3<f32>, position: Pnt3<f32>) -> Mat4<f32> {
    let mut model: Mat4<f32> = zero();
    model.set_col(0, Vec4::x() * scale.x);
    model.set_col(1, Vec4::y() * scale.y);
    model.set_col(2, Vec4::z() * scale.z);
    model.set_col(3, position.to_homogeneous().to_vec());
    model
}

pub enum ScaleRange {
    Proportional {
        xyz: Range,
    },
    NonProportional {
        x: Range,
        y: Range,
        z: Range,
    },
}

pub struct Scatter {
    pub scale: ScaleRange,
    pub pos_x: Range,
    pub pos_y: Range,
}

impl Scatter {
    pub fn new() -> Scatter {
        Scatter {
            scale: ScaleRange::Proportional { xyz: Range { min: 0.0, max: 1.0 } },
            pos_x: Range { min: 0.0, max: 1.0 },
            pos_y: Range { min: 0.0, max: 1.0 },
        }
    }

    pub fn scale_proportional(self, xyz: Range) -> Scatter {
        Scatter { scale: ScaleRange::Proportional { xyz: xyz }, ..self }
    }

    pub fn scale_non_proportional(self, x: Range, y: Range, z: Range) -> Scatter {
        Scatter { scale: ScaleRange::NonProportional { x: x, y: y, z: z }, ..self }
    }

    pub fn pos_x(self, pos_x:  Range) -> Scatter {
        Scatter { pos_x: pos_x, ..self }
    }

    pub fn pos_y(self, pos_y:  Range) -> Scatter {
        Scatter { pos_y: pos_y, ..self }
    }

    pub fn gen_scale<R: Rng>(&self, rng: &mut R) -> Vec3<f32> {
        match self.scale {
            ScaleRange::Proportional { xyz } => {
                let xyz = xyz.shift(rng.gen());
                Vec3::new(xyz, xyz, xyz)
            },
            ScaleRange::NonProportional { x, y, z } => {
                Vec3::new(x.shift(rng.gen()),
                          y.shift(rng.gen()),
                          z.shift(rng.gen()))
            },
        }
    }

    pub fn gen_position<S: Source, R: Rng>(&self, terrain: &Terrain<S>, rng: &mut R) -> Pnt3<f32> {
        let x = self.pos_x.shift(rng.gen());
        let y = self.pos_y.shift(rng.gen());
        let z = terrain.get_height_at(x, y);
        Pnt3::new(x, y, z)
    }

    pub fn scatter_objects<S: Source, R: Rng>(&self, count: uint, terrain: &Terrain<S>, rng: &mut R) -> Objects {
        Objects {
            transforms: {
                range(0, count)
                    .map(|_| model_mat(self.gen_scale(rng), self.gen_position(terrain, rng)))
                    .collect()
            },
        }
    }

    pub fn scatter_billboards<S: Source, R: Rng>(self, count: uint, terrain: &Terrain<S>, rng: &mut R) -> Billboards {
        Billboards {
            scales: range(0, count).map(|_| self.gen_scale(rng)).collect(),
            positions: range(0, count).map(|_| self.gen_position(terrain, rng)).collect(),
        }
    }
}

pub struct Objects {
    pub transforms: Vec<Mat4<f32>>,
}

impl Objects {
    pub fn map_worlds(&self, sun_dir: Vec3<f32>, view_proj: Mat4<f32>, f: |&World|) {
        let mut world = World {
            sun_dir: sun_dir,
            model: one(),
            view_proj: view_proj,
        };

        for model in self.transforms.iter() {
            world.model = *model;
            f(&world)
        }
    }
}

pub struct Billboards {
    pub scales: Vec<Vec3<f32>>,
    pub positions: Vec<Pnt3<f32>>,
}

impl Billboards {
    pub fn map_worlds(&self, sun_dir: Vec3<f32>, cam: Camera<f32>, f: |&World|) {
        let mut world = World {
            sun_dir: sun_dir,
            model: one(),
            view_proj: cam.to_mat(),
        };

        for (scale, position) in self.scales.iter().zip(self.positions.iter()) {
            let Vec3 { x, y, .. } = cam.view.translation;
            let scale_mat = model_mat(*scale, Pnt3::new(0.0, 0.0, 0.0));
            let mut tform: Iso3<f32> = one();
            tform.look_at_z(position, &Pnt3::new(x, y, position.z), &Vec3::z());
            world.model = tform.to_homogeneous() * scale_mat;
            f(&world)
        }
    }
}