use crate::loader::*;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Relationships {
    Relationship: Vec<Relationship>,
}

#[derive(Deserialize, Debug)]
struct Relationship {
    Type: String,
    Target: String,
    Id: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "model")]
struct ThreeMFModel {
    resources: ThreeMFResource,
}

#[derive(Deserialize, Debug)]
struct ThreeMFResource {
    object: ThreeMFObject,
}

#[derive(Deserialize, Debug)]
struct ThreeMFObject {
    mesh: ThreeMFMesh,
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

pub struct ThreeMFLoader {}

impl Loader for ThreeMFLoader {
    fn load(&self, filepath: &str) -> Option<(Vec<Vertex>, Vec<IndexedTriangle>)> {
        let zipfile = std::fs::File::open(filepath).unwrap();

        let mut archive = zip::ZipArchive::new(zipfile).unwrap();

        let rel_file = match archive.by_name("_rels/.rels") {
            Ok(file) => file,
            Err(..) => {
                println!("File not found");
                return None;
            }
        };

        let rel: Relationships = serde_xml_rs::de::from_reader(rel_file).unwrap();

        let ModelPath = rel.Relationship[0].Target.clone();
        println!("Model Path: {}", ModelPath);

        let model_file = match archive.by_name(&ModelPath[1..]) {
            Ok(file) => file,
            Err(..) => {
                println!("File not found");
                return None;
            }
        };

        let model: ThreeMFModel = serde_xml_rs::de::from_reader(model_file).unwrap();

        let mut triangles = vec![];
        let vertices = model.resources.object.mesh.vertices.list;

        for triangle in model.resources.object.mesh.triangles.list {
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

        Some((vertices, triangles))
    }
}
