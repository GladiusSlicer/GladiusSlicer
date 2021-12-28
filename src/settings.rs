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
    pub first_layer_speed: MovementParameter,
    pub acceleration: MovementParameter,

    pub infill_percentage: f64,


    pub first_layer_height: f64,
    pub first_layer_width: f64,

    pub inner_permimeters_first: bool,

    pub number_of_perimeters: usize,

    pub top_layers: usize,
    pub bottom_layers: usize,

    pub print_x: f64,
    pub print_y: f64,
    pub print_z: f64,

    pub minimum_retract_distance: f64,

    pub infill_perimeter_overlap_percentage: f64,
    pub infill_type: PartialInfillTypes,

    pub starting_gcode: String,
    pub ending_gcode: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            layer_height: 0.15,
            first_layer_height: 0.3,
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

            speed: MovementParameter{
                inner_perimeter: 5.0,
                outer_perimeter: 5.0,
                solid_top_infill: 200.0,
                solid_infill: 200.0,
                infill: 200.0,
                travel: 180.0,
                bridge: 30.0
            },
            first_layer_speed: MovementParameter{
                inner_perimeter: 5.0,
                outer_perimeter: 5.0,
                solid_top_infill: 20.0,
                solid_infill: 20.0,
                infill: 20.0,
                travel: 5.0,
                bridge: 20.0
            },
            acceleration: MovementParameter{
                inner_perimeter: 800.0,
                outer_perimeter: 800.0,
                solid_top_infill: 1000.0,
                solid_infill: 1000.0,
                infill: 1000.0,
                travel: 1000.0,
                bridge: 1000.0
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
            first_layer_width: 0.6,
        }
    }
}

impl Settings {
    pub fn get_layer_settings(&self, layer: usize) -> LayerSettings {
        if layer == 0 {
            LayerSettings {
                layer_height: self.first_layer_height,
                speed: self.first_layer_speed.clone(),
                acceleration: self.acceleration.clone(),
                layer_width: self.first_layer_width,
                infill_type: self.infill_type,
                infill_percentage: self.infill_percentage,
                infill_perimeter_overlap_percentage: self.infill_perimeter_overlap_percentage,
                inner_permimeters_first: self.inner_permimeters_first,
            }
        } else {
            LayerSettings {
                layer_height: self.layer_height,
                speed: self.speed.clone(),
                acceleration: self.acceleration.clone(),
                layer_width: self.layer_width,
                infill_type: self.infill_type,
                infill_percentage: self.infill_percentage,
                infill_perimeter_overlap_percentage: self.infill_perimeter_overlap_percentage,
                inner_permimeters_first: self.inner_permimeters_first,
            }
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

#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct MovementParameter {
    pub inner_perimeter: f64,
    pub outer_perimeter: f64,
    pub solid_top_infill: f64,
    pub solid_infill: f64,
    pub infill: f64,
    pub travel: f64,
    pub bridge: f64,


}


#[derive(Serialize, Deserialize, Debug)]
pub struct FilamentSettings {
    pub diameter: f64,
    pub density: f64,
    pub cost: f64,
    pub extruder_temp: f64,
    pub first_layer_extruder_temp: f64,
    pub bed_temp: f64,
    pub first_layer_bed_temp: f64,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct SkirtSettings {
    pub layers: usize,
    pub distance: f64,
}
