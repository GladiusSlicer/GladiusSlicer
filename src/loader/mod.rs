use crate::types::*;

mod stl;
mod threemf;

pub use stl::STLLoader;
pub use threemf::ThreeMFLoader;
use crate::SlicerErrors;

pub trait Loader {
    fn load(&self, filepath: &str) -> Result<(Vec<Vertex>, Vec<IndexedTriangle>),SlicerErrors>;
}
