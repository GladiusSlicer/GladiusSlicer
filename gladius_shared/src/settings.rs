#![deny(missing_docs)]

use crate::error::SlicerErrors;
use crate::types::PartialInfillTypes;
use serde::{Deserialize, Serialize};

///A complete settings file for the entire slicer.
#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    ///The height of the layers
    pub layer_height: f64,

    ///The extrusion width of the layers
    pub layer_width: f64,

    ///The filament Settings
    pub filament: FilamentSettings,

    ///The fan settings
    pub fan: FanSettings,

    ///The skirt settings, if None no skirt will be generated
    pub skirt: Option<SkirtSettings>,

    ///The support settings, if None no support will be generated
    pub support: Option<SupportSettings>,

    ///Diameter of the nozzle in mm
    pub nozzle_diameter: f64,

    ///length to retract in mm
    pub retract_length: f64,

    ///Distance to lift the z axis during a retract
    pub retract_lift_z: f64,

    ///The velocity of retracts
    pub retract_speed: f64,

    ///The speeds used for movement
    pub speed: MovementParameter,

    ///The acceleration for movement
    pub acceleration: MovementParameter,

    ///The percentage of infill to use for partial infill
    pub infill_percentage: f64,

    ///Controls the order of perimeters
    pub inner_perimeters_first: bool,

    ///Number of perimeters to use if possible
    pub number_of_perimeters: usize,

    ///Number of solid top layers for infill
    pub top_layers: usize,

    ///Number of solid bottom layers before infill
    pub bottom_layers: usize,

    ///Size of the printer in x dimension in mm
    pub print_x: f64,

    ///Size of the printer in y dimension in mm
    pub print_y: f64,

    ///Size of the printer in z dimension in mm
    pub print_z: f64,

    ///Width of the brim, if None no brim will be generated
    pub brim_width: Option<f64>,

    ///Inset the layer by the provided amount, if None on inset will be performed
    pub layer_shrink_amount: Option<f64>,

    ///The minimum travel distance required to perform a retraction
    pub minimum_retract_distance: f64,

    ///Overlap between infill and interior perimeters
    pub infill_perimeter_overlap_percentage: f64,

    ///Partial Infill type
    pub partial_infill_type: PartialInfillTypes,

    ///The instructions to prepend to the exported instructions
    pub starting_instructions: String,

    ///The instructions to append to the end of the exported instructions
    pub ending_instructions: String,

    ///Settings for specific layers
    pub layer_settings: Vec<(LayerRange, PartialLayerSettings)>,
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

            support: None,

            speed: MovementParameter {
                inner_perimeter: 5.0,
                outer_perimeter: 5.0,
                solid_top_infill: 200.0,
                solid_infill: 200.0,
                infill: 200.0,
                travel: 180.0,
                bridge: 30.0,
                support: 50.0,
            },
            acceleration: MovementParameter {
                inner_perimeter: 800.0,
                outer_perimeter: 800.0,
                solid_top_infill: 1000.0,
                solid_infill: 1000.0,
                infill: 1000.0,
                travel: 1000.0,
                bridge: 1000.0,
                support: 1000.0,
            },

            infill_percentage: 0.2,

            print_x: 210.0,
            print_y: 210.0,
            print_z: 210.0,
            inner_perimeters_first: true,
            minimum_retract_distance: 1.0,
            infill_perimeter_overlap_percentage: 0.25,
            partial_infill_type: PartialInfillTypes::Linear,
            starting_instructions: "G90 ; use absolute coordinates \n\
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
            ending_instructions: "G4 ; wait\n\
                                M221 S100 \n\
                                M104 S0 ; turn off temperature \n\
                                M140 S0 ; turn off heatbed \n\
                                G1 X0 F3000 ; home X axis \n\
                                M84 ; disable motors\n\
                                M107 ; disable fan\n"
                .to_string(),
            brim_width: None,
            layer_settings: vec![(
                LayerRange::SingleLayer(0),
                PartialLayerSettings {
                    layer_width: Some(0.6),
                    speed: Some(MovementParameter {
                        inner_perimeter: 5.0,
                        outer_perimeter: 5.0,
                        solid_top_infill: 20.0,
                        solid_infill: 20.0,
                        infill: 20.0,
                        travel: 5.0,
                        bridge: 20.0,
                        support: 20.0,
                    }),
                    layer_height: Some(0.3),
                    bed_temp: Some(60.0),
                    extruder_temp: Some(210.0),
                    ..Default::default()
                },
            )],
            layer_shrink_amount: None,
        }
    }
}

impl Settings {
    ///Get the layer settings for a specific layer index and height
    pub fn get_layer_settings(&self, layer: usize, height: f64) -> LayerSettings {
        let changes = self
            .layer_settings
            .iter()
            .filter(|(layer_range, _)| match layer_range {
                LayerRange::LayerCountRange { end, start } => *start <= layer && layer <= *end,
                LayerRange::HeightRange { end, start } => *start <= height && height <= *end,
                LayerRange::SingleLayer(filter_layer) => *filter_layer == layer,
            })
            .map(|(_lr, pls)| pls)
            .fold(PartialLayerSettings::default(), |a, b| a.combine(b));

        LayerSettings {
            layer_height: changes.layer_height.unwrap_or(self.layer_height),
            layer_shrink_amount: changes.layer_shrink_amount.or(self.layer_shrink_amount),
            speed: changes.speed.unwrap_or_else(|| self.speed.clone()),
            acceleration: changes
                .acceleration
                .unwrap_or_else(|| self.acceleration.clone()),
            layer_width: changes.layer_width.unwrap_or(self.layer_width),
            partial_infill_type: changes
                .partial_infill_type
                .unwrap_or(self.partial_infill_type),
            infill_percentage: changes.infill_percentage.unwrap_or(self.infill_percentage),
            infill_perimeter_overlap_percentage: changes
                .infill_perimeter_overlap_percentage
                .unwrap_or(self.infill_perimeter_overlap_percentage),
            inner_perimeters_first: changes
                .inner_perimeters_first
                .unwrap_or(self.inner_perimeters_first),
            bed_temp: changes.bed_temp.unwrap_or(self.filament.bed_temp),
            extruder_temp: changes.extruder_temp.unwrap_or(self.filament.extruder_temp),
        }
    }
}

///Settings specific to a Layer
pub struct LayerSettings {
    ///The height of the layers
    pub layer_height: f64,

    ///Inset the layer by the provided amount, if None on inset will be performed
    pub layer_shrink_amount: Option<f64>,

    ///The speeds used for movement
    pub speed: MovementParameter,

    ///The acceleration for movement
    pub acceleration: MovementParameter,

    ///The extrusion width of the layers
    pub layer_width: f64,

    ///Partial Infill type
    pub partial_infill_type: PartialInfillTypes,

    ///The percentage of infill to use for partial infill
    pub infill_percentage: f64,

    ///Overlap between infill and interior perimeters
    pub infill_perimeter_overlap_percentage: f64,

    ///Controls the order of perimeters
    pub inner_perimeters_first: bool,

    ///Temperature of the bed
    pub bed_temp: f64,

    ///Temperature of the extuder
    pub extruder_temp: f64,
}

///A set of values for different movement types
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MovementParameter {
    ///Value fpr interior perimeter moves
    pub inner_perimeter: f64,

    ///Value for outer perimeter move
    pub outer_perimeter: f64,

    ///Value for solid top infill moves
    pub solid_top_infill: f64,

    ///Value for solid infill moves
    pub solid_infill: f64,

    ///Value for pertial infill moves
    pub infill: f64,

    ///Value for travel moves
    pub travel: f64,

    ///Value for bridging
    pub bridge: f64,

    ///Value for support structures
    pub support: f64,
}

///Settings for a filament
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FilamentSettings {
    ///Diameter of this filament in mm
    pub diameter: f64,

    ///Density of this filament in grams per cm^3
    pub density: f64,

    ///Cost of this filament in $ per kg
    pub cost: f64,

    ///Extruder temp for this filament
    pub extruder_temp: f64,

    ///Bed temp for this filament
    pub bed_temp: f64,
}

///Settigns for the fans
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FanSettings {
    ///The default fan speed
    pub fan_speed: f64,

    ///Disable the fan for layers below this value
    pub disable_fan_for_layers: usize,

    ///Threshold to start slowing down based on layer print time in seconds
    pub slow_down_threshold: f64,

    ///Minimum speed to slow down to
    pub min_print_speed: f64,
}

impl Default for FilamentSettings {
    fn default() -> Self {
        FilamentSettings {
            diameter: 1.75,
            density: 1.24,
            cost: 24.99,
            extruder_temp: 210.0,
            bed_temp: 60.0,
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

///Support settings
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SupportSettings {
    ///Angle to start production supports in degrees
    pub max_overhang_angle: f64,

    ///Spacing between the ribs of support
    pub support_spacing: f64,
}

///The Settings for Skirt generation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SkirtSettings {
    ///the number of layer to generate the skirt
    pub layers: usize,

    ///Distance from the models to place the skirt
    pub distance: f64,
}

///A partial complete settings file
#[derive(Serialize, Deserialize, Debug)]
pub struct PartialSettings {
    ///The height of the layers
    pub layer_height: Option<f64>,

    ///The extrusion width of the layers
    pub layer_width: Option<f64>,

    ///Inset the layer by the provided amount, if None on inset will be performed
    pub layer_shrink_amount: Option<f64>,
    ///The filament Settings
    pub filament: Option<FilamentSettings>,
    ///The fan settings
    pub fan: Option<FanSettings>,
    ///The skirt settings, if None no skirt will be generated
    pub skirt: Option<SkirtSettings>,
    ///The support settings, if None no support will be generated
    pub support: Option<SupportSettings>,
    ///Diameter of the nozzle in mm
    pub nozzle_diameter: Option<f64>,

    ///length to retract in mm
    pub retract_length: Option<f64>,
    ///Distance to lift the z axis during a retract
    pub retract_lift_z: Option<f64>,

    ///The velocity of retracts
    pub retract_speed: Option<f64>,

    ///The speeds used for movement
    pub speed: Option<MovementParameter>,

    ///The acceleration for movement
    pub acceleration: Option<MovementParameter>,

    ///The percentage of infill to use for partial infill
    pub infill_percentage: Option<f64>,

    ///Controls the order of perimeters
    pub inner_perimeters_first: Option<bool>,

    ///Number of perimeters to use if possible
    pub number_of_perimeters: Option<usize>,

    ///Number of solid top layers before infill
    pub top_layers: Option<usize>,

    ///Number of solid bottom layers before infill
    pub bottom_layers: Option<usize>,

    ///Size of the printer in x dimension in mm
    pub print_x: Option<f64>,

    ///Size of the printer in y dimension in mm
    pub print_y: Option<f64>,

    ///Size of the printer in z dimension in mm
    pub print_z: Option<f64>,

    ///Width of the brim, if None no brim will be generated
    pub brim_width: Option<f64>,

    ///The minimum travel distance required to perform a retraction
    pub minimum_retract_distance: Option<f64>,

    ///Overlap between infill and interior perimeters
    pub infill_perimeter_overlap_percentage: Option<f64>,

    ///Partial Infill type
    pub partial_infill_type: Option<PartialInfillTypes>,

    ///The instructions to prepend to the exported instructions
    pub starting_instructions: Option<String>,

    ///The instructions to append to the end of the exported instructions
    pub ending_instructions: Option<String>,

    ///Other files to load
    pub other_files: Option<Vec<String>>,

    ///Settings for specific layers
    pub layer_settings: Option<Vec<(LayerRange, PartialLayerSettings)>>,
}

impl PartialSettings {
    ///Convert a partial settings file into a complete settings file
    /// returns an error if a settings is not present in this or any sub file
    pub fn get_settings(mut self) -> Result<Settings, SlicerErrors> {
        self.combine_with_other_files()?;

        try_convert_partial_to_settings(self).map_err(|err| {
            SlicerErrors::SettingsFileMissingSettings {
                missing_setting: err,
            }
        })
    }

    fn combine_with_other_files(&mut self) -> Result<(), SlicerErrors> {
        let files: Vec<String> = self
            .other_files
            .as_mut()
            .map(|of| of.drain(..).collect())
            .unwrap_or_default();

        for file in &files {
            let mut ps: PartialSettings =
                deser_hjson::from_str(&std::fs::read_to_string(file).map_err(|_| {
                    SlicerErrors::SettingsRecursiveLoadError {
                        filepath: file.to_string(),
                    }
                })?)
                .map_err(|_| SlicerErrors::SettingsFileMisformat {
                    filepath: file.to_string(),
                })?;

            ps.combine_with_other_files()?;

            *self = self.combine(ps);
        }

        Ok(())
    }

    fn combine(&self, other: PartialSettings) -> PartialSettings {
        PartialSettings {
            layer_height: self.layer_height.or(other.layer_height),
            layer_width: self.layer_width.or(other.layer_width),
            layer_shrink_amount: self.layer_shrink_amount.or(other.layer_shrink_amount),
            filament: self.filament.clone().or_else(|| other.filament.clone()),
            fan: self.fan.clone().or_else(|| other.fan.clone()),
            skirt: self.skirt.clone().or_else(|| other.skirt.clone()),
            support: self.support.clone().or_else(|| other.support.clone()),
            nozzle_diameter: self.nozzle_diameter.or(other.nozzle_diameter),
            retract_length: self.retract_length.or(other.retract_length),
            retract_lift_z: self.retract_lift_z.or(other.retract_lift_z),
            retract_speed: self.retract_speed.or(other.retract_speed),
            speed: self.speed.clone().or_else(|| other.speed.clone()),
            acceleration: self
                .acceleration
                .clone()
                .or_else(|| other.acceleration.clone()),
            infill_percentage: self.infill_percentage.or(other.infill_percentage),
            inner_perimeters_first: self.inner_perimeters_first.or(other.inner_perimeters_first),
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
            partial_infill_type: self.partial_infill_type.or(other.partial_infill_type),
            starting_instructions: self
                .starting_instructions
                .clone()
                .or_else(|| other.starting_instructions.clone()),
            ending_instructions: self
                .ending_instructions
                .clone()
                .or(other.ending_instructions),
            other_files: None,
            layer_settings: {
                match (self.layer_settings.as_ref(), other.layer_settings.as_ref()) {
                    (None, None) => None,
                    (None, Some(v)) | (Some(v), None) => Some(v.clone()),
                    (Some(a), Some(b)) => {
                        let mut v = vec![];
                        v.append(&mut a.clone());
                        v.append(&mut b.clone());
                        Some(v)
                    }
                }
            },
        }
    }
}

/// The different types of layer ranges supported
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum LayerRange {
    ///A single single based on the index
    SingleLayer(usize),

    ///A range of layers based on index inclusive
    LayerCountRange {
        ///The start index
        start: usize,

        ///The end index
        end: usize,
    },

    ///A Range of layers based on the height of the bottom on the slice
    HeightRange {
        ///The start height
        start: f64,

        ///The end height
        end: f64,
    },
}

///A Partial List of all slicer settings
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct PartialLayerSettings {
    ///The height of the layers
    pub layer_height: Option<f64>,

    ///Inset the layer by the provided amount, if None on inset will be performed
    pub layer_shrink_amount: Option<f64>,

    ///The speeds used for movement
    pub speed: Option<MovementParameter>,

    ///The acceleration for movement
    pub acceleration: Option<MovementParameter>,

    ///The extrusion width of the layers
    pub layer_width: Option<f64>,

    ///Partial Infill type
    pub partial_infill_type: Option<PartialInfillTypes>,

    ///The percentage of infill to use for partial infill
    pub infill_percentage: Option<f64>,

    ///Overlap between infill and interior perimeters
    pub infill_perimeter_overlap_percentage: Option<f64>,

    ///Controls the order of perimeters
    pub inner_perimeters_first: Option<bool>,

    ///The Bed Temperature
    pub bed_temp: Option<f64>,

    ///The Extruder Temperature
    pub extruder_temp: Option<f64>,
}

impl PartialLayerSettings {
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

            inner_perimeters_first: self.inner_perimeters_first.or(other.inner_perimeters_first),

            bed_temp: self.bed_temp.or(other.bed_temp),
            extruder_temp: self.extruder_temp.or(other.extruder_temp),
            infill_perimeter_overlap_percentage: self
                .infill_perimeter_overlap_percentage
                .or(other.infill_perimeter_overlap_percentage),
            partial_infill_type: self.partial_infill_type.or(other.partial_infill_type),
            layer_shrink_amount: self.layer_shrink_amount.or(other.layer_shrink_amount),
        }
    }
}

fn try_convert_partial_to_settings(part: PartialSettings) -> Result<Settings, String> {
    Ok(Settings {
        layer_height: part.layer_height.ok_or("layer_height")?,
        layer_width: part.layer_width.ok_or("layer_width")?,
        filament: part.filament.ok_or("filament")?,
        fan: part.fan.ok_or("fan")?,
        skirt: part.skirt,
        support: part.support,
        nozzle_diameter: part.nozzle_diameter.ok_or("nozzle_diameter")?,
        retract_length: part.retract_length.ok_or("retract_length")?,
        retract_lift_z: part.retract_lift_z.ok_or("retract_lift_z")?,
        retract_speed: part.retract_speed.ok_or("retract_speed")?,
        speed: part.speed.ok_or("speed")?,
        acceleration: part.acceleration.ok_or("acceleration")?,
        infill_percentage: part.infill_percentage.ok_or("infill_percentage")?,
        inner_perimeters_first: part
            .inner_perimeters_first
            .ok_or("inner_perimeters_first")?,
        number_of_perimeters: part.number_of_perimeters.ok_or("number_of_perimeters")?,
        top_layers: part.top_layers.ok_or("top_layers")?,
        bottom_layers: part.bottom_layers.ok_or("bottom_layers")?,
        print_x: part.print_x.ok_or("print_x")?,
        print_y: part.print_y.ok_or("print_y")?,
        print_z: part.print_z.ok_or("print_z")?,
        brim_width: part.brim_width,
        layer_shrink_amount: part.layer_shrink_amount,
        minimum_retract_distance: part
            .minimum_retract_distance
            .ok_or("minimum_retract_distance")?,
        infill_perimeter_overlap_percentage: part
            .infill_perimeter_overlap_percentage
            .ok_or("infill_perimeter_overlap_percentage")?,
        partial_infill_type: part.partial_infill_type.ok_or("partial_infill_type")?,
        starting_instructions: part.starting_instructions.ok_or("starting_instructions")?,
        ending_instructions: part.ending_instructions.ok_or("ending_instructions")?,

        layer_settings: part.layer_settings.unwrap_or_default(),
    })
}
