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
        {
            let mut first_ptr = first.first_element.clone();

            while first_ptr.borrow().next_clone().unwrap().borrow().deref() != first.last_element.borrow().deref() {
                let next = first_ptr.borrow().next_clone().unwrap();
                first_ptr = next;
            }
            {}

            let mut borrow = first_ptr.borrow_mut();

            borrow.set_next(Some(second.first_element.clone()));
        }

        let mut new_frag = TowerRing{first_element: first.first_element, last_element: second.last_element};

        new_frag.repair_loop();

        new_frag
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

    for frag in &*fragments{
        //debug!("fragment {}",frag);
    }

    let mut found = true;
    while(found)
    {
        if let Some(((first_pos,_),(second_pos,_))) = fragments.iter().enumerate().cartesian_product(fragments.iter().enumerate()).find(|((first_pos,first),(second_pos,second))|{

            if first_pos == second_pos{
                return false
            }
            else{

            }

            if let TowerRingElement::Edge{ end_index: first_end,.. } = first.last_element.borrow().deref()
            {
                if let TowerRingElement::Edge{ end_index: second_end,.. } = second.first_element.borrow().deref()
                {
                    *first_end == *second_end
                }
                else{
                    false
                }
            }
            else if let TowerRingElement::Face{ triangle_index:  first_tri,.. } = first.last_element.borrow().deref()
            {
                if let TowerRingElement::Face{ triangle_index: second_tri,.. } = second.first_element.borrow().deref()
                {
                    //println!("!!!!!!!");
                    *first_tri == *second_tri
                }
                else{
                    false
                }
            }
            else {
                false
            }

        })
        {
            //println!("join {} {}", first_pos,second_pos);
            let (first_r,second_r)  = if first_pos > second_pos{
                let first_r = fragments.remove(first_pos);
                let second_r = fragments.remove(second_pos);
                (first_r,second_r)
            }
            else{
                let second_r = fragments.remove(second_pos);
                let first_r = fragments.remove(first_pos);

                (first_r,second_r)
            };

            fragments.push(TowerRing::join_rings(first_r,second_r));

        }
        else{
            found = false;
        }

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
            //println!("Here");
            //debug!("Advance to height {} {} {}", self.tower.get_height_of_vertex(self.tower_vert_index), z, self.tower.tower_vertices[self.tower_vert_index].start_index);
            //println!("Advance to height {} {} {}", self.tower.get_height_of_vertex(self.tower_vert_index), z, self.tower.tower_vertices[self.tower_vert_index].start_index);
            for ring in &mut self.active_rings {

                let new = if let TowerRingElement::Edge{..}  = *ring.first_element.borrow() {
                    ring.first_element.borrow().next_clone().unwrap()
                }
                else{
                     ring.first_element.clone()
                };

                ring.first_element = new.clone();
                ring.last_element = new;

                //debug!("Input Ring {} ", ring);
            }


            let pop_tower_vert = self.tower.tower_vertices[self.tower_vert_index].clone();

            let mut frags = vec![];
            let mut rings = vec![];
            for mut tower_ring in self.active_rings.drain(..)
            {
                let mut ring_ptr = tower_ring.first_element.clone();
                let mut last_ptr = tower_ring.last_element.clone();

                //debug!("Tower ring = {}", tower_ring);

                while {
                    last_ptr = ring_ptr.clone();
                    let next = ring_ptr.borrow().next_clone();
                    ring_ptr = next.unwrap();
                    if let TowerRingElement::Edge { end_index, .. } = *ring_ptr.borrow()
                    {
                        end_index != pop_tower_vert.start_index && *ring_ptr.borrow() != *tower_ring.last_element.borrow()
                    } else {
                        *ring_ptr.borrow() != *tower_ring.last_element.borrow()
                    }

                }
                {}

                if *ring_ptr.borrow() == *tower_ring.last_element.borrow()
                {
                    //debug!("Next ring = {}", tower_ring);
                    rings.push(tower_ring)
                } else {

                    tower_ring.last_element = last_ptr.clone();
                    tower_ring.last_element.borrow_mut().set_next(None);
                    while {
                        if let TowerRingElement::Edge { end_index, .. } = *ring_ptr.borrow()
                        {
                            end_index == pop_tower_vert.start_index && *tower_ring.last_element.borrow() != *ring_ptr.borrow()
                        } else {
                             *tower_ring.last_element.borrow() != *ring_ptr.borrow()
                        }
                    }
                    {
                        last_ptr = ring_ptr.clone();
                        let next = ring_ptr.borrow().next_clone();
                        ring_ptr = next.unwrap();
                    }
                    if *tower_ring.last_element.borrow() != *ring_ptr.borrow()
                    {
                        tower_ring.first_element = last_ptr;

                        //debug!("Next Frag = {}", tower_ring);


                        let mut last_ptr = tower_ring.first_element.clone();
                        let mut temp = tower_ring.first_element.clone();
                        let mut ring_ptr = tower_ring.first_element.clone();

                        let mut deleteing = false;

                        while ring_ptr.borrow().next_clone().is_some() {
                            let next = ring_ptr.borrow().next_clone().unwrap();


                            if let TowerRingElement::Edge { end_index, .. } = *ring_ptr.borrow()
                            {
                                if deleteing {
                                    if end_index != pop_tower_vert.start_index {
                                        deleteing = false;

                                        temp = last_ptr.clone();
                                    }
                                } else {
                                    if end_index == pop_tower_vert.start_index {
                                        deleteing = true;
                                        last_ptr.borrow_mut().set_next(None);
                                        let new_ring = TowerRing { first_element: temp.clone(), last_element: last_ptr.clone() };
                                        //debug!("new Frag = {}", new_ring);

                                        frags.push(new_ring);

                                        temp = ring_ptr.clone();
                                    }
                                }
                            }
                            /*
                        match ring_ptr.borrow().deref(){
                            TowerRingElement::Face { triangle_index,.. } => {println!("F{} ",triangle_index)}
                            TowerRingElement::Edge { end_index,.. } => {println!("E{} ",end_index)}
                        };
                        */
                            last_ptr = ring_ptr.clone();
                            ring_ptr = next.clone();
                        }
                        {}

                        if !deleteing {
                            tower_ring.first_element = temp;
                            tower_ring.repair_loop();
                            //debug!("new Frag = {}", tower_ring);
                            frags.push(tower_ring);
                        }
                    }


                }
            }

            for mut fragment in pop_tower_vert.next_ring_fragments.clone() {
                if *fragment.first_element.borrow() == *fragment.last_element.borrow() {
                    //fragment.repair_loop();
                    rings.push(fragment);
                } else {
                    //debug!("point Frag = {}", fragment);
                    frags.push(fragment)
                }
            }

            join_fragments(&mut frags);

            rings.append(&mut frags);

            self.active_rings = rings;
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


