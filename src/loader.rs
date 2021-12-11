use crate::types::*;
use std::io::prelude::*;
use std::io::BufReader;
use serde::{Serialize, Deserialize};

pub trait Loader{
    fn load(&self, filepath: &str) -> Option< (Vec<Vertex>, Vec<IndexedTriangle>)>;

}

pub struct STLLoader {}

impl Loader for STLLoader {
    fn load(&self , filepath: &str) -> Option< (Vec<Vertex>, Vec<IndexedTriangle>)>{

        let file = std::fs::OpenOptions::new().read(true).open(filepath).unwrap();

        let mut root_vase = BufReader::new(&file);
        let mesh: nom_stl::IndexMesh = nom_stl::parse_stl(&mut root_vase).unwrap().into();

        let mut triangles = vec![];
        let vertices = mesh.vertices().iter().map(|vert|Vertex{x: vert[0] as f64, y: vert[1] as f64, z: vert[2] as f64}).collect::<Vec<Vertex>>();

        for triangle in mesh.triangles()
        {
            let normal : [f32;3] =  triangle.normal().into();
            let mut converted_tri = IndexedTriangle{verts:[triangle.vertices_indices()[0],triangle.vertices_indices()[1],triangle.vertices_indices()[2]]};
            let mut v0 = vertices[converted_tri.verts[0]];
            let mut v1 = vertices[converted_tri.verts[1]];
            let mut v2 = vertices[converted_tri.verts[2]];
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
            let mut v0 = vertices[converted_tri.verts[0]];
            let mut v1 = vertices[converted_tri.verts[1]];
            let mut v2 = vertices[converted_tri.verts[2]];


            if v0 <v1 && v0 < v2 {
                triangles.push(converted_tri);
            }

            else if v1 <v2 && v1 < v0 {
                let temp = converted_tri.verts[0];
                converted_tri.verts[0] = converted_tri.verts[1];
                converted_tri.verts[1] = converted_tri.verts[2];
                converted_tri.verts[2] = temp;
                triangles.push(converted_tri);
            }
            else{
                let temp = converted_tri.verts[0];
                converted_tri.verts[0] = converted_tri.verts[2];
                converted_tri.verts[2] = converted_tri.verts[1];
                converted_tri.verts[1] = temp;
                triangles.push(converted_tri);
            }
        }


        Some((vertices,triangles))


     }
}

#[derive(Deserialize, Debug)]
struct Relationships{
    Relationship : Vec<Relationship>
}

#[derive(Deserialize, Debug)]
struct Relationship{
    Type : String,
    Target : String,
    Id : String,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "model")]
struct ThreeMFModel{
    resources : ThreeMFResource,
}

#[derive(Deserialize, Debug)]
struct ThreeMFResource{
    object : ThreeMFObject,
}

#[derive(Deserialize, Debug)]
struct ThreeMFObject{
    mesh : ThreeMFMesh,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "triangle")]
struct ThreeMFTriangle{
    v1 : usize,
    v2 : usize,
    v3 : usize,
}

#[derive(Deserialize, Debug)]
struct ThreeMFMesh{
    vertices : ThreeMFVertices,
    triangles : ThreeMFTriangles,
}

#[derive(Deserialize, Debug)]
struct ThreeMFVertices{
    #[serde(rename = "vertex", default)]
    list : Vec<Vertex>,
}

#[derive(Deserialize, Debug)]
struct ThreeMFTriangles{
    #[serde(rename = "triangle", default)]
    list : Vec<ThreeMFTriangle>,
}

pub struct ThreeMFLoader {}

impl Loader for ThreeMFLoader {
    fn load(&self , filepath: &str) -> Option< (Vec<Vertex>, Vec<IndexedTriangle>)>{


        let zipfile = std::fs::File::open(filepath).unwrap();

        let mut archive = zip::ZipArchive::new(zipfile).unwrap();




        let ModelPath = {
             let mut rel_file = match archive.by_name("_rels/.rels") {
                Ok(file) => file,
                Err(..) => {
                    println!("File not found");
                    return None;
                }
            };
            let mut rel_str = String::new();
            rel_file.read_to_string(&mut rel_str);

            let rel: Relationships = quick_xml::de::from_str(&rel_str).unwrap();

            let ModelPath = rel.Relationship[0].Target.clone();
            println!("Model Path: {}", ModelPath);
            ModelPath
        };

        let mut model_file = match archive.by_name(&ModelPath[1..]) {
            Ok(file) => file,
            Err(..) => {
                println!("File not found");
                return None;
            }
        };

        let mut model_str = String::new();
        model_file.read_to_string(&mut model_str);
        let model :ThreeMFModel = quick_xml::de::from_str(&model_str).unwrap();


        let mut triangles = vec![];
        let vertices = model.resources.object.mesh.vertices.list;

        for triangle in model.resources.object.mesh.triangles.list
        {
            let mut converted_tri = IndexedTriangle{verts:[triangle.v1,triangle.v2,triangle.v3]};
            let mut v0 = vertices[converted_tri.verts[0]];
            let mut v1 = vertices[converted_tri.verts[1]];
            let mut v2 = vertices[converted_tri.verts[2]];

            if v0 <v1 && v0 < v2 {
                triangles.push(converted_tri);
            }

            else if v1 <v2 && v1 < v0 {
                let temp = converted_tri.verts[0];
                converted_tri.verts[0] = converted_tri.verts[1];
                converted_tri.verts[1] = converted_tri.verts[2];
                converted_tri.verts[2] = temp;
                triangles.push(converted_tri);
            }
            else{
                let temp = converted_tri.verts[0];
                converted_tri.verts[0] = converted_tri.verts[2];
                converted_tri.verts[2] = converted_tri.verts[1];
                converted_tri.verts[1] = temp;
                triangles.push(converted_tri);
            }
        }


        Some((vertices,triangles))

     }
}