#![deny(missing_docs)]

use crate::error::SlicerErrors;
use crate::types::{IndexedTriangle, Transform, Vertex};

mod stl;
mod threemf;

pub use stl::STLLoader;
pub use threemf::ThreeMFLoader;

/// Loader trait to define loading in a file type of a model into a triangles and vertices
pub trait Loader {
    /// Load a specific file
    fn load(
        &self,
        filepath: &str,
    ) -> Result<Vec<(Vec<Vertex>, Vec<IndexedTriangle>)>, SlicerErrors>;
}
