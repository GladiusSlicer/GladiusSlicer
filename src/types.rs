use std::cmp::Ordering;
use nalgebra::{Vector3,Point3};
use geo::Coordinate;
use serde::{Serialize, Deserialize};

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

impl std::ops::Mul<Vertex> for &Transform{
    type Output = Vertex;

    fn mul(self, rhs: Vertex) -> Self::Output {
        Vertex{
            x: self.0[0][0] * rhs.x +  self.0[0][1] * rhs.y +self.0[0][2] * rhs.z + self.0[0][3] ,
            y: self.0[1][0] * rhs.x +  self.0[1][1] * rhs.y +self.0[1][2] * rhs.z + self.0[1][3] ,
            z: self.0[2][0] * rhs.x +  self.0[2][1] * rhs.y +self.0[2][2] * rhs.z + self.0[2][3] ,
        }
    }
}

impl Transform{
    pub fn new_translation_transform(x: f64,y:f64,z:f64) -> Self{
        Transform(
            [[1.,0.,0.,x],
             [0.,1.,0.,y],
             [0.,0.,1.,z],
             [0.,0.,0.,1.]]
        )
    }
}

/*
impl std::ops::Mul<Transform> for Transform{
    type Output = Transform;

    fn mul(self, rhs: Transform) -> Self::Output {
        let arrays = [[0.0 ; 4];4];
        for 0..4

        Transform(arrays)
    }
}
*/
#[derive(Serialize, Deserialize, Debug)]
pub struct Transform([[f64;4];4]);




#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct IndexedTriangle{
    pub verts : [usize ;3],
    pub normal :Vector3<f32>,
}

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct IndexedLine{
    pub verts : [usize ;2]
}

pub struct Move{
    end: Coordinate<f64>,
    move_type: MoveType
}

pub enum MoveType{
    SolidInfill,
    Infill,
    Outer_Perimeter,
    Inner_Perimeter,
    Support,
    Travel

}

#[derive( Clone,  Debug)]
pub enum Command{
    MoveTo{end: Coordinate<f64>},
    MoveAndExtrude{start: Coordinate<f64>, end: Coordinate<f64>},
    LayerChange{z: f64},
    SetState{new_state: StateChange},
    Delay{msec: u64},
    Arc{start: Coordinate<f64>, end: Coordinate<f64>, center: Coordinate<f64>, clockwise: bool},
    //Used in optimization , should be optimized out
    NoAction

}
/*
impl Command{
    fn get_time(&self,state : &mut StateChange,location: &mut Coordinate<f64>) -> f32{
        match self {
            Command::MoveTo { end } => {0.0}
            Command::MoveAndExtrude { start, end } => {}
            Command::LayerChange { .. } => {0.0}
            Command::SetState { new_state } => {state.state_diff(new_state)}
            Command::Delay { msec } => { msec as f32/ 1000.0 }
            Command::Arc { start, end, center, clockwise } => {0.0}
            Command::NoAction => { 0.0}
        }
    }
}
*/
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


