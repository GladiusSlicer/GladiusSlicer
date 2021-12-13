use crate::types::*;

mod threemf;
mod stl;

pub use stl::STLLoader;
pub use threemf::ThreeMFLoader
;
pub trait Loader {
    fn load(&self, filepath: &str) -> Option<(Vec<Vertex>, Vec<IndexedTriangle>)>;
}
