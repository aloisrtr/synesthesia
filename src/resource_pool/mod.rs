pub mod model_loader;
mod obj_loader;

use std::cell::RefCell;
pub use obj_loader::NormalVertex;

use std::collections::HashMap;

pub struct ResourcePool<T> (HashMap<String, T>);
impl<T> ResourcePool<T> {
    /// Returns a mutable handle given a resource id, or None if
    /// no such resource is present in the pool
    pub fn get_mut(&mut self, resource_id: &str) -> Option<&mut T> {
        self.0.get_mut(resource_id)
    }

    pub fn get(&self, resource_id: &str) -> Option<&T> { self.0.get(resource_id) }

    /// Releases a resource given its id
    pub fn release(&mut self, resource_id: &str) {
        self.0.remove(resource_id);
    }
}
impl<T> Default for ResourcePool<T> {
    fn default() -> Self {
        ResourcePool(HashMap::new())
    }
}