use crate::settings::{LayerSettings, Settings};
use geo::contains::Contains;
use geo::prelude::SimplifyVW;
use geo::simplifyvw::SimplifyVWPreserve;
use geo::*;
use itertools::Itertools;
use nalgebra::Point3;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

pub struct Slice {
    pub main_polygon: MultiPolygon<f64>,
    pub remaining_area: MultiPolygon<f64>,
    pub support_interface: Option<MultiPolygon<f64>>,
    pub support_tower: Option<MultiPolygon<f64>>,
    pub fixed_chains: Vec<MoveChain>,
    pub chains: Vec<MoveChain>,
    pub bottom_height: f64,
    pub top_height: f64,
    pub layer_settings: LayerSettings,
}
impl Slice {
    pub fn from_single_point_loop<I>(
        line: I,
        bottom_height: f64,
        top_height: f64,
        layer_count: usize,
        settings: &Settings,
    ) -> Self
    where
        I: Iterator<Item = (f64, f64)>,
    {
        let polygon = Polygon::new(LineString::from_iter(line), vec![]);

        let layer_settings =
            settings.get_layer_settings(layer_count, (bottom_height + top_height) / 2.0);

        Slice {
            main_polygon: MultiPolygon(vec![polygon.simplifyvw_preserve(&0.01)]),
            remaining_area: MultiPolygon(vec![polygon]),
            support_interface: None,
            support_tower: None,
            fixed_chains: vec![],
            chains: vec![],
            bottom_height,
            top_height,
            layer_settings,
        }
    }

    pub fn from_multiple_point_loop(
        lines: MultiLineString<f64>,
        bottom_height: f64,
        top_height: f64,
        layer_count: usize,
        settings: &Settings,
    ) -> Self {
        let mut lines_and_area: Vec<(LineString<f64>, f64)> = lines
            .into_iter()
            .map(|line| {
                let area: f64 = line
                    .clone()
                    .into_points()
                    .iter()
                    .circular_tuple_windows::<(_, _)>()
                    .map(|(p1, p2)| (p1.x() + p2.x()) * (p2.y() - p1.y()))
                    .sum();
                (line, area)
            })
            .filter(|(_, area)| area.abs() > 0.0001)
            .collect();

        lines_and_area.sort_by(|(_l1, a1), (_l2, a2)| a2.partial_cmp(a1).unwrap());
        let mut polygons = vec![];

        for (line, area) in lines_and_area {
            if area > 0.0 {
                polygons.push(Polygon::new(line.clone(), vec![]));
            } else {
                //counter clockwise interior polygon
                let smallest_polygon = polygons
                    .iter_mut()
                    .rev()
                    .find(|poly| poly.contains(&line.0[0]))
                    .expect("Polygon order failure");
                smallest_polygon.interiors_push(line);
            }
        }

        let multi_polygon: MultiPolygon<f64> = MultiPolygon(polygons);

        let layer_settings =
            settings.get_layer_settings(layer_count, (bottom_height + top_height) / 2.0);

        Slice {
            main_polygon: multi_polygon.clone(),
            remaining_area: multi_polygon.simplifyvw(&0.0001),
            support_interface: None,
            support_tower: None,
            chains: vec![],
            fixed_chains: vec![],
            bottom_height,
            top_height,
            layer_settings,
        }
    }

    pub fn get_height(&self) -> f64 {
        (self.bottom_height + self.top_height) / 2.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum SolidInfillsTypes {
    Rectilinear,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum PartialInfillTypes {
    Linear,
    Rectilinear,
    Triangle,
    Cubic,
    Lightning,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename = "vertex")]
pub struct Vertex {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
impl Vertex {
    fn new(x: f64, y: f64, z: f64) -> Self {
        Vertex { x, y, z }
    }
}
impl From<Vertex> for Point3<f64> {
    fn from(v: Vertex) -> Self {
        Point3::new(v.x, v.y, v.z)
    }
}

impl PartialOrd for Vertex {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.z != other.z {
            self.z.partial_cmp(&other.z)
        } else if self.y != other.y {
            self.y.partial_cmp(&other.y)
        } else {
            self.x.partial_cmp(&other.x)
        }
    }
}

impl std::ops::Mul<Vertex> for &Transform {
    type Output = Vertex;

    fn mul(self, rhs: Vertex) -> Self::Output {
        Vertex {
            x: self.0[0][0] * rhs.x + self.0[0][1] * rhs.y + self.0[0][2] * rhs.z + self.0[0][3],
            y: self.0[1][0] * rhs.x + self.0[1][1] * rhs.y + self.0[1][2] * rhs.z + self.0[1][3],
            z: self.0[2][0] * rhs.x + self.0[2][1] * rhs.y + self.0[2][2] * rhs.z + self.0[2][3],
        }
    }
}

impl Transform {
    pub fn new_translation_transform(x: f64, y: f64, z: f64) -> Self {
        Transform([
            [1., 0., 0., x],
            [0., 1., 0., y],
            [0., 0., 1., z],
            [0., 0., 0., 1.],
        ])
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

pub struct Object {
    pub layers: Vec<Slice>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum InputObject {
    Raw(String, Transform),
    Auto(String),
    AutoTranslate(String, f64, f64),
}

impl InputObject {
    pub fn get_model_path(&self) -> &str {
        match self {
            InputObject::Raw(str, _) => str,
            InputObject::Auto(str) => str,
            InputObject::AutoTranslate(str, _, _) => str,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Transform(pub [[f64; 4]; 4]);

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct IndexedTriangle {
    pub verts: [usize; 3],
}

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct IndexedLine {
    pub verts: [usize; 2],
}

pub struct Move {
    pub end: Coordinate<f64>,
    pub width: f64,
    pub move_type: MoveType,
}

pub struct MoveChain {
    pub start_point: Coordinate<f64>,
    pub moves: Vec<Move>,
}

#[derive(Serialize, Deserialize,Clone, Copy, Debug, PartialEq)]
pub enum MoveType {
    TopSolidInfill,
    SolidInfill,
    Infill,
    OuterPerimeter,
    InnerPerimeter,
    Bridging,
    Support,
    Travel,
}

#[derive(Serialize, Deserialize,Clone, Debug, PartialEq)]
pub enum Command {
    MoveTo {
        end: Coordinate<f64>,
    },
    MoveAndExtrude {
        start: Coordinate<f64>,
        end: Coordinate<f64>,
        thickness: f64,
        width: f64,
    },
    LayerChange {
        z: f64,
    },
    SetState {
        new_state: StateChange,
    },
    Delay {
        msec: u64,
    },
    Arc {
        start: Coordinate<f64>,
        end: Coordinate<f64>,
        center: Coordinate<f64>,
        clockwise: bool,
        thickness: f64,
        width: f64,
    },
    ChangeObject {
        object: usize,
    },
    //Used in optimization , should be optimized out
    NoAction,
}

#[derive(Serialize, Deserialize, Debug,Clone, Default, PartialEq)]
pub struct StateChange {
    pub extruder_temp: Option<f64>,
    pub bed_temp: Option<f64>,
    pub fan_speed: Option<f64>,
    pub movement_speed: Option<f64>,
    pub acceleration: Option<f64>,
    pub retract: Option<bool>,
}

impl StateChange {
    pub fn state_diff(&mut self, other: &StateChange) -> StateChange {
        StateChange {
            extruder_temp: {
                if self.extruder_temp == other.extruder_temp {
                    None
                } else {
                    self.extruder_temp = other.extruder_temp.or(self.extruder_temp);
                    other.extruder_temp
                }
            },
            bed_temp: {
                if self.bed_temp == other.bed_temp {
                    None
                } else {
                    self.bed_temp = other.bed_temp.or(self.bed_temp);
                    other.bed_temp
                }
            },
            fan_speed: {
                if self.fan_speed == other.fan_speed {
                    None
                } else {
                    self.fan_speed = other.fan_speed.or(self.fan_speed);
                    other.fan_speed
                }
            },
            movement_speed: {
                if self.movement_speed == other.movement_speed {
                    None
                } else {
                    self.movement_speed = other.movement_speed.or(self.movement_speed);
                    other.movement_speed
                }
            },
            acceleration: {
                if self.acceleration == other.acceleration {
                    None
                } else {
                    self.acceleration = other.acceleration.or(self.acceleration);
                    other.acceleration
                }
            },
            retract: {
                if self.retract == other.retract {
                    None
                } else {
                    self.retract = other.retract.or(self.retract);
                    other.retract
                }
            },
        }
    }

    pub fn combine(&self, other: &StateChange) -> StateChange {
        StateChange {
            extruder_temp: { other.extruder_temp.or(self.extruder_temp) },
            bed_temp: { other.bed_temp.or(self.bed_temp) },
            fan_speed: { other.fan_speed.or(self.fan_speed) },
            movement_speed: { other.movement_speed.or(self.movement_speed) },
            acceleration: { other.acceleration.or(self.acceleration) },
            retract: { other.retract.or(self.retract) },
        }
    }
}

impl MoveChain {
    pub fn create_commands(self, settings: &LayerSettings, thickness: f64) -> Vec<Command> {
        let mut cmds = vec![];
        let mut current_type = None;
        let mut current_loc = self.start_point;

        for m in self.moves {
            if Some(m.move_type) != current_type {
                match m.move_type {
                    MoveType::TopSolidInfill => {
                        cmds.push(Command::SetState {
                            new_state: StateChange {
                                bed_temp: None,
                                extruder_temp: None,
                                fan_speed: None,
                                movement_speed: Some(settings.speed.solid_top_infill),
                                acceleration: Some(settings.acceleration.solid_top_infill),
                                retract: Some(false),
                            },
                        });
                    }
                    MoveType::SolidInfill => {
                        cmds.push(Command::SetState {
                            new_state: StateChange {
                                bed_temp: None,
                                extruder_temp: None,
                                fan_speed: None,
                                movement_speed: Some(settings.speed.solid_infill),
                                acceleration: Some(settings.acceleration.solid_infill),
                                retract: Some(false),
                            },
                        });
                    }
                    MoveType::Infill => {
                        cmds.push(Command::SetState {
                            new_state: StateChange {
                                bed_temp: None,
                                extruder_temp: None,
                                fan_speed: None,
                                movement_speed: Some(settings.speed.infill),
                                acceleration: Some(settings.acceleration.infill),
                                retract: Some(false),
                            },
                        });
                    }
                    MoveType::Bridging => {
                        cmds.push(Command::SetState {
                            new_state: StateChange {
                                bed_temp: None,
                                extruder_temp: None,
                                fan_speed: None,
                                movement_speed: Some(settings.speed.bridge),
                                acceleration: Some(settings.acceleration.bridge),
                                retract: Some(false),
                            },
                        });
                    }
                    MoveType::OuterPerimeter => {
                        cmds.push(Command::SetState {
                            new_state: StateChange {
                                bed_temp: None,
                                extruder_temp: None,
                                fan_speed: None,
                                movement_speed: Some(settings.speed.outer_perimeter),
                                acceleration: Some(settings.acceleration.outer_perimeter),
                                retract: Some(false),
                            },
                        });
                    }
                    MoveType::InnerPerimeter => {
                        cmds.push(Command::SetState {
                            new_state: StateChange {
                                bed_temp: None,
                                extruder_temp: None,
                                fan_speed: None,
                                movement_speed: Some(settings.speed.inner_perimeter),
                                acceleration: Some(settings.acceleration.inner_perimeter),
                                retract: Some(false),
                            },
                        });
                    }
                    MoveType::Support => {
                        cmds.push(Command::SetState {
                            new_state: StateChange {
                                bed_temp: None,
                                extruder_temp: None,
                                fan_speed: None,
                                movement_speed: Some(settings.speed.support),
                                acceleration: Some(settings.acceleration.support),
                                retract: Some(false),
                            },
                        });
                    }
                    MoveType::Travel => {
                        cmds.push(Command::SetState {
                            new_state: StateChange {
                                bed_temp: None,
                                extruder_temp: None,
                                fan_speed: None,
                                movement_speed: Some(settings.speed.travel),
                                acceleration: Some(settings.acceleration.travel),
                                retract: Some(true),
                            },
                        });
                    }
                }
                current_type = Some(m.move_type);
            }

            if m.move_type == MoveType::Travel {
                cmds.push(Command::MoveTo { end: m.end });
                current_loc = m.end;
            } else {
                cmds.push(Command::MoveAndExtrude {
                    start: current_loc,
                    end: m.end,
                    thickness,
                    width: m.width,
                });
                current_loc = m.end;
            }
        }

        cmds
    }

    pub fn rotate(&mut self, angle: f64) {
        let cos_a = angle.cos();
        let sin_a = angle.sin();

        for m in self.moves.iter_mut() {
            let nx = m.end.x * cos_a - m.end.y * sin_a;
            let ny = m.end.x * sin_a + m.end.y * cos_a;
            m.end.x = nx;
            m.end.y = ny;
        }
        let nx = self.start_point.x * cos_a - self.start_point.y * sin_a;
        let ny = self.start_point.x * sin_a + self.start_point.y * cos_a;

        self.start_point.x = nx;
        self.start_point.y = ny;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CalculatedValues {
    ///Total plastic used by the print in mm^3
    pub plastic_volume: f64,

    ///Total plastic used by the print in grams
    pub plastic_weight: f64,

    ///Total plastic used by the print in mm of filament
    pub plastic_length: f64,

    ///Total time to print in seconds
    pub total_time: f64,
}


impl CalculatedValues{
    ///Returns total time converted to hours, minutes, seconds, and remaining fractional seconds
    pub fn get_hours_minutes_seconds_fract_time(&self) -> (usize,usize,usize,f64){
        let total_time = self.total_time.floor() as usize;

        let fract = self.total_time - total_time as f64;
        (total_time / 3600, (total_time % 3600) / 60, total_time % 60,fract)
    }
}