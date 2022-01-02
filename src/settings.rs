use itertools::Itertools;
use crate::plotter::PartialInfillTypes;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    pub layer_height: f64,
    pub layer_width: f64,

    pub filament: FilamentSettings,
    pub fan: FanSettings,
    pub skirt: Option<SkirtSettings>,

    pub nozzle_diameter: f64,

    pub retract_length: f64,
    pub retract_lift_z: f64,
    pub retract_speed: f64,

    pub speed: MovementParameter,
    //pub first_layer_speed: MovementParameter,
    pub acceleration: MovementParameter,

    pub infill_percentage: f64,

    pub inner_permimeters_first: bool,

    pub number_of_perimeters: usize,

    pub top_layers: usize,
    pub bottom_layers: usize,

    pub print_x: f64,
    pub print_y: f64,
    pub print_z: f64,

    pub brim_width: Option<f64>,

    pub minimum_retract_distance: f64,

    pub infill_perimeter_overlap_percentage: f64,
    pub infill_type: PartialInfillTypes,

    pub starting_gcode: String,
    pub ending_gcode: String,

    pub layer_settings: Vec<(LayerRange,PartialLayerSettings)>
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            layer_height: 0.15,
            number_of_perimeters: 3,
            top_layers: 3,
            bottom_layers: 3,
            layer_width: 0.6,
            filament: FilamentSettings::default(),
            fan: FanSettings::default(),
            skirt: None,
            nozzle_diameter: 0.4,
            retract_length: 0.8,
            retract_lift_z: 0.6,
            retract_speed: 35.0,

            speed: MovementParameter {
                inner_perimeter: 5.0,
                outer_perimeter: 5.0,
                solid_top_infill: 200.0,
                solid_infill: 200.0,
                infill: 200.0,
                travel: 180.0,
                bridge: 30.0,
            },
            acceleration: MovementParameter {
                inner_perimeter: 800.0,
                outer_perimeter: 800.0,
                solid_top_infill: 1000.0,
                solid_infill: 1000.0,
                infill: 1000.0,
                travel: 1000.0,
                bridge: 1000.0,
            },

            infill_percentage: 0.2,

            print_x: 210.0,
            print_y: 210.0,
            print_z: 210.0,
            inner_permimeters_first: true,
            minimum_retract_distance: 1.0,
            infill_perimeter_overlap_percentage: 0.25,
            infill_type: PartialInfillTypes::Linear,
            starting_gcode: "G90 ; use absolute coordinates \n\
                                M83 ; extruder relative mode\n\
                                M106 S255 ; FANNNNN\n\
                                M104 S[First Layer Extruder Temp] ; set extruder temp\n\
                                M140 S[First Layer Bed Temp] ; set bed temp\n\
                                M190 S[First Layer Bed Temp]; wait for bed temp\n\
                                M109 S[First Layer Extruder Temp] ; wait for extruder temp\n\
                                G28 W ; home all without mesh bed level\n\
                                G80 ; mesh bed leveling\n\
                                G1 Y-3.0 F1000.0 ; go outside print area\n\
                                G92 E0.0\n\
                                G1 X60.0 E9.0 F1000.0 ; intro line\n\
                                G1 X100.0 E12.5 F1000.0 ; intro line\n\
                                G92 E0.0;\n"
                .to_string(),
            ending_gcode: "G4 ; wait\n\
                                M221 S100 \n\
                                M104 S0 ; turn off temperature \n\
                                M140 S0 ; turn off heatbed \n\
                                G1 X0 F3000 ; home X axis \n\
                                M84 ; disable motors\n\
                                M107 ; disable fan\n"
                .to_string(),
            brim_width: None,
            layer_settings: vec![
                (LayerRange::SingleLayer(0), PartialLayerSettings{layer_width: Some(0.6),speed:
                    Some( MovementParameter {
                        inner_perimeter: 5.0,
                        outer_perimeter: 5.0,
                        solid_top_infill: 20.0,
                        solid_infill: 20.0,
                        infill: 20.0,
                        travel: 5.0,
                        bridge: 20.0,
                    }) ,
                    layer_height: Some(0.3),
                    .. Default::default()
                })
            ]
        }
    }
}

impl Settings {
    pub fn get_layer_settings(&self, layer: usize, height: f64) -> LayerSettings {

        let changes = self
            .layer_settings
            .iter()
            .filter( |(layer_range,_)| {
                match layer_range {
                    LayerRange::LayerRange {end, start} => *start <= layer&& layer <= *end,
                    LayerRange::HeightRange {end, start} => *start <= height && height <= *end,
                    LayerRange::SingleLayer (filter_layer) => *filter_layer == layer,
                }
            } )
            .map(|(lr,pls)| pls)
            .fold(PartialLayerSettings::default(),|a,b| a.combine(b));

        LayerSettings {
            layer_height: changes.layer_height.unwrap_or(self.layer_height),
            speed: changes.speed.unwrap_or(self.speed.clone()),
            acceleration: changes.acceleration.unwrap_or(self.acceleration.clone()),
            layer_width: changes.layer_width.unwrap_or(self.layer_width),
            infill_type: changes.infill_type.unwrap_or(self.infill_type),
            infill_percentage: changes.infill_percentage.unwrap_or(self.infill_percentage),
            infill_perimeter_overlap_percentage: changes.infill_perimeter_overlap_percentage.unwrap_or(self.infill_perimeter_overlap_percentage),
            inner_permimeters_first: changes.inner_permimeters_first.unwrap_or(self.inner_permimeters_first),
        }
    }
}

pub struct LayerSettings {
    pub layer_height: f64,

    pub speed: MovementParameter,
    pub acceleration: MovementParameter,

    pub layer_width: f64,

    pub infill_type: PartialInfillTypes,
    pub infill_percentage: f64,
    pub infill_perimeter_overlap_percentage: f64,
    pub inner_permimeters_first: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MovementParameter {
    pub inner_perimeter: f64,
    pub outer_perimeter: f64,
    pub solid_top_infill: f64,
    pub solid_infill: f64,
    pub infill: f64,
    pub travel: f64,
    pub bridge: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FilamentSettings {
    pub diameter: f64,
    pub density: f64,
    pub cost: f64,
    pub extruder_temp: f64,
    pub first_layer_extruder_temp: f64,
    pub bed_temp: f64,
    pub first_layer_bed_temp: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FanSettings {
    pub fan_speed: f64,
    pub disable_fan_for_layers: usize,
    pub slow_down_threshold: f64,
    pub min_print_speed: f64,
}

impl Default for FilamentSettings {
    fn default() -> Self {
        FilamentSettings {
            diameter: 1.75,
            density: 1.24,
            cost: 24.99,
            extruder_temp: 210.0,
            first_layer_extruder_temp: 215.0,
            bed_temp: 60.0,
            first_layer_bed_temp: 60.0,
        }
    }
}

impl Default for FanSettings {
    fn default() -> Self {
        FanSettings {
            fan_speed: 100.0,
            disable_fan_for_layers: 1,
            slow_down_threshold: 15.0,
            min_print_speed: 15.0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SkirtSettings {
    pub layers: usize,
    pub distance: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PartialSettings {
    pub layer_height: Option<f64>,
    pub layer_width: Option<f64>,

    pub filament: Option<FilamentSettings>,
    pub fan: Option<FanSettings>,
    pub skirt: Option<SkirtSettings>,

    pub nozzle_diameter: Option<f64>,

    pub retract_length: Option<f64>,
    pub retract_lift_z: Option<f64>,
    pub retract_speed: Option<f64>,

    pub speed: Option<MovementParameter>,
    pub first_layer_speed: Option<MovementParameter>,
    pub acceleration: Option<MovementParameter>,

    pub infill_percentage: Option<f64>,

    pub first_layer_height: Option<f64>,
    pub first_layer_width: Option<f64>,

    pub inner_permimeters_first: Option<bool>,

    pub number_of_perimeters: Option<usize>,

    pub top_layers: Option<usize>,
    pub bottom_layers: Option<usize>,

    pub print_x: Option<f64>,
    pub print_y: Option<f64>,
    pub print_z: Option<f64>,

    pub brim_width: Option<f64>,

    pub minimum_retract_distance: Option<f64>,

    pub infill_perimeter_overlap_percentage: Option<f64>,
    pub infill_type: Option<PartialInfillTypes>,

    pub starting_gcode: Option<String>,
    pub ending_gcode: Option<String>,

    pub other_files: Option<Vec<String>>,

    pub layer_settings: Option<Vec<(LayerRange,PartialLayerSettings)>>
}

impl PartialSettings {
    pub fn get_settings(mut self) -> Result<Settings, String> {
        self.combine_with_other_files();

        let settings = Settings {
            layer_height: self.layer_height.ok_or("layer_height")?,
            layer_width: self.layer_width.ok_or("layer_width")?,
            filament: self.filament.ok_or("filament")?,
            fan: self.fan.ok_or("fan")?,
            skirt: self.skirt,
            nozzle_diameter: self.nozzle_diameter.ok_or("nozzle_diameter")?,
            retract_length: self.retract_length.ok_or("retract_length")?,
            retract_lift_z: self.retract_lift_z.ok_or("retract_lift_z")?,
            retract_speed: self.retract_speed.ok_or("retract_speed")?,
            speed: self.speed.ok_or("speed")?,
            acceleration: self.acceleration.ok_or("acceleration")?,
            infill_percentage: self.infill_percentage.ok_or("infill_percentage")?,
            inner_permimeters_first: self
                .inner_permimeters_first
                .ok_or("inner_permimeters_first")?,
            number_of_perimeters: self.number_of_perimeters.ok_or("number_of_perimeters")?,
            top_layers: self.top_layers.ok_or("top_layers")?,
            bottom_layers: self.bottom_layers.ok_or("bottom_layers")?,
            print_x: self.print_x.ok_or("print_x")?,
            print_y: self.print_y.ok_or("print_y")?,
            print_z: self.print_z.ok_or("print_z")?,
            brim_width: self.brim_width,
            minimum_retract_distance: self
                .minimum_retract_distance
                .ok_or("minimum_retract_distance")?,
            infill_perimeter_overlap_percentage: self
                .infill_perimeter_overlap_percentage
                .ok_or("infill_perimeter_overlap_percentage")?,
            infill_type: self.infill_type.ok_or("infill_type")?,
            starting_gcode: self.starting_gcode.ok_or("starting_gcode")?,
            ending_gcode: self.ending_gcode.ok_or("ending_gcode")?,

            layer_settings: self.layer_settings.unwrap_or(vec![])
        };

        Ok(settings)
    }

    fn combine_with_other_files(&mut self) {
        let files: Vec<String> = self
            .other_files
            .as_mut()
            .map(|of| of.drain(..).collect())
            .unwrap_or_default();

        for file in files {
            println!("file {}", file);
            let mut ps: PartialSettings =
                deser_hjson::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();

            ps.combine_with_other_files();

            *self = self.combine(ps);
        }
    }

    fn combine(&self, other: PartialSettings) -> PartialSettings {
        PartialSettings {
            layer_height: self.layer_height.or(other.layer_height),
            layer_width: self.layer_width.or(other.layer_width),
            filament: self.filament.clone().or_else(|| other.filament.clone()),
            fan: self.fan.clone().or_else(|| other.fan.clone()),
            skirt: self.skirt.clone().or_else(|| other.skirt.clone()),
            nozzle_diameter: self.nozzle_diameter.or(other.nozzle_diameter),
            retract_length: self.retract_length.or(other.retract_length),
            retract_lift_z: self.retract_lift_z.or(other.retract_lift_z),
            retract_speed: self.retract_speed.or(other.retract_speed),
            speed: self.speed.clone().or_else(|| other.speed.clone()),
            first_layer_speed: self
                .first_layer_speed
                .clone()
                .or_else(|| other.first_layer_speed.clone()),
            acceleration: self
                .acceleration
                .clone()
                .or_else(|| other.acceleration.clone()),
            infill_percentage: self.infill_percentage.or(other.infill_percentage),
            first_layer_height: self.first_layer_height.or(other.first_layer_height),
            first_layer_width: self.first_layer_width.or(other.first_layer_width),
            inner_permimeters_first: self
                .inner_permimeters_first
                .or(other.inner_permimeters_first),
            number_of_perimeters: self.number_of_perimeters.or(other.number_of_perimeters),
            top_layers: self.top_layers.or(other.top_layers),
            bottom_layers: self.bottom_layers.or(other.bottom_layers),
            print_x: self.print_x.or(other.print_x),
            print_y: self.print_y.or(other.print_y),
            print_z: self.print_z.or(other.print_z),
            brim_width: self.brim_width.or(other.brim_width),
            minimum_retract_distance: self
                .minimum_retract_distance
                .or(other.minimum_retract_distance),
            infill_perimeter_overlap_percentage: self
                .infill_perimeter_overlap_percentage
                .or(other.infill_perimeter_overlap_percentage),
            infill_type: self.infill_type.or(other.infill_type),
            starting_gcode: self
                .starting_gcode
                .clone()
                .or_else(|| other.starting_gcode.clone()),
            ending_gcode: self.ending_gcode.clone().or(other.ending_gcode),
            other_files: None,
            layer_settings: {
                match (self.layer_settings.as_ref(),other.layer_settings.as_ref()) {
                    (None , None) => None,
                    (None , Some(v)) | (Some(v) , None)=> Some(v.clone()),
                    (Some(a) , Some(b)) => {
                        let mut v = vec![];
                        v.append(&mut a.clone());
                        v.append(&mut b.clone());
                        Some(v)
                    },
                }
            }
        }
    }
}

#[derive(Deserialize,Serialize, Debug,Clone)]
pub enum LayerRange{
    SingleLayer(usize),
    LayerRange{start: usize, end: usize},
    HeightRange{start: f64, end: f64},
}

#[derive(Serialize, Deserialize, Debug,Default,Clone)]
pub struct PartialLayerSettings {
    pub layer_height: Option<f64>,

    pub speed: Option<MovementParameter>,
    pub acceleration: Option<MovementParameter>,

    pub layer_width: Option<f64>,

    pub infill_type: Option<PartialInfillTypes>,
    pub infill_percentage: Option<f64>,
    pub infill_perimeter_overlap_percentage: Option<f64>,
    pub inner_permimeters_first: Option<bool>,
}

impl PartialLayerSettings{
    fn combine(&self, other: &PartialLayerSettings) -> PartialLayerSettings {
        PartialLayerSettings {
            layer_height: self.layer_height.or(other.layer_height),
            layer_width: self.layer_width.or(other.layer_width),
            speed: self.speed.clone().or_else(|| other.speed.clone()),
            acceleration: self
                .acceleration
                .clone()
                .or_else(|| other.acceleration.clone()),
            infill_percentage: self.infill_percentage.or(other.infill_percentage),

            inner_permimeters_first: self
                .inner_permimeters_first
                .or(other.inner_permimeters_first),

            infill_perimeter_overlap_percentage: self
                .infill_perimeter_overlap_percentage
                .or(other.infill_perimeter_overlap_percentage),
            infill_type: self.infill_type.or(other.infill_type),

        }
    }
}