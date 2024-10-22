use crate::loader::{IndexedTriangle, Loader, SlicerErrors, Vertex};
use std::io::BufReader;

/// Loader for STL files
pub struct STLLoader {}

impl Loader for STLLoader {
    fn load(
        &self,
        filepath: &str,
    ) -> Result<Vec<(Vec<Vertex>, Vec<IndexedTriangle>)>, SlicerErrors> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .open(filepath)
            .map_err(|_| SlicerErrors::ObjectFileNotFound {
                filepath: filepath.to_string(),
            })?;

        let mut root_vase = BufReader::new(&file);
        let mesh: nom_stl::IndexMesh = nom_stl::parse_stl(&mut root_vase)
            .map_err(|_| SlicerErrors::StlLoadError)?
            .into();

        let mut triangles = vec![];
        let vertices = mesh
            .vertices()
            .iter()
            .map(|vert| Vertex {
                x: vert[0] as f64,
                y: vert[1] as f64,
                z: vert[2] as f64,
            })
            .collect::<Vec<Vertex>>();

        for triangle in mesh.triangles() {
            let mut converted_tri = IndexedTriangle {
                verts: [
                    triangle.vertices_indices()[0],
                    triangle.vertices_indices()[1],
                    triangle.vertices_indices()[2],
                ],
            };
            /*
                        let A = v1.x * v0.y + v2.x * v1.y + v0.x * v2.y;
                        let B = v0.x * v1.y + v1.x * v2.y + v2.x * v0.y;

                        if  A < B
                        {
                            let temp = converted_tri.verts[0];
                            converted_tri.verts[0] = converted_tri.verts[1];
                            converted_tri.verts[1] = temp;
                            std::mem::swap(&mut v0, &mut v1);
                        }
            */
            let v0 = vertices[converted_tri.verts[0]];
            let v1 = vertices[converted_tri.verts[1]];
            let v2 = vertices[converted_tri.verts[2]];

            if v0 < v1 && v0 < v2 {
                triangles.push(converted_tri);
            } else if v1 < v2 && v1 < v0 {
                let temp = converted_tri.verts[0];
                converted_tri.verts[0] = converted_tri.verts[1];
                converted_tri.verts[1] = converted_tri.verts[2];
                converted_tri.verts[2] = temp;
                triangles.push(converted_tri);
            } else {
                let temp = converted_tri.verts[0];
                converted_tri.verts[0] = converted_tri.verts[2];
                converted_tri.verts[2] = converted_tri.verts[1];
                converted_tri.verts[1] = temp;
                triangles.push(converted_tri);
            }
        }

        Ok(vec![(vertices, triangles)])
    }
}
