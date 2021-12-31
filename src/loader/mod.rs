use crate::types::*;

mod stl;
mod threemf;

use crate::SlicerErrors;
pub use stl::STLLoader;
pub use threemf::ThreeMFLoader;

pub trait Loader {
    fn load(&self, filepath: &str) -> Result<Vec<(Vec<Vertex>, Vec<IndexedTriangle>)>, SlicerErrors>;
}
