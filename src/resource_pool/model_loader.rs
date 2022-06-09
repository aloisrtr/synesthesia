use std::cell::RefCell;
use std::sync::Arc;
use glm::{identity, inverse_transpose, rotate_normalized_axis, TMat4, translate, TVec3, vec3};
use crate::{Render};
use super::obj_loader::{ Loader, NormalVertex };
use crate::resource_pool::ResourcePool;

vulkano::impl_vertex!(NormalVertex, position, normal, color);

impl ResourcePool<Model> {
    pub fn load(&mut self, resource_id: &str, file_path: &str) -> Result<(), String> {
        let mut model = Model::new(file_path).build();
        self.0.insert(String::from(resource_id), model);
        Ok(())
    }
}

/// This has been taken from taidaesal's Vulkano tutorial
///
/// Holds our data for a renderable model, including the model matrix data
///
/// Note: When building an instance of `Model` the loader will assume that
/// the input obj file is in clockwise winding order. If it is already in
/// counter-clockwise winding order, call `.invert_winding_order(false)`
/// when building the `Model`.
pub struct Model {
    data: Vec<NormalVertex>,
    translation: TMat4<f32>,
    rotation: TMat4<f32>,
    model: TMat4<f32>,
    normals: TMat4<f32>,
    // we might call multiple translation/rotation calls
    // in between asking for the model matrix. This lets
    // only recreate the model matrix when needed.
    requires_update: bool,
}

pub struct ModelBuilder {
    file_name: String,
    custom_color: [f32; 3],
    invert: bool,
}
impl ModelBuilder {
    fn new(file: String) -> ModelBuilder {
        ModelBuilder {
            file_name: file,
            custom_color: [1.0, 0.35, 0.137],
            invert: true,
        }
    }

    pub fn build(self) -> Model {
        let loader = Loader::new(self.file_name.as_str(), self.custom_color, self.invert);
        Model {
            data: loader.as_normal_vertices(),
            translation: identity(),
            rotation: identity(),
            model: identity(),
            normals: identity(),
            requires_update: false,
        }
    }

    pub fn color(mut self, new_color: [f32; 3]) -> ModelBuilder {
        self.custom_color = new_color;
        self
    }

    pub fn file(mut self, file: String) -> ModelBuilder {
        self.file_name = file;
        self
    }

    pub fn invert_winding_order(mut self, invert: bool) -> ModelBuilder {
        self.invert = invert;
        self
    }
}

impl Model {
    pub fn new(file_name: &str) -> ModelBuilder {
        ModelBuilder::new(file_name.into())
    }

    pub fn data(&self) -> Vec<NormalVertex> {
        self.data.clone()
    }

    pub fn model_matrices(&self) -> (TMat4<f32>, TMat4<f32>) {
        (self.model, self.normals)
    }

    pub fn rotate(&mut self, radians: f32, v: TVec3<f32>) -> &mut Self {
        self.rotation = rotate_normalized_axis(&self.rotation, radians, &v);
        self.model = self.translation * self.rotation;
        self.normals = inverse_transpose(self.model);

        self
    }

    pub fn translate(&mut self, v: TVec3<f32>) -> &mut Self {
        self.translation = translate(&self.translation, &v);
        self.model = self.translation * self.rotation;
        self.normals = inverse_transpose(self.model);

        self
    }

    /// Return the model's rotation to 0
    pub fn zero_rotation(&mut self) -> &mut Self {
        self.rotation = identity();
        self.model = self.translation * self.rotation;
        self.normals = inverse_transpose(self.model);

        self
    }
}
impl Render<NormalVertex> for Model {
    fn vertices(&self) -> Vec<NormalVertex> {
        self.data()
    }

    fn model_matrices(&self) -> (TMat4<f32>, TMat4<f32>) {
        self.model_matrices()
    }
}