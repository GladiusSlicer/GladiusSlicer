use crate::error::SlicerErrors;
use crate::loader::{IndexedTriangle, Loader, Transform, Vertex};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Relationships {
    #[serde(rename = "Relationship")]
    relationship: Vec<Relationship>,
}

#[derive(Deserialize, Debug)]
struct Relationship {
    #[serde(rename = "Type")]
    relationship_type: String,
    #[serde(rename = "Target")]
    target: String,
    #[serde(rename = "Id")]
    id: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "model")]
struct ThreeMFModel {
    resources: ThreeMFResource,
    build: ThreeMFBuild,
}

#[derive(Deserialize, Debug)]
struct ThreeMFResource {
    object: Vec<ThreeMFObject>,
}
#[derive(Deserialize, Debug)]
struct ThreeMFBuild {
    item: Vec<ThreeMFItem>,
}

#[derive(Deserialize, Debug)]
struct ThreeMFObject {
    mesh: Option<ThreeMFMesh>,
    components: Option<ThreeMFComponents>,
    id: usize,
}

#[derive(Deserialize, Debug)]
struct ThreeMFItem {
    objectid: usize,
    transform: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ThreeMFComponents {
    component: Vec<ThreeMFComponent>,
}

#[derive(Deserialize, Debug)]
struct ThreeMFComponent {
    objectid: usize,
    transform: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "triangle")]
struct ThreeMFTriangle {
    v1: usize,
    v2: usize,
    v3: usize,
}

#[derive(Deserialize, Debug)]
struct ThreeMFMesh {
    vertices: ThreeMFVertices,
    triangles: ThreeMFTriangles,
}

#[derive(Deserialize, Debug)]
struct ThreeMFVertices {
    #[serde(rename = "vertex", default)]
    list: Vec<Vertex>,
}

#[derive(Deserialize, Debug)]
struct ThreeMFTriangles {
    #[serde(rename = "triangle", default)]
    list: Vec<ThreeMFTriangle>,
}

/// Loader for 3MF files
pub struct ThreeMFLoader {}

impl Loader for ThreeMFLoader {
    fn load(
        &self,
        filepath: &str,
    ) -> Result<Vec<(Vec<Vertex>, Vec<IndexedTriangle>)>, SlicerErrors> {
        let zipfile =
            std::fs::File::open(filepath).map_err(|_| SlicerErrors::ObjectFileNotFound {
                filepath: filepath.to_string(),
            })?;

        let mut archive =
            zip::ZipArchive::new(zipfile).map_err(|_| SlicerErrors::ThreemfUnsupportedType)?;

        let rel_file = match archive.by_name("_rels/.rels") {
            Ok(file) => file,
            Err(..) => {
                return Err(SlicerErrors::ThreemfLoadError);
            }
        };

        let rel: Relationships =
            serde_xml_rs::de::from_reader(rel_file).map_err(|_| SlicerErrors::ThreemfLoadError)?;

        let model_path = rel.relationship[0].target.clone();

        let model_file = match archive.by_name(&model_path[1..]) {
            Ok(file) => file,
            Err(..) => {
                return Err(SlicerErrors::ThreemfLoadError);
            }
        };

        let model: ThreeMFModel = serde_xml_rs::de::from_reader(model_file)
            .map_err(|_| SlicerErrors::ThreemfLoadError)?;

        model
            .build
            .item
            .iter()
            .map(|item| {
                let (mut v, t) = handle_object(item.objectid, &model.resources)?;

                if let Some(t_str) = &item.transform {
                    let transform = get_transform_from_string(t_str)?;

                    for vert in &mut v {
                        *vert = &transform * *vert;
                    }
                }
                Ok((v, t))
            })
            .collect()
    }
}

fn handle_object(
    obj_index: usize,
    comps: &ThreeMFResource,
) -> Result<(Vec<Vertex>, Vec<IndexedTriangle>), SlicerErrors> {
    let object = comps
        .object
        .iter()
        .find(|obj| obj.id == obj_index)
        .ok_or(SlicerErrors::ThreemfLoadError)?;

    if let Some(mesh) = &object.mesh {
        Ok(handle_mesh(mesh))
    } else if let Some(components) = &object.components {
        let mut v = vec![];
        let mut t = vec![];
        let mut start = 0;
        for component in &components.component {
            let (mut verts, mut triangles) = handle_object(component.objectid, comps)?;

            if let Some(t_str) = &component.transform {
                let transform = get_transform_from_string(t_str)?;

                for vert in &mut verts {
                    *vert = &transform * *vert;
                }
            }

            if start != 0 {
                for triangle in &mut triangles {
                    triangle.verts[0] += start;
                    triangle.verts[1] += start;
                    triangle.verts[2] += start;
                }
            }

            start += verts.len();

            v.append(&mut verts);
            t.append(&mut triangles);
        }

        Ok((v, t))
    } else {
        Err(SlicerErrors::ThreemfLoadError)
    }
}

fn handle_mesh(mesh: &ThreeMFMesh) -> (Vec<Vertex>, Vec<IndexedTriangle>) {
    let mut triangles = vec![];
    let vertices = mesh.vertices.list.clone();

    for triangle in &mesh.triangles.list {
        let mut converted_tri = IndexedTriangle {
            verts: [triangle.v1, triangle.v2, triangle.v3],
        };
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

    (vertices, triangles)
}

fn get_transform_from_string(transform_string: &str) -> Result<Transform, SlicerErrors> {
    let res_values: Result<Vec<f64>, _> =
        transform_string.split(' ').map(str::parse).collect();

    let values = res_values.map_err(|_| SlicerErrors::ThreemfLoadError)?;
    if values.len() != 12 {
        Err(SlicerErrors::ThreemfLoadError)
    } else {
        let t = [
            [values[0], values[3], values[6], values[9]],
            [values[1], values[4], values[7], values[10]],
            [values[2], values[5], values[8], values[11]],
            [0.0, 0.0, 0.0, 1.0],
        ];
        Ok(Transform(t))
    }
}
