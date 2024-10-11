use serde::{Deserialize, Serialize};

///Warnings that can be generated during the slicing process
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SlicerWarnings {
    ///Layer size too low for the nozzle size
    LayerSizeTooLow {
        ///The nozzles Diameter
        nozzle_diameter: f64,
        ///The layer height
        layer_height: f64,
    },

    ///Layer size too low for the nozzle size
    LayerSizeTooHigh {
        ///The nozzles Diameter
        nozzle_diameter: f64,
        ///The layer height
        layer_height: f64,
    },

    ///The acceleration is too low
    AccelerationTooLow {
        ///The acceleration
        acceleration: f64,
        ///The speed
        speed: f64,

        ///The bed size
        bed_size: f64,
    },

    ///Temps to high
    NozzleTemperatureTooHigh {
        ///Temp
        temp: f64,
    },
    ///Temps too low
    NozzleTemperatureTooLow {
        ///Temp
        temp: f64,
    },

    ///The Skirt and Brim over lap
    SkirtAndBrimOverlap {
        ///The skirts distance
        skirt_distance: f64,

        ///The brims width
        brim_width: f64,
    },

    ///Extrusion width too high for the nozzle size
    ExtrusionWidthTooHigh {
        ///The nozzles Diameter
        nozzle_diameter: f64,
        ///The extrusion width
        extrusion_width: f64,
    },

    ///Extrusion width too low for the nozzle size
    ExtrusionWidthTooLow {
        ///The nozzles Diameter
        nozzle_diameter: f64,
        ///The extrusion width
        extrusion_width: f64,
    },
}

impl SlicerWarnings {
    ///Return the error code and pretty error message
    pub fn get_code_and_message(&self) -> (u32, String) {
        match self {
            SlicerWarnings::LayerSizeTooLow { nozzle_diameter, layer_height } => {
                (0x1000, format!("The provided layer height({} mm) is less than 20% of the nozzle diameter({} mm).", layer_height, nozzle_diameter))
            }
            SlicerWarnings::LayerSizeTooHigh { nozzle_diameter, layer_height } => {
                (0x1001, format!("The provided layer height({} mm) is more than 80% of the nozzle diameter({} mm).", layer_height, nozzle_diameter))
            }
            SlicerWarnings::AccelerationTooLow { acceleration, speed, bed_size } => {
                (0x1002, format!("The provided acceleration({} mm/s^2) is low enough that it take more than the length of the bed({} mm) to reach your max speed({} mm/s).", acceleration, bed_size, speed))
            }
            SlicerWarnings::SkirtAndBrimOverlap { skirt_distance, brim_width } => {
                (0x1003, format!("The skirt is too close({} mm) that is over laps with the brim({} mm).", skirt_distance, brim_width))
            }
            SlicerWarnings::NozzleTemperatureTooHigh { temp } => {
                (0x1004, format!("The provided nozzle temperature({} C) is above the safe point of PTFE tubing and close cause issue.", temp))
            }
            SlicerWarnings::NozzleTemperatureTooLow { temp } => {
                (0x1005, format!("The provided nozzle temperature({} C) is below the melting point of most thermoplastics.", temp))
            }
            SlicerWarnings::ExtrusionWidthTooHigh { nozzle_diameter, extrusion_width } => {
                (0x1006, format!("The provided extrusion width({} mm) is more than 200% of the nozzle diameter({} mm).", extrusion_width, nozzle_diameter))
            }
            SlicerWarnings::ExtrusionWidthTooLow { nozzle_diameter, extrusion_width } => {
                (0x1007, format!("The provided extrusion width({} mm) is less than 60% of the nozzle diameter({} mm).", extrusion_width, nozzle_diameter))
            }
        }
    }
}
