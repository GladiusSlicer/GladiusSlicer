use crate::{SlicerErrors, TopAndBottomLayersPass};
use gladius_shared::types::*;
use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::Rc;

use log::{trace};

/*

    Rough algortim 

    build tower 
        For each point store all edges and face connected to but above it

    progress up tower



    


*/

#[inline]
fn line_z_intersection(z: f64, v_start: Vertex, v_end: Vertex) -> Vertex {
    let z_normal = (z - v_start.z) / (v_end.z - v_start.z);
    let y = lerp(v_start.y, v_end.y, z_normal);
    let x = lerp(v_start.x, v_end.x, z_normal);
    Vertex { x, y, z }
}

#[inline]
fn lerp(a: f64, b: f64, f: f64) -> f64 {
    a + f * (b - a)
}

pub struct TriangleTower {
    vertices: Vec<Vertex>,
    tower_vertices: Vec<TowerVertex>,
}

impl TriangleTower {
    pub fn from_triangles_and_vertices(
        triangles: &[IndexedTriangle],
        vertices: Vec<Vertex>,
    ) -> Result<Self, SlicerErrors> {
        let mut future_tower_vert: Vec<Vec<TriangleEvent>> =
            (0..vertices.len()).map(|_| vec![]).collect();

        
        //for each triangle add it to the tower

        for (triangle_index, index_tri) in triangles.iter().enumerate() {

            //index 0 is always lowest
            future_tower_vert[index_tri.verts[0]].push(TriangleEvent::MiddleVertex {
                trailing_edge: index_tri.verts[1],
                leading_edge: index_tri.verts[2],
                triangle: triangle_index,
            });


            // depending what is the next vertex is its either leading or trailing
            if vertices[index_tri.verts[1]] < vertices[index_tri.verts[2]] {
                future_tower_vert[index_tri.verts[1]].push(TriangleEvent::TrailingEdge {
                    trailing_edge: index_tri.verts[2],
                    triangle: triangle_index,
                })
            }
            else {
                future_tower_vert[index_tri.verts[2]].push(TriangleEvent::LeadingEdge {
                    leading_edge: index_tri.verts[1],
                    triangle: triangle_index,
                })
            }
        }

        //for each triangle event, add it to the lowest vertex and
        //create a list of all vertices and there above edges

        let res_tower_vertices: Result<Vec<TowerVertex>, SlicerErrors> = future_tower_vert
            .into_iter()
            .enumerate()
            .map(|(index, events)| {
                join_triangle_event(events, index).map(|fragments| TowerVertex {
                    start_index: index,
                    next_ring_fragments: fragments,
                })
            })
            .collect();

        //propagate errors
        let mut tower_vertices = res_tower_vertices?;

        //sort lowest to highest
        tower_vertices.sort_by(|a, b| {
            vertices[a.start_index]
                .partial_cmp(&vertices[b.start_index])
                .expect("STL ERROR: No Points should have NAN values")
        });

        Ok(Self {
            vertices,
            tower_vertices,
        })
        
    }

    pub fn get_height_of_vertex(&self, index: usize) -> f64 {
        if index >= self.tower_vertices.len() {
            f64::INFINITY
        } else {
            self.vertices[self.tower_vertices[index].start_index].z
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct TowerVertex {
    pub next_ring_fragments: Vec<TowerRing>,
    pub start_index: usize,
}
#[derive(Clone, Debug, PartialEq)]
struct TowerRing {
    elements: Vec<TowerRingElement>,
}

impl TowerRing {
    /*fn repair_loop(&mut self) -> Result<(), SlicerErrors> {
        if *self.last_element.borrow() == *self.first_element.borrow() {
            let mut ring_ptr = self.first_element.clone();

            while {
                let next = if let Some(next) = ring_ptr.borrow().next().cloned() {
                    next
                } else {
                    return Err(SlicerErrors::TowerGeneration);
                };

                ring_ptr = next;

                ring_ptr.borrow().next() != Some(&self.last_element)
            } {}
            let mut borrow = ring_ptr.borrow_mut();
            borrow.set_next(Some(self.first_element.clone()));

            self.last_element = self.first_element.clone();
        }

        Ok(())
    }*/

    fn join_rings(mut first:  TowerRing, mut second: TowerRing) -> Result<Self, SlicerErrors> {

        first.elements.extend(&mut second.elements.drain(1..));


        //new_frag.repair_loop()?;

        Ok(first)
    }
    /*
    fn add_to_end(&mut self, second: TowerRing) {
        let second_next = second.first_element.borrow().next_clone();

        self.last_element.borrow_mut().set_next(second_next);
        self.last_element = second.last_element;

        self.repair_loop();
    }*/

    //splits the ring by removing elements and returning fragments without that edge
    
    fn split_on_edge(mut self, edge: usize) -> Result<Vec<Self>, SlicerErrors> {

        let mut new_ring = vec![];
        let mut frags = vec![];

        for e in self.elements.drain(..){
            if let TowerRingElement::Edge { end_index, .. } = e{
                if end_index == edge{
                    frags.push(TowerRing{elements: new_ring});
                    new_ring = vec![];
                }
                else{
                    new_ring.push (e)
                }
            }
            else{
                new_ring.push(e);
            }
        }

        if(frags.is_empty() ){
            //add in the fragment
            frags.push(TowerRing{elements: new_ring});
        }
        else{
            //append to the begining to prevent ophaned segments
            if frags[0].elements.len() ==0{
                frags[0].elements = new_ring;
            }
            else{
                new_ring.extend(&mut frags[0].elements.drain(1..));
                frags[0].elements = new_ring;
            }
            
        }

        //remove all fragments that are single sized and faces. They ends with that vertex

        frags.retain(|frag|{
            frag.elements.len() > 1

        });

        Ok(frags)
    }
}

impl Display for TowerRing {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for e in self.elements.iter(){
            write!(f, "{} ", e)?;
        }

        Ok(())
    }
}

/*
impl Display for TowerRing {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if *self.first_element.borrow() == *self.last_element.clone().borrow() {
            write!(f, "Loop! ")?;
        }

        match *self.first_element.borrow() {
            TowerRingElement::Face { triangle_index, .. } => {
                write!(f, "First F{} ", triangle_index)
            }
            TowerRingElement::Edge { end_index, .. } => {
                write!(f, "First E{} ", end_index)
            }
        }?;

        match *self.last_element.borrow() {
            TowerRingElement::Face { triangle_index, .. } => {
                write!(f, "Last F{} ", triangle_index)
            }
            TowerRingElement::Edge { end_index, .. } => {
                write!(f, "Last E{} ", end_index)
            }
        }?;

        let mut ring_ptr = self.first_element.clone();

        while {
            match ring_ptr.borrow().deref() {
                TowerRingElement::Face { triangle_index, .. } => {
                    write!(f, "F{} ", triangle_index)
                }
                TowerRingElement::Edge { end_index, .. } => {
                    write!(f, "E{} ", end_index)
                }
            }?;
            write!(f, "-> ")?;
            let next = ring_ptr.borrow().next_clone().expect("Next Must Be valid");
            ring_ptr = next;

            *ring_ptr.borrow() != *self.last_element.borrow()
        } {}

        match ring_ptr.borrow().deref() {
            TowerRingElement::Face { triangle_index, .. } => {
                write!(f, "F{} ", triangle_index)
            }
            TowerRingElement::Edge { end_index, .. } => {
                write!(f, "E{} ", end_index)
            }
        }?;

        write!(f, "")
    }
}*/

#[derive(Clone, Debug, Eq)]
enum TowerRingElement {
    Face {
        triangle_index: usize,
    },
    Edge {
        start_index: usize,
        end_index: usize,
    },
}

impl Display for TowerRingElement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            TowerRingElement::Face { triangle_index, .. } => {
                write!(f, "F{} ", triangle_index)
            }
            TowerRingElement::Edge { end_index, .. } => {
                write!(f, "E{} ", end_index)
            }
        }
    }
}
/* 
impl TowerRingElement {
    fn next_clone(&self) -> Option<Rc<RefCell<TowerRingElement>>> {
        match self {
            TowerRingElement::Face { next, .. } => next.clone(),
            TowerRingElement::Edge { next, .. } => next.clone(),
        }
    }

    fn next(&self) -> Option<&Rc<RefCell<TowerRingElement>>> {
        match self {
            TowerRingElement::Face { next, .. } => next.as_ref(),
            TowerRingElement::Edge { next, .. } => next.as_ref(),
        }
    }
    fn set_next(&mut self, n: Option<Rc<RefCell<TowerRingElement>>>) {
        match self {
            TowerRingElement::Edge { ref mut next, .. } => *next = n,
            TowerRingElement::Face { ref mut next, .. } => *next = n,
        }
    }
    /*
    fn deep_clone(&self) -> TowerRingElement {
        match self {
            TowerRingElement::Edge {
                start_index,
                end_index,
                ..
            } => TowerRingElement::Edge {
                start_index: *start_index,
                end_index: *end_index,
                next: None,
            },
            TowerRingElement::Face { triangle_index, .. } => TowerRingElement::Face {
                triangle_index: *triangle_index,
                next: None,
            },
        }
    }*/
}
*/
impl PartialEq for TowerRingElement {
    fn eq(&self, other: &Self) -> bool {
        match self {
            TowerRingElement::Edge {
                end_index,
                start_index,
                ..
            } => match other {
                TowerRingElement::Edge {
                    end_index: oei,
                    start_index: osi,
                    ..
                } => end_index == oei && start_index == osi,
                _ => false,
            },
            TowerRingElement::Face { triangle_index, .. } => match other {
                TowerRingElement::Face {
                    triangle_index: oti,
                    ..
                } => oti == triangle_index,
                _ => false,
            },
        }
    }
}

impl Hash for TowerRingElement {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            TowerRingElement::Edge {
                end_index,
                start_index,
                ..
            } => {
                end_index.hash(state);
                start_index.hash(state);
            }
            TowerRingElement::Face { triangle_index, .. } => {
                triangle_index.hash(state);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TriangleEvent {
    MiddleVertex {
        leading_edge: usize,
        triangle: usize,
        trailing_edge: usize,
    },
    LeadingEdge {
        leading_edge: usize,
        triangle: usize,
    },
    TrailingEdge {
        triangle: usize,
        trailing_edge: usize,
    },
}

fn join_triangle_event(
    events: Vec<TriangleEvent>,
    starting_point: usize,
) -> Result<Vec<TowerRing>, SlicerErrors> {
    //debug!("Tri events = {:?}",events);
    let mut element_list: Vec<TowerRing> = Vec::new();
    for event in events.iter() {
        match event {
            TriangleEvent::LeadingEdge {
                leading_edge,
                triangle,
            } => {
                let triangle_element = TowerRingElement::Face {
                    triangle_index: *triangle,
                };
                let edge_element =TowerRingElement::Edge {
                    start_index: starting_point,
                    end_index: *leading_edge,
                };
                
                let new_ring = TowerRing{elements: vec![edge_element,triangle_element] };

                element_list.push(new_ring);
            }
            TriangleEvent::TrailingEdge {
                triangle,
                trailing_edge,
            } => {
                let edge_element = TowerRingElement::Edge {
                    start_index: starting_point,
                    end_index: *trailing_edge,
                };

                let triangle_element = TowerRingElement::Face {
                    triangle_index: *triangle,
                };
                let new_ring = TowerRing{elements: vec![triangle_element,edge_element] };

                element_list.push(new_ring);
            }
            TriangleEvent::MiddleVertex {
                leading_edge,
                triangle,
                trailing_edge,
            } => {
                let trail_edge_element = TowerRingElement::Edge {
                    start_index: starting_point,
                    end_index: *trailing_edge,
                };

                let triangle_element = TowerRingElement::Face {
                    triangle_index: *triangle,
                };

                let lead_edge_element = TowerRingElement::Edge {
                    start_index: starting_point,
                    end_index: *leading_edge,
                };
                let new_ring = TowerRing{elements: vec![lead_edge_element,triangle_element,trail_edge_element] };

                element_list.push(new_ring);
            }
        }
    }

    join_fragments(&mut element_list)?;

    Ok(element_list)
}

/*
fn join_fragments(fragments: &mut Vec<TowerRing>) {

    let mut hm = HashMap::new();
    let mut frags =  fragments.clone();
    for frag in fragments.drain(..){
        let last = frag.last_element.borrow().clone();
        hm.insert(last, frag);
    }

    let mut found = true;

    while found {
        for frag in fragments.drain(..){
            let last = frag.last_element.borrow().clone();
            hm.insert(last, frag);
        }
        found = false;
        for frag in frags.drain(..) {
            let first_el = frag.first_element.borrow().clone();
            let last_el = frag.last_element.borrow().clone();
            if first_el != last_el {
                if let Some(mut first) = hm.remove(&first_el) {
                    println!("here");

                    first.add_to_end(frag);
                    hm.remove(&last_el);
                    hm.insert(last_el, first);
                    found = true;
                }
            }
        }
        frags.extend(hm.values().map(|v|  v.clone()));
    }

    fragments.extend(hm.drain().map(|(k,v)| v));

}
*/
fn join_fragments(fragments: &mut Vec<TowerRing>) -> Result<(), SlicerErrors> {
    /*

        for frag in &*fragments{
            println!("fragment {}",frag);
        }
    */
    'outer: loop {
        for first_pos in 0..fragments.len() {
            for second_pos in (first_pos + 1)..fragments.len() {
                let first = fragments
                    .get(first_pos)
                    .ok_or(SlicerErrors::TowerGeneration)?;
                let second = fragments
                    .get(second_pos)
                    .ok_or(SlicerErrors::TowerGeneration)?;

                if first.elements.last() == second.elements.get(0) {
                    let second_r = fragments.swap_remove(second_pos);
                    let first_r = fragments.swap_remove(first_pos);

                    fragments.push(TowerRing::join_rings(first_r, second_r)?);

                    continue 'outer;
                }
            }
        }

        //No more points to join
        return Ok(());
    }
}

pub struct TriangleTowerIterator<'s> {
    tower: &'s TriangleTower,
    tower_vert_index: usize,
    z_height: f64,
    active_rings: Vec<TowerRing>,
}

impl<'s> TriangleTowerIterator<'s> {
    pub fn new(tower: &'s TriangleTower) -> Self {
        let z_height = tower.get_height_of_vertex(0);
        Self {
            z_height,
            tower,
            tower_vert_index: 0,
            active_rings: vec![],
        }
    }

    pub fn advance_to_height(&mut self, z: f64) -> Result<(), SlicerErrors> {
        //println!("Advance to height {} {} {}", self.tower.get_height_of_vertex(self.tower_vert_index), z, self.tower.tower_vertices[self.tower_vert_index].start_index);

        while self.tower.get_height_of_vertex(self.tower_vert_index) < z
            && self.tower.tower_vertices.len() + 1 != self.tower_vert_index
        {
            let pop_tower_vert = self.tower.tower_vertices[self.tower_vert_index].clone();

            //Create Frags from rings by removing current edges
            let vec_frag: Result<Vec<Vec<TowerRing>>, SlicerErrors> = self
                .active_rings
                .drain(..)
                .map(|tower_ring| tower_ring.split_on_edge(pop_tower_vert.start_index))
                .collect();

            let mut frags: Vec<TowerRing> = vec_frag?
                .drain(..)
                .flat_map(|vec_frag| vec_frag.into_iter())
                .collect();

            trace!("split edge: {:?}", pop_tower_vert.start_index );
            trace!("split frags:");
            for f in &frags{
                trace!("\t{}",f);
            }
            

            //Add the new fragments

            frags.extend(pop_tower_vert.next_ring_fragments.clone().into_iter());
            trace!("all frags:");
            for f in &frags{
                trace!("\t{}",f);
            }

            join_fragments(&mut frags)?;
            trace!("join frags:");
            for f in &frags{
                trace!("\t{}",f);
            }
            self.active_rings = frags;
            self.tower_vert_index += 1;
        }

        self.z_height = z;

        Ok(())
    }

    pub fn get_points(&self) -> Vec<Vec<Vertex>> {

        self.active_rings.iter().map(|ring|{
            let mut points : Vec<Vertex> = ring.elements.iter()
                .filter_map(|e| {
                    if let TowerRingElement::Edge {
                        start_index,
                        end_index,
                        ..
                    } = e
                    {
                        Some(line_z_intersection(
                            self.z_height,
                            self.tower.vertices[*start_index],
                            self.tower.vertices[*end_index],
                        ))
                    }
                    else{
                        None
                    }
                }) 
                .collect();
            
            //complete loop
            if (points.len() > 0){
                points.push(points[0]);
            }

            points

        }).collect()

    }
}

pub fn create_towers(
    models: &[(Vec<Vertex>, Vec<IndexedTriangle>)],
) -> Result<Vec<TriangleTower>, SlicerErrors> {
    models
        .iter()
        .map(|(vertices, triangles)| {
            TriangleTower::from_triangles_and_vertices(triangles, vertices.clone())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde::de::Expected;

    use super::*;

    #[test]
    fn join_rings_test() {
        let r1 = TowerRing{elements: vec![
            TowerRingElement::Edge {
                start_index: 0,
                end_index: 1,
            },
            TowerRingElement::Face {
                triangle_index: 0,
            },
            TowerRingElement::Edge {
                start_index: 0,
                end_index: 2,
            }
        ]};

        let r2 = TowerRing{elements: vec![
            TowerRingElement::Edge {
                start_index: 0,
                end_index: 2,
            },
            TowerRingElement::Face {
                triangle_index: 2,
            },
            TowerRingElement::Edge {
                start_index:4,
                end_index: 6,
            }
        ]};

        let r3 = TowerRing{elements: vec![
            TowerRingElement::Edge {
                start_index: 0,
                end_index: 1,
            },
            TowerRingElement::Face {
                triangle_index: 0,
            },
            TowerRingElement::Edge {
                start_index: 0,
                end_index: 2,
            },
            TowerRingElement::Face {
                triangle_index: 2,
            },
            TowerRingElement::Edge {
                start_index:4,
                end_index: 6,
            }
        ]};

        assert_eq!(TowerRing::join_rings(r1,r2),Ok(r3));
    }
    
    #[test]
    fn split_on_edge_test() {
        let r1 = TowerRing{elements: vec![
            TowerRingElement::Edge {
                start_index: 0,
                end_index: 1,
            },
            TowerRingElement::Face {
                triangle_index: 0,
            },
            TowerRingElement::Edge {
                start_index: 0,
                end_index: 2,
            },
            TowerRingElement::Face {
                triangle_index: 2,
            },
            TowerRingElement::Edge {
                start_index:4,
                end_index: 6,
            }
        ]};

        let frags = r1.split_on_edge(2).unwrap();

        let expected = vec![
            TowerRing{elements: vec![
            TowerRingElement::Edge {
                start_index: 0,
                end_index: 1,
            },
            TowerRingElement::Face {
                triangle_index: 0,
            }]},
            TowerRing{elements: vec![
            TowerRingElement::Face {
                triangle_index: 2,
            },
            TowerRingElement::Edge {
                start_index:4,
                end_index: 6,
            }]}
        ];
        assert_eq!(frags,expected);
    }

    #[test]
    fn assemble_fragment_test() {
        let mut frags = 
            vec![
                TowerRing{elements: vec![
                TowerRingElement::Edge {
                    start_index: 0,
                    end_index: 1,
                },
                TowerRingElement::Face {
                    triangle_index: 0,
                },
                TowerRingElement::Edge {
                    start_index: 0,
                    end_index: 2,
                },
                TowerRingElement::Face {
                    triangle_index: 2,
                },
                TowerRingElement::Edge {
                    start_index:4,
                    end_index: 6,
                }
            ]},
            TowerRing{elements: vec![ 
                TowerRingElement::Edge {
                    start_index:4,
                    end_index: 6,
                },
                TowerRingElement::Face {
                    triangle_index: 2,
                },
                TowerRingElement::Edge {
                    start_index: 0,
                    end_index: 1,
                }
            
            ]}];

         join_fragments(&mut frags).unwrap();






        let expected = vec![
            TowerRing{elements: vec![
                TowerRingElement::Edge {
                    start_index: 0,
                    end_index: 1,
                },
                TowerRingElement::Face {
                    triangle_index: 0,
                },
                TowerRingElement::Edge {
                    start_index: 0,
                    end_index: 2,
                },
                TowerRingElement::Face {
                    triangle_index: 2,
                },
                TowerRingElement::Edge {
                    start_index:4,
                    end_index: 6,
                },
                TowerRingElement::Face {
                    triangle_index: 2,
                },
                TowerRingElement::Edge {
                    start_index: 0,
                    end_index: 1,
                }
        
        ]}];


        assert_eq!(frags,expected);
    }


}
