use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings{
    pub layer_height: f64,
    pub layer_width: f64,

    pub filament: FilamentSettings,

    pub nozzle_diameter: f64,

    pub retract_length: f64,
    pub retract_lift_z: f64,
    pub retract_speed: f64,

    pub perimeter_speed: f64,
    pub infill_speed:  f64,
    pub travel_speed: f64,
    
    pub starting_gcode: String,
    pub ending_gcode: String,
}

impl Default for Settings{
    fn default() -> Self {
        Settings{
            layer_height: 0.1,
            layer_width: 0.6,
            filament: FilamentSettings::default(),

            nozzle_diameter: 0.4,
            retract_length: 0.8,
            retract_lift_z: 0.6,
            retract_speed: 35.0,

            perimeter_speed: 5.0,
            infill_speed: 200.0,
            travel_speed: 180.0,
            starting_gcode:     "G90 ; use absolute coordinates \n\
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
                                G92 E0.0;\n".to_string(),
            ending_gcode:     "G1 X0 F3000 ; home X axis ; disable motors\n\
                                M84 ; disable motors\n\
                                M107 ; disable fan\n".to_string()
        }
    }
}


#[derive(Serialize, Deserialize, Debug)]
pub struct FilamentSettings{
    pub diameter: f64,
    pub density: f64,
    pub cost: f64,
    pub extruder_temp: f64,
    pub bed_temp: f64
}

impl Default for FilamentSettings{
    fn default() -> Self {
        FilamentSettings{
            diameter: 1.75,
            density: 1.24,
            cost: 24.99,
            extruder_temp: 215.0,
            bed_temp: 60.0
        }
    }
}

