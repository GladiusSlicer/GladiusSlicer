#![deny(missing_docs)]

use crate::error::SlicerErrors;
use crate::settings::{LayerSettings, Settings};
use geo::contains::Contains;
use geo::prelude::SimplifyVW;
use geo::simplifyvw::SimplifyVWPreserve;
use geo::*;
use itertools::Itertools;
use nalgebra::Point3;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

///A single slice of an object containing it's current plotting status.
pub struct Slice {
    ///The slice's entire polygon. Should not be modified after creation by the slicing process.
    pub main_polygon: MultiPolygon<f64>,

    ///The slice's remaining area that needs to be processes. Passes will slowly subtract from this until finally infill will fill the space.
    pub remaining_area: MultiPolygon<f64>,

    /// The area that will be filled by support interface material.
    pub support_interface: Option<MultiPolygon<f64>>,

    ///The area that will be filled by support towers
    pub support_tower: Option<MultiPolygon<f64>>,

    ///Theses moves ares applied in order and the start of the commands for the slice.
    pub fixed_chains: Vec<MoveChain>,

    ///The move chains generaated by various passses. These chains can be reordered by the optomization process to create faster commands.
    pub chains: Vec<MoveChain>,

    ///The lower height of this slice.
    pub bottom_height: f64,

    ///The upper height of tis slice.
    pub top_height: f64,

    ///A copy of this layers settings
    pub layer_settings: LayerSettings,
}
impl Slice {
    ///Creates a slice from a spefic iterator of points
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

    ///creates a slice from  a multi line string
    pub fn from_multiple_point_loop(
        lines: MultiLineString<f64>,
        bottom_height: f64,
        top_height: f64,
        layer_count: usize,
        settings: &Settings,
    ) -> Result<Self, SlicerErrors> {
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
                    .ok_or(SlicerErrors::SliceGeneration)?;
                smallest_polygon.interiors_push(line);
            }
        }

        let multi_polygon: MultiPolygon<f64> = MultiPolygon(polygons);

        let layer_settings =
            settings.get_layer_settings(layer_count, (bottom_height + top_height) / 2.0);

        Ok(Slice {
            main_polygon: multi_polygon.clone(),
            remaining_area: multi_polygon.simplifyvw(&0.0001),
            support_interface: None,
            support_tower: None,
            chains: vec![],
            fixed_chains: vec![],
            bottom_height,
            top_height,
            layer_settings,
        })
    }

    ///return the reference height of the slice
    pub fn get_height(&self) -> f64 {
        (self.bottom_height + self.top_height) / 2.0
    }
}

///Types of solid infill
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum SolidInfillsTypes {
    ///Back and forth lines to fill polygons
    Rectilinear,
}

///Types of partial infill
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum PartialInfillTypes {
    ///Back and forth spaced lines to fill polygons
    Linear,

    ///Back and forth spaced lines to fill polygons and there perpendicular lines
    Rectilinear,

    /// Lines in 3 directions to form tessellating triangle pattern
    Triangle,

    /// Creates a 3d cube structure.
    Cubic,

    ///Creates lightning shaped infill that retracts into the print walls
    Lightning,
}

///A single 3D vertex
#[derive(Default, Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename = "vertex")]
pub struct Vertex {
    ///X Coordinate
    pub x: f64,

    ///Y Coordinate
    pub y: f64,

    ///Z Coordinate
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
    ///create a new transform for translation
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

///A object is the collection of slices for a particular model.
pub struct Object {
    /// The slices for this model sorted from lowest to highest.
    pub layers: Vec<Slice>,
}

///The different types of input that the slicer can take.
#[derive(Serialize, Deserialize, Debug)]
pub enum InputObject {
    /// The Raw format that is the file to load and the transform to apply to it.
    Raw(String, Transform),

    ///Automatically Center and raise the model for printing
    Auto(String),

    ///Automatically Center and raise the model for printing but offset it by x and y
    AutoTranslate(String, f64, f64),
}

impl InputObject {
    /// Helper function to get the model path from the input
    pub fn get_model_path(&self) -> &str {
        match self {
            InputObject::Raw(str, _) => str,
            InputObject::Auto(str) => str,
            InputObject::AutoTranslate(str, _, _) => str,
        }
    }
}

///4x4 Matrix used to transform models
#[derive(Serialize, Deserialize, Debug)]
pub struct Transform(pub [[f64; 4]; 4]);

/// A triangle that contains indices to it's 3 points. Used with a Vector of Vertices.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct IndexedTriangle {
    ///Array of the 3 Vertices
    pub verts: [usize; 3],
}

/// A line that contains indices to it's 2 points. Used with a Vector of Vertices.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct IndexedLine {
    ///Array of the 2 Vertices
    pub verts: [usize; 2],
}

///A move of the plotter
pub struct Move {
    ///The end Coordinate of the Move. The start of the move is the previous moves end point.
    pub end: Coordinate<f64>,
    ///The width of plastic to extrude for this move
    pub width: f64,
    ///The type of move
    pub move_type: MoveType,
}

/// A chain of moves that should happen in order
pub struct MoveChain {
    ///start point for the chain of moves. Needed as Moves don't contain there own start point.
    pub start_point: Coordinate<f64>,

    ///List of all moves in order that they must be moved
    pub moves: Vec<Move>,
}

///Types of Moves
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum MoveType {
    ///The top later of infill
    TopSolidInfill,

    ///Solid Infill
    SolidInfill,

    ///Standard Partial infill
    Infill,

    ///The Outer Layer of infill both exterior and holes
    OuterPerimeter,

    ///Inner layers of perimeter
    InnerPerimeter,

    ///A bridge over open air
    Bridging,

    ///Support towers and interface
    Support,

    ///Standard travel moves without extrusion
    Travel,
}

///The intermediate representation of the commands to send to the printer. The commands will be optimized organized and converted into the output expected ( for example GCode)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Command {
    ///Move to a specific location without extrusion
    MoveTo {
        ///The end point of the move
        end: Coordinate<f64>,
    },
    ///Move to a location while extruding plastic
    MoveAndExtrude {
        ///Start point of the move
        start: Coordinate<f64>,

        ///End point of the move
        end: Coordinate<f64>,

        ///The height thickness of the move
        thickness: f64,

        /// The extrusion width
        width: f64,
    },

    ///Change the layer height
    LayerChange {
        ///The height the print head should move to
        z: f64,
    },

    ///Sets the System state to the new values
    SetState {
        ///The new state to change into
        new_state: StateChange,
    },

    ///A fixed duration delay
    Delay {
        ///Number of milliseconds to delay
        msec: u64,
    },

    ///An arc move of the extruder
    Arc {
        ///start point of the arc
        start: Coordinate<f64>,

        ///end point of the arc
        end: Coordinate<f64>,

        ///The center point that the arc keeps equidistant from
        center: Coordinate<f64>,

        ///Whether the arc is clockwise or anticlockwise
        clockwise: bool,

        ///Thickness of the arc, the height
        thickness: f64,

        ///The width of the extrusion
        width: f64,
    },

    ///Change the object that is being printed
    ChangeObject {
        ///The index of the new object being changed to
        object: usize,
    },
    ///Used in optimization , should be optimized out
    NoAction,
}

///A change in the state of the printer. all fields are optional and should only be set when the state is changing.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct StateChange {
    ///The temperature of the current extruder
    pub extruder_temp: Option<f64>,

    ///The temperature of the printing bed
    pub bed_temp: Option<f64>,

    ///The speed of the fan
    pub fan_speed: Option<f64>,

    ///The spped movement commands are performed at
    pub movement_speed: Option<f64>,

    ///The acceleration that movement commands are performed at
    pub acceleration: Option<f64>,

    ///Whether the filament is retracted
    pub retract: Option<bool>,
}

impl StateChange {
    ///Change the current state to the new state and return the differences between the 2 states
    #[must_use]
    pub fn state_diff(&mut self, new_state: &StateChange) -> StateChange {
        StateChange {
            extruder_temp: {
                if self.extruder_temp == new_state.extruder_temp {
                    None
                } else {
                    self.extruder_temp = new_state.extruder_temp.or(self.extruder_temp);
                    new_state.extruder_temp
                }
            },
            bed_temp: {
                if self.bed_temp == new_state.bed_temp {
                    None
                } else {
                    self.bed_temp = new_state.bed_temp.or(self.bed_temp);
                    new_state.bed_temp
                }
            },
            fan_speed: {
                if self.fan_speed == new_state.fan_speed {
                    None
                } else {
                    self.fan_speed = new_state.fan_speed.or(self.fan_speed);
                    new_state.fan_speed
                }
            },
            movement_speed: {
                if self.movement_speed == new_state.movement_speed {
                    None
                } else {
                    self.movement_speed = new_state.movement_speed.or(self.movement_speed);
                    new_state.movement_speed
                }
            },
            acceleration: {
                if self.acceleration == new_state.acceleration {
                    None
                } else {
                    self.acceleration = new_state.acceleration.or(self.acceleration);
                    new_state.acceleration
                }
            },
            retract: {
                if self.retract == new_state.retract {
                    None
                } else {
                    self.retract = new_state.retract.or(self.retract);
                    new_state.retract
                }
            },
        }
    }

    ///combine the 2 state changes into one, prioritizing the new state if both contain a file
    #[must_use]
    pub fn combine(&self, new_state: &StateChange) -> StateChange {
        StateChange {
            extruder_temp: { new_state.extruder_temp.or(self.extruder_temp) },
            bed_temp: { new_state.bed_temp.or(self.bed_temp) },
            fan_speed: { new_state.fan_speed.or(self.fan_speed) },
            movement_speed: { new_state.movement_speed.or(self.movement_speed) },
            acceleration: { new_state.acceleration.or(self.acceleration) },
            retract: { new_state.retract.or(self.retract) },
        }
    }
}

impl MoveChain {
    ///Convert a move chain into a list of commands
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

    ///Rotate all moves in the movechain by a specific angle in radians.
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

///Calculated values about an entire print
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

impl CalculatedValues {
    ///Returns total time converted to hours, minutes, seconds, and remaining fractional seconds
    pub fn get_hours_minutes_seconds_fract_time(&self) -> (usize, usize, usize, f64) {
        let total_time = self.total_time.floor() as usize;

        let fract = self.total_time - total_time as f64;
        (
            total_time / 3600,
            (total_time % 3600) / 60,
            total_time % 60,
            fract,
        )
    }
}
