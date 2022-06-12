pub mod model_loader;
mod obj_loader;
pub mod sound_loader;

pub use obj_loader::NormalVertex;

use std::collections::HashMap;

pub struct ResourcePool<T> (HashMap<String, T>);
impl<T: Clone> ResourcePool<T> {
    /// Returns a handle to a resource given its id if it exists
    pub fn get(&self, resource_id: &str) -> Option<&T> { self.0.get(resource_id) }

    /// Returns a copy of a resource given its id
    /// Might be more expensive
    pub fn get_copy(&self, resource_id: &str) -> Option<T> {
        self.0.get(resource_id).cloned()
    }

    /// Releases a resource given its id
    pub fn _release(&mut self, resource_id: &str) {
        self.0.remove(resource_id);
    }
}
impl<T> Default for ResourcePool<T> {
    fn default() -> Self {
        ResourcePool(HashMap::new())
    }
}