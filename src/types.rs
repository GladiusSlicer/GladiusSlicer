use std::cmp::Ordering;
use nalgebra::{Vector3,Point3};
use geo::Coordinate;

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Vertex{
    pub x : f64,
    pub y : f64,
    pub z : f64
}
impl Vertex{
    fn new(x:f64,y:f64,z:f64)-> Self{
        Vertex{x,y,z}
    }

}
impl From<Vertex> for Point3<f64> {
    fn from(v: Vertex) -> Self {
        Point3::new(v.x,v.y,v.z)
    }
}

impl PartialOrd for Vertex{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.z != other.z {
            self.z.partial_cmp(&other.z)
        }
        else if self.y != other.y{
            self.y.partial_cmp(&other.y)
        }
        else{
            self.x.partial_cmp(&other.x)
        }
    }
}


#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct IndexedTriangle{
    pub verts : [usize ;3],
    pub normal :Vector3<f32>,
}

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct IndexedLine{
    pub verts : [usize ;2]
}

#[derive( Clone,  Debug)]
pub enum Command{
    MoveTo{end: Coordinate<f64>},
    MoveAndExtrude{start: Coordinate<f64>, end: Coordinate<f64>},
    LayerChange{z: f64},
    SetState{new_state: StateChange},
    Delay{msec: u64},
    Arc{start: Coordinate<f64>, end: Coordinate<f64>, center: Coordinate<f64>, clockwise: bool}

}

#[derive( Clone, Debug,Default)]
pub struct StateChange{
    pub ExtruderTemp : Option<f64>,
    pub BedTemp : Option<f64>,
    pub MovementSpeed : Option<f64>,
    pub Retract: Option<bool>
}

impl StateChange{
    pub fn state_diff(&mut self,other: &StateChange) -> StateChange{

        StateChange{
            ExtruderTemp: {
                if self.ExtruderTemp == other.ExtruderTemp{ None }
                else {
                    self.ExtruderTemp = other.ExtruderTemp.or(self.ExtruderTemp); other.ExtruderTemp
                }
            },
            BedTemp: {
                if self.BedTemp == other.BedTemp{ None }
                else { self.BedTemp = other.BedTemp.or(self.BedTemp); other.BedTemp }
            },
            MovementSpeed: {
                if self.MovementSpeed == other.MovementSpeed{ None }
                else { self.MovementSpeed = other.MovementSpeed.or(self.MovementSpeed);other.MovementSpeed }
            },
            Retract: {
                if self.Retract == other.Retract{ None }
                else { self.Retract = other.Retract.or(self.Retract); other.Retract }
            }
        }

    }

    pub fn combine(&self,other: &StateChange) -> StateChange{

        StateChange{
            ExtruderTemp: { other.ExtruderTemp.or(self.ExtruderTemp) },
            BedTemp: { other.BedTemp.or(self.BedTemp) },
            MovementSpeed: { other.MovementSpeed.or(self.MovementSpeed) },
            Retract: { other.Retract.or(self.Retract) },
        }

    }
}