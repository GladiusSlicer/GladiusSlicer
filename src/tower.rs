use std::cell::RefCell;
use std::rc::Rc;
use itertools::Itertools;
use std::ops::{DerefMut, Deref};
use crate::types::*;
use std::fmt::{Display, Formatter};

fn line_z_intersection(z: f64, v_start : Vertex, v_end : Vertex) -> Option<Vertex>{

    if v_end.z == v_start.z{
        None
    }
    else {
        let z_normal = (z - v_start.z) / (v_end.z - v_start.z);
        let y = lerp(v_start.y,v_end.y,z_normal);
        let x = lerp(v_start.x,v_end.x,z_normal);

        Some(Vertex {  x,y,z})
    }
}


fn lerp(a:f64, b:f64, f:f64) -> f64
{
    a + f * (b - a)
}

pub struct TriangleTower{
    vertices : Vec<Vertex>,
    tower_vertices: Vec<TowerVertex>,

}

impl TriangleTower{
    pub fn from_triangles_and_vertices(triangles: &Vec<IndexedTriangle> , mut vertices: Vec<Vertex>) -> Self{

        let mut future_tower_vert: Vec<Vec<TriangleEvent>> = (0..vertices.len()).map(|_| vec![]).collect();

        for (triangle_index,index_tri) in triangles.iter().enumerate(){
            future_tower_vert[index_tri.verts[0]].push(TriangleEvent::MiddleVertex { trailing_edge: index_tri.verts[1], leading_edge: index_tri.verts[2], triangle:triangle_index });

            if vertices[index_tri.verts[1]] < vertices[index_tri.verts[2]] {
                future_tower_vert[index_tri.verts[1]].push(TriangleEvent::TrailingEdge { trailing_edge: index_tri.verts[2], triangle: triangle_index })
            }

            if vertices[index_tri.verts[2]] < vertices[index_tri.verts[1]] {
                future_tower_vert[index_tri.verts[2]].push(TriangleEvent::LeadingEdge { leading_edge: index_tri.verts[1], triangle: triangle_index })
            }
        }

        let mut tower_vertices : Vec<TowerVertex> =  future_tower_vert.into_iter().enumerate()
            .map(| ( index,events)|{
                 TowerVertex { start_index: index, next_ring_fragments: joinTriangleEvent(events, index) }
            }).collect();

        tower_vertices.sort_by(|a,b| vertices[a.start_index].partial_cmp(&vertices[b.start_index]).unwrap() );

        Self{tower_vertices,vertices}

    }


    pub fn get_height_of_vertex(&self,index: usize) -> f64 {
        if index >= self.tower_vertices.len(){
            f64::INFINITY
        }
        else {
            self.vertices[self.tower_vertices[index].start_index].z
        }
    }
}

#[derive( Clone,  Debug, PartialEq)]
struct TowerVertex{
    pub next_ring_fragments: Vec<TowerRing>,
    pub start_index : usize,
}
#[derive( Clone,  Debug, PartialEq)]
struct TowerRing{
    pub first_element : Rc<RefCell<TowerRingElement>>,
    pub last_element : Rc<RefCell<TowerRingElement>>,
}

impl TowerRing{
    fn repair_loop(&mut self){
        if *self.last_element.borrow() == *self.first_element.borrow()
        {
            let mut ring_ptr = self.first_element.clone();

            while
            {
                let next = ring_ptr.borrow().next_clone().unwrap();
                ring_ptr = next;

                *ring_ptr.borrow().next_clone().unwrap().borrow() != *self.last_element.borrow()
            } {}
            let mut borrow = ring_ptr.borrow_mut();
            borrow.set_next(Some(self.first_element.clone()));

            self.last_element = self.first_element.clone();
        }
    }
    fn join_rings(first: TowerRing, second: TowerRing) -> Self{

        let second_next = second.first_element.borrow().next_clone();

        first.last_element.borrow_mut().set_next(second_next);

        let mut new_frag = TowerRing{first_element: first.first_element, last_element: second.last_element};

        new_frag.repair_loop();

        new_frag
    }

    fn split_on_edge(mut self, edge: usize) -> Vec<Self>{


        let mut frags = vec![];

        let mut ring_ptr = self.first_element.clone();
        let mut last_ptr = self.last_element.clone();

        let mut temp_frag = TowerRing{first_element: self.first_element.clone(), last_element: self.last_element.clone()};

        let mut found = false;

        while {
            last_ptr = ring_ptr.clone();
            let next = ring_ptr.borrow().next_clone();
            ring_ptr = next.unwrap();
            if let TowerRingElement::Edge { end_index, .. } = *ring_ptr.borrow()
            {
                if end_index == edge{
                    last_ptr.borrow_mut().set_next(None);
                    temp_frag.last_element = last_ptr.clone();
                    frags.push(std::mem::replace(&mut temp_frag,TowerRing{first_element: ring_ptr.borrow().next_clone().unwrap(), last_element: self.last_element.clone()} ));
                    self.first_element = ring_ptr.borrow().next_clone().unwrap();

                    found = true;

                }
            } else {

            }

            *ring_ptr.borrow() != *self.last_element.borrow()

        }
        {}

        if found{
            let frag = frags.remove(0);
            self.last_element = frag.last_element;
        }

        frags.push(self);

        frags.retain(|frag| frag.first_element.borrow().next_clone().is_some());

        frags

    }
}

impl Display for TowerRing{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {


        if *self.first_element.borrow() == *self.last_element.clone().borrow(){
            write!(f,"Loop! ")?;
        }

        match *self.first_element.borrow(){
            TowerRingElement::Face { triangle_index,.. } => {write!(f,"First F{} ",triangle_index)}
            TowerRingElement::Edge { end_index,.. } => {write!(f,"First E{} ",end_index)}
        }?;


        match *self.last_element.borrow(){
            TowerRingElement::Face { triangle_index,.. } => {write!(f,"Last F{} ",triangle_index)}
            TowerRingElement::Edge { end_index,.. } => {write!(f,"Last E{} ",end_index)}
        }?;


        let mut ring_ptr = self.first_element.clone();

        while
        {


            match ring_ptr.borrow().deref(){
                TowerRingElement::Face { triangle_index,.. } => {write!(f,"F{} ",triangle_index)}
                TowerRingElement::Edge { end_index,.. } => {write!(f,"E{} ",end_index)}
            }?;
            write!(f,"-> ")?;
            let next = ring_ptr.borrow().next_clone().unwrap();
            ring_ptr = next;


            *ring_ptr.borrow() != *self.last_element.borrow()
        } {}

        match ring_ptr.borrow().deref(){
            TowerRingElement::Face { triangle_index,.. } => {write!(f,"F{} ",triangle_index)}
            TowerRingElement::Edge { end_index,.. } => {write!(f,"E{} ",end_index)}
        }?;


        write!(f,"")

    }
}



#[derive( Clone,  Debug)]
enum TowerRingElement{
    Face{ triangle_index: usize, next: Option<Rc<RefCell<TowerRingElement>>>  },
    Edge{ start_index: usize, end_index: usize,next: Option<Rc<RefCell<TowerRingElement>>> },
}

impl TowerRingElement{
    fn next_clone(&self) -> Option<Rc<RefCell<TowerRingElement>>>
    {
        match self{
            TowerRingElement::Face {next,..} => next.clone(),
            TowerRingElement::Edge {next,..} => next.clone()
        }

    }
    fn set_next(&mut self,  n: Option<Rc<RefCell<TowerRingElement>>>){
        match self{
            TowerRingElement::Edge { ref mut next, .. } => *next = n,
            TowerRingElement::Face { ref mut next, .. } => *next = n
        }

    }

    fn deep_clone(&self) -> TowerRingElement{
        match self{
            TowerRingElement::Edge {start_index,end_index,.. } => TowerRingElement::Edge { start_index: *start_index,end_index: *end_index,next:None},
            TowerRingElement::Face { triangle_index, ..} => TowerRingElement::Face { triangle_index: *triangle_index,next:None}
        }

    }



}

impl PartialEq for TowerRingElement{
    fn eq(&self, other: &Self) -> bool {
        match self{
            TowerRingElement::Edge {end_index, start_index,..} => {
                match other{
                    TowerRingElement::Edge {end_index: oei, start_index: osi, ..} => end_index == oei && start_index == osi,
                    _=> false
                }
            }
            TowerRingElement::Face {triangle_index, ..} => {
                match other{
                    TowerRingElement::Face {triangle_index : oti, ..}  => oti == triangle_index,
                    _=> false
                }
            }
        }
    }
}

#[derive( Clone,  Debug, PartialEq)]
pub enum TriangleEvent{
    MiddleVertex{leading_edge: usize, triangle: usize, trailing_edge: usize },
    LeadingEdge{ leading_edge: usize, triangle: usize},
    TrailingEdge{ triangle: usize, trailing_edge: usize }

}

fn joinTriangleEvent(events: Vec<TriangleEvent>, starting_point: usize) -> Vec<TowerRing>
{

    //debug!("Tri events = {:?}",events);
    let mut element_list : Vec<TowerRing> = Vec::new();
    for event in events.iter(){
        match event{
            TriangleEvent::LeadingEdge { leading_edge, triangle} => {


                let triangle_element = Rc::new(RefCell::new(TowerRingElement::Face {triangle_index: *triangle, next: None }));
                let edge_element = Rc::new(RefCell::new(TowerRingElement::Edge {start_index:starting_point,end_index: *leading_edge,next:Some(triangle_element.clone()) }));
                let new_ring = TowerRing{first_element: edge_element, last_element: triangle_element};
                element_list.push(new_ring);

            },
            TriangleEvent::TrailingEdge{ triangle, trailing_edge } => {

                let edge_element = Rc::new(RefCell::new(TowerRingElement::Edge {start_index:starting_point,end_index: *trailing_edge,next:None }));
                let triangle_element = Rc::new(RefCell::new(TowerRingElement::Face {triangle_index: *triangle, next: Some(edge_element.clone())}));
                let new_ring = TowerRing{first_element: triangle_element, last_element: edge_element};
                element_list.push(new_ring);

            },
            TriangleEvent::MiddleVertex{leading_edge, triangle, trailing_edge } => {
                let trail_edge_element = Rc::new(RefCell::new(TowerRingElement::Edge {start_index:starting_point,end_index: *trailing_edge,next:None }));
                let triangle_element = Rc::new(RefCell::new(TowerRingElement::Face {triangle_index: *triangle, next: Some(trail_edge_element.clone())}));
                let lead_edge_element = Rc::new(RefCell::new(TowerRingElement::Edge {start_index:starting_point,end_index: *leading_edge,next: Some(triangle_element.clone()) }));
                let new_ring = TowerRing{first_element: lead_edge_element, last_element: trail_edge_element};
                element_list.push(new_ring);
            },


        }
    }

    join_fragments(&mut element_list);

    element_list

}


fn join_fragments(fragments: &mut Vec<TowerRing>){

    'outer: loop
    {
        for first_pos  in 0..fragments.len(){
            for second_pos in (first_pos+1)..fragments.len(){

                let first = fragments.get(first_pos).unwrap();
                let second = fragments.get(second_pos).unwrap();

                if first.last_element == second.first_element
                {
                    let second_r = fragments.swap_remove(second_pos);
                    let first_r = fragments.swap_remove(first_pos);

                    fragments.push(TowerRing::join_rings(first_r,second_r));

                    continue 'outer;
                }
            }
        }

        //No more points to join
        return;
    }


}

pub struct TriangleTowerIterator<'s>{
    tower : &'s TriangleTower,
    tower_vert_index : usize,
    z_height :f64,
    active_rings: Vec<TowerRing>,
}

impl<'s> TriangleTowerIterator<'s>{

    pub fn new(tower: &'s TriangleTower) -> Self{
        let z_height = tower.get_height_of_vertex(0);
        Self{z_height,tower,tower_vert_index : 0, active_rings: vec![]}
    }

    pub fn advance_to_height(&mut self , z:f64) {

        while  self.tower.tower_vertices.len() +1 != self.tower_vert_index &&  self.tower.get_height_of_vertex(self.tower_vert_index ) < z
        {

            let pop_tower_vert = self.tower.tower_vertices[self.tower_vert_index].clone();

            //Create Frags from rings by removing current edges
            let mut frags :Vec<TowerRing> = self.active_rings
                .drain(..)
                .map(|tower_ring| {
                    tower_ring.split_on_edge(pop_tower_vert.start_index).into_iter()
                }).flatten().collect();

            //Add the new fragments
            frags.append(&mut pop_tower_vert.next_ring_fragments.clone() );

            join_fragments(&mut frags);
            self.active_rings = frags;
            self.tower_vert_index += 1;
        }

        self.z_height = z;
    }

    pub fn get_points(&self) -> Vec<Vec<Vertex>> {

        let mut points_vec = vec![];
        for ring in &self.active_rings{
            let mut points = vec![];
            let mut ring_ptr =  ring.first_element.clone();
            while {

                if let TowerRingElement::Edge { start_index, end_index,..} = ring_ptr.borrow().deref()
                {
                    points.push( line_z_intersection(self.z_height, self.tower.vertices[*start_index ], self.tower.vertices[*end_index ]).unwrap() );
                }
                let next = ring_ptr.borrow().next_clone().expect("Rings must be complete Loops").clone();

                ring_ptr =next ;

                ring_ptr != ring.last_element
            }{}

            let first_point = points[0];

            points.push(first_point);
            points_vec.push(points);
        }

        points_vec


    }
}


