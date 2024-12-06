#![deny(missing_docs)]

use std::fmt::Display;
use std::path::PathBuf;

use crate::error::SlicerErrors;
use crate::types::{MoveType, PartialInfillTypes, SolidInfillTypes};
use crate::warning::SlicerWarnings;
// for exclude area setting
use geo::MultiPolygon;
use geo_validity_check::Valid;
use gladius_proc_macros::Settings;
use log::trace;
#[cfg(feature = "json_schema_gen")]
/// json schema gen
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

macro_rules! setting_less_than_or_equal_to_zero {
    ($settings:ident,$setting:ident) => {{
        if $settings.$setting as f64 <= 0.0 {
            return SettingsValidationResult::Error(SlicerErrors::SettingLessThanOrEqualToZero {
                setting: stringify!($setting).to_string(),
                value: $settings.$setting as f64,
            });
        }
    }};
}

macro_rules! option_setting_less_than_or_equal_to_zero {
    ($settings:ident,$setting:ident) => {{
        if let Some(temp) = $settings.$setting {
            if (temp as f64) <= 0.0 {
                return SettingsValidationResult::Error(
                    SlicerErrors::SettingLessThanOrEqualToZero {
                        setting: stringify!($setting).to_string(),
                        value: temp as f64,
                    },
                );
            }
        }
    }};
}

macro_rules! setting_less_than_zero {
    ($settings:ident,$setting:ident) => {{
        if ($settings.$setting as f64) < 0.0 {
            return SettingsValidationResult::Error(SlicerErrors::SettingLessThanZero {
                setting: stringify!($setting).to_string(),
                value: $settings.$setting as f64,
            });
        }
    }};
}

macro_rules! option_setting_less_than_zero {
    ($settings:ident,$setting:ident) => {{
        if let Some(temp) = $settings.$setting {
            if (temp as f64) < 0.0 {
                return SettingsValidationResult::Error(
                    SlicerErrors::SettingLessThanOrEqualToZero {
                        setting: stringify!($setting).to_string(),
                        value: temp as f64,
                    },
                );
            }
        }
    }};
}

/// A complete settings file for the entire slicer.
#[cfg_attr(feature = "json_schema_gen", derive(JsonSchema))]
#[derive(Settings, Serialize, Deserialize, Debug)]
pub struct Settings {
    #[cfg_attr(feature = "json_schema_gen", schemars(example = "0.2"))]
    /// The height of the layers
    pub layer_height: f64,

    #[Recursive(PartialMovementParameter)]
    /// The extrusion width of the layers
    pub extrusion_width: MovementParameter,

    #[Recursive(PartialFilamentSettings)]
    /// The filament Settings
    pub filament: FilamentSettings,

    #[Recursive(PartialFanSettings)]
    /// The fan settings
    pub fan: FanSettings,

    #[Optional]
    #[Recursive(PartialFanSettings)]
    /// The auxilerary fan settings, if None or printer does not have one the fan will not be used
    pub aux_fan: Option<FanSettings>,

    /// The skirt settings, if None no skirt will be generated
    #[Optional]
    #[Recursive(PartialSkirtSettings)]
    pub skirt: Option<SkirtSettings>,

    /// The support settings, if None no support will be generated
    #[Optional]
    #[Recursive(PartialSupportSettings)]
    pub support: Option<SupportSettings>,

    #[cfg_attr(feature = "json_schema_gen", validate(range(min = 0.1)))]
    /// Diameter of the nozzle in mm
    pub nozzle_diameter: f64,

    /// length to retract in mm
    pub retract_length: f64,

    /// Distance to lift the z axis during a retract
    pub retract_lift_z: f64,

    /// The velocity of retracts
    pub retract_speed: f64,

    #[Optional]
    #[Recursive(PartialRetractionWipeSettings)]
    /// Retraction Wipe
    pub retraction_wipe: Option<RetractionWipeSettings>,

    #[Recursive(PartialMovementParameter)]
    /// The speeds used for movement
    pub speed: MovementParameter,

    #[Recursive(PartialMovementParameter)]
    /// The acceleration for movement
    pub acceleration: MovementParameter,

    #[cfg_attr(feature = "json_schema_gen", validate(range(max = 100.0)))]
    /// The percentage of infill to use for partial infill
    pub infill_percentage: f64,

    /// Controls the order of perimeters
    pub inner_perimeters_first: bool,

    #[cfg_attr(feature = "json_schema_gen", validate(range(min = 1)))]
    /// Number of perimeters to use if possible
    pub number_of_perimeters: u32,

    /// Number of solid top layers for infill
    pub top_layers: usize,

    /// Number of solid bottom layers before infill
    pub bottom_layers: usize,

    #[cfg_attr(feature = "json_schema_gen", validate(range(min = 1)))]
    /// Size of the printer in x dimension in mm
    pub print_x: f64,

    #[cfg_attr(feature = "json_schema_gen", validate(range(min = 1)))]
    /// Size of the printer in y dimension in mm
    pub print_y: f64,

    #[cfg_attr(feature = "json_schema_gen", validate(range(min = 1)))]
    /// Size of the printer in z dimension in mm
    pub print_z: f64,

    #[Optional]
    /// Width of the brim, if None no brim will be generated
    pub brim_width: Option<f64>,

    #[Optional]
    /// Inset the layer by the provided amount, if None no inset will be performed
    pub layer_shrink_amount: Option<f64>,

    /// The minimum travel distance required to perform a retraction
    pub minimum_retract_distance: f64,

    /// Overlap between infill and interior perimeters
    pub infill_perimeter_overlap_percentage: f64,

    /// Solid Infill type
    pub solid_infill_type: SolidInfillTypes,

    /// Partial Infill type
    pub partial_infill_type: PartialInfillTypes,

    /// The instructions to prepend to the exported instructions
    #[CustomPrint]
    pub starting_instructions: String,

    /// The instructions to append to the end of the exported instructions
    #[CustomPrint]
    pub ending_instructions: String,

    /// The instructions to append before layer changes
    #[CustomPrint]
    pub before_layer_change_instructions: String,

    /// The instructions to append after layer changes
    #[CustomPrint]
    pub after_layer_change_instructions: String,

    /// The instructions to append between object changes
    #[CustomPrint]
    pub object_change_instructions: String,

    /// Maximum Acceleration in x dimension
    pub max_acceleration_x: f64,
    /// Maximum Acceleration in y dimension
    pub max_acceleration_y: f64,
    /// Maximum Acceleration in z dimension
    pub max_acceleration_z: f64,
    /// Maximum Acceleration in e dimension
    pub max_acceleration_e: f64,

    /// Maximum Acceleration while extruding
    pub max_acceleration_extruding: f64,
    /// Maximum Acceleration while traveling
    pub max_acceleration_travel: f64,
    /// Maximum Acceleration while retracting
    pub max_acceleration_retracting: f64,

    /// Maximum Jerk in x dimension
    pub max_jerk_x: f64,
    /// Maximum Jerk in y dimension
    pub max_jerk_y: f64,
    /// Maximum Jerk in z dimension
    pub max_jerk_z: f64,
    /// Maximum Jerk in e dimension
    pub max_jerk_e: f64,

    /// Minimum feedrate for extrusion moves
    pub minimum_feedrate_print: f64,
    /// Minimum feedrate for travel moves
    pub minimum_feedrate_travel: f64,
    /// Maximum feedrate for x dimension
    pub maximum_feedrate_x: f64,
    /// Maximum feedrate for y dimension
    pub maximum_feedrate_y: f64,
    /// Maximum feedrate for z dimension
    pub maximum_feedrate_z: f64,
    /// Maximum feedrate for e dimension
    pub maximum_feedrate_e: f64,

    /// The maximum tempeture that the extruder can acheve safely
    pub max_extruder_temp: f64,

    #[Combine]
    #[AllowDefault]
    #[CustomPrint]
    /// Settings for specific layers
    pub layer_settings: Vec<(LayerRange, PartialLayerSettings)>,

    #[Combine]
    #[Optional]
    #[CustomPrint]
    /// Areas of the bed that can't have parts on it
    pub bed_exclude_areas: Option<MultiPolygon>,

    /// Tells the slicer if it can use an aux fan
    pub has_aux_fan: bool,

    #[Optional]
    #[cfg_attr(feature = "json_schema_gen", validate(range(min = -89.0, max = 89.0)))]
    /// The tilt angle of the slicing plane from the Z-axis in degrees, **0** if flat
    pub slice_angle: Option<f64>,
}

impl Settings {
    /// Generate the json schema for [`Settings`]
    #[cfg(feature = "json_schema_gen")]
    pub fn gen_schema(path: &std::path::Path) -> Result<(), std::io::Error> {
        use std::{fs::File, io::Write};

        let schema = schema_for!(Settings);
        let mut file = File::create(path.join("schema.json"))?;
        file.write_all(serde_json::to_string_pretty(&schema)?.as_bytes())?;

        Ok(())
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            layer_height: 0.15,
            number_of_perimeters: 3,
            top_layers: 3,
            bottom_layers: 3,
            extrusion_width: MovementParameter {
                interior_inner_perimeter: 0.4,
                interior_surface_perimeter: 0.4,
                exterior_inner_perimeter: 0.4,
                solid_top_infill: 0.4,
                solid_infill: 0.4,
                infill: 0.4,
                travel: 0.4,
                bridge: 0.4,
                support: 0.4,
                exterior_surface_perimeter: 0.4,
            },
            filament: FilamentSettings::default(),
            fan: FanSettings::default(),
            aux_fan: None,
            skirt: None,
            nozzle_diameter: 0.4,
            retract_length: 0.8,
            retract_lift_z: 0.6,
            retract_speed: 35.0,

            support: None,

            speed: MovementParameter {
                interior_inner_perimeter: 40.0,
                interior_surface_perimeter: 40.0,
                exterior_inner_perimeter: 40.0,
                solid_top_infill: 200.0,
                solid_infill: 200.0,
                infill: 200.0,
                travel: 180.0,
                bridge: 30.0,
                support: 50.0,
                exterior_surface_perimeter: 40.0,
            },
            acceleration: MovementParameter {
                interior_inner_perimeter: 900.0,
                interior_surface_perimeter: 900.0,
                exterior_inner_perimeter: 800.0,
                solid_top_infill: 1000.0,
                solid_infill: 1000.0,
                infill: 1000.0,
                travel: 1000.0,
                bridge: 1000.0,
                support: 1000.0,
                exterior_surface_perimeter: 800.0,
            },

            infill_percentage: 0.2,

            print_x: 210.0,
            print_y: 210.0,
            print_z: 210.0,
            inner_perimeters_first: true,
            minimum_retract_distance: 1.0,
            infill_perimeter_overlap_percentage: 0.25,
            solid_infill_type: SolidInfillTypes::Rectilinear,
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
            before_layer_change_instructions: String::new(),
            after_layer_change_instructions: String::new(),
            object_change_instructions: String::new(),
            max_acceleration_x: 1000.0,
            max_acceleration_y: 1000.0,
            max_acceleration_z: 1000.0,
            max_acceleration_e: 5000.0,
            max_acceleration_extruding: 1250.0,
            max_acceleration_travel: 1250.0,
            max_acceleration_retracting: 1250.0,
            max_jerk_x: 8.0,
            max_jerk_y: 8.0,
            max_jerk_z: 0.4,
            brim_width: None,
            layer_settings: vec![(
                LayerRange::SingleLayer(0),
                PartialLayerSettings {
                    extrusion_width: None,
                    speed: Some(PartialMovementParameter {
                        interior_inner_perimeter: Some(20.0),
                        interior_surface_perimeter: Some(20.0),
                        exterior_inner_perimeter: Some(20.0),
                        solid_top_infill: Some(20.0),
                        solid_infill: Some(20.0),
                        infill: Some(20.0),
                        travel: Some(5.0),
                        bridge: Some(20.0),
                        support: Some(20.0),
                        exterior_surface_perimeter: Some(20.0),
                    }),
                    layer_height: Some(0.3),
                    bed_temp: Some(60.0),
                    extruder_temp: Some(210.0),
                    ..Default::default()
                },
            )],
            layer_shrink_amount: None,
            max_jerk_e: 1.5,
            minimum_feedrate_print: 0.0,
            minimum_feedrate_travel: 0.0,
            maximum_feedrate_x: 200.0,
            maximum_feedrate_y: 200.0,
            maximum_feedrate_z: 12.0,
            maximum_feedrate_e: 120.0,
            retraction_wipe: None,
            bed_exclude_areas: None,
            max_extruder_temp: 260.0,
            has_aux_fan: false,
            slice_angle: Some(0.0),
        }
    }
}

impl Settings {
    /// Get the layer settings for a specific layer index and height
    // todo if their are no layer specific setting then skip
    pub fn get_layer_settings(&self, layer: u32, height: f64) -> LayerSettings {
        let changes = self
            .layer_settings
            .iter()
            .filter(|(layer_range, _)| match layer_range {
                LayerRange::LayerCountRange { end, start } => *start <= layer && layer <= *end,
                LayerRange::HeightRange { end, start } => *start <= height && height <= *end,
                LayerRange::SingleLayer(filter_layer) => *filter_layer == layer,
            })
            .map(|(_lr, pls)| pls)
            .fold(PartialLayerSettings::default(), |mut a, b| {
                a.combine(b.clone());
                a
            });

        LayerSettings {
            layer_height: changes.layer_height.unwrap_or(self.layer_height),
            layer_shrink_amount: changes.layer_shrink_amount.or(self.layer_shrink_amount),
            speed: changes
                .speed
                .map(|a| {
                    MovementParameter::try_from(inline_combine(a, self.speed.clone().into()))
                        .expect("self is geneteed to be complete")
                })
                .unwrap_or(self.speed.clone()),
            acceleration: changes
                .acceleration
                .map(|a| {
                    MovementParameter::try_from(inline_combine(a, self.acceleration.clone().into()))
                        .expect("self is geneteed to be complete")
                })
                .unwrap_or(self.acceleration.clone()),
            extrusion_width: changes
                .extrusion_width
                .map(|a| {
                    MovementParameter::try_from(inline_combine(
                        a,
                        self.extrusion_width.clone().into(),
                    ))
                    .expect("self is geneteed to be complete")
                })
                .unwrap_or(self.extrusion_width.clone()),
            solid_infill_type: changes
                .solid_infill_type
                .unwrap_or(self.solid_infill_type.clone()),
            partial_infill_type: changes
                .partial_infill_type
                .unwrap_or(self.partial_infill_type.clone()),
            infill_percentage: changes.infill_percentage.unwrap_or(self.infill_percentage),
            infill_perimeter_overlap_percentage: changes
                .infill_perimeter_overlap_percentage
                .unwrap_or(self.infill_perimeter_overlap_percentage),
            inner_perimeters_first: changes
                .inner_perimeters_first
                .unwrap_or(self.inner_perimeters_first),
            bed_temp: changes.bed_temp.unwrap_or(self.filament.bed_temp),
            extruder_temp: changes.extruder_temp.unwrap_or(self.filament.extruder_temp),
            retraction_wipe: changes
                .retraction_wipe
                .or_else(|| self.retraction_wipe.clone()),
            retraction_length: changes.retraction_length.unwrap_or(self.retract_length),
        }
    }

    /// Validate settings and return any warnings and errors
    pub fn validate_settings(&self) -> SettingsValidationResult {
        setting_less_than_or_equal_to_zero!(self, print_x);
        setting_less_than_or_equal_to_zero!(self, print_y);
        setting_less_than_or_equal_to_zero!(self, print_z);
        setting_less_than_or_equal_to_zero!(self, nozzle_diameter);
        setting_less_than_or_equal_to_zero!(self, layer_height);
        setting_less_than_or_equal_to_zero!(self, retract_speed);
        setting_less_than_or_equal_to_zero!(self, max_acceleration_x);
        setting_less_than_or_equal_to_zero!(self, max_acceleration_y);
        setting_less_than_or_equal_to_zero!(self, max_acceleration_z);
        setting_less_than_or_equal_to_zero!(self, max_acceleration_e);
        setting_less_than_or_equal_to_zero!(self, max_jerk_x);
        setting_less_than_or_equal_to_zero!(self, max_jerk_y);
        setting_less_than_or_equal_to_zero!(self, max_jerk_z);
        setting_less_than_or_equal_to_zero!(self, max_jerk_e);
        setting_less_than_or_equal_to_zero!(self, max_acceleration_extruding);
        setting_less_than_or_equal_to_zero!(self, max_acceleration_travel);
        setting_less_than_or_equal_to_zero!(self, max_acceleration_retracting);
        setting_less_than_or_equal_to_zero!(self, maximum_feedrate_x);
        setting_less_than_or_equal_to_zero!(self, maximum_feedrate_y);
        setting_less_than_or_equal_to_zero!(self, maximum_feedrate_z);
        setting_less_than_or_equal_to_zero!(self, maximum_feedrate_e);
        setting_less_than_or_equal_to_zero!(self, max_extruder_temp);
        setting_less_than_zero!(self, number_of_perimeters);
        setting_less_than_zero!(self, infill_percentage);
        setting_less_than_zero!(self, top_layers);
        setting_less_than_zero!(self, bottom_layers);
        setting_less_than_zero!(self, retract_length);
        setting_less_than_zero!(self, retract_lift_z);
        setting_less_than_zero!(self, minimum_feedrate_travel);
        setting_less_than_zero!(self, minimum_feedrate_print);
        setting_less_than_zero!(self, minimum_retract_distance);

        if let Some(exclude_area) = self.bed_exclude_areas.as_ref() {
            //If it fails its likely failing due to the polygon not being complete
            //The first and last points must be the same to be complete
            if let Some(reason) = exclude_area.explain_invalidity() {
                return SettingsValidationResult::Error(SlicerErrors::InvalidBedExcludeArea(
                    format!("{}", reason),
                ));
            }
        }

        if self.layer_height < self.nozzle_diameter * 0.2 {
            return SettingsValidationResult::Warning(SlicerWarnings::LayerSizeTooLow {
                layer_height: self.layer_height,
                nozzle_diameter: self.nozzle_diameter,
            });
        } else if self.layer_height > self.nozzle_diameter * 0.8 {
            return SettingsValidationResult::Warning(SlicerWarnings::LayerSizeTooHigh {
                layer_height: self.layer_height,
                nozzle_diameter: self.nozzle_diameter,
            });
        }

        let r = check_extrusions(&self.extrusion_width, self.nozzle_diameter);
        match r {
            SettingsValidationResult::NoIssue => {}
            _ => return r,
        }

        let r = check_accelerations(
            &self.acceleration,
            &self.speed,
            self.print_x.min(self.print_y),
        );
        match r {
            SettingsValidationResult::NoIssue => {}
            _ => return r,
        }

        if let Some(skirt) = self.skirt.as_ref() {
            if let Some(brim) = self.brim_width.as_ref() {
                if skirt.distance <= *brim {
                    return SettingsValidationResult::Warning(
                        SlicerWarnings::SkirtAndBrimOverlap {
                            skirt_distance: skirt.distance,
                            brim_width: *brim,
                        },
                    );
                }
            }
        }

        if self.filament.extruder_temp < 145.0 {
            return SettingsValidationResult::Warning(SlicerWarnings::NozzleTemperatureTooLow {
                temp: self.filament.extruder_temp,
            });
        } else if self.filament.extruder_temp > self.max_extruder_temp {
            return SettingsValidationResult::Warning(SlicerWarnings::NozzleTemperatureTooHigh {
                temp: self.filament.extruder_temp,
            });
        }

        // 90 or more can be represented with a smaller angle
        if self.slice_angle.unwrap_or_default() >= 90.0 || self.slice_angle.unwrap_or_default() <= -90.0 {
            return SettingsValidationResult::Error(
                SlicerErrors::SliceAngleOutOfRange(
                    self.slice_angle.expect("This can only be true if it's not default as that's 0.0")
                )
            );
        }

        for (_, pls) in &self.layer_settings {
            option_setting_less_than_or_equal_to_zero!(pls, layer_height);
            option_setting_less_than_zero!(pls, infill_percentage);
            option_setting_less_than_zero!(pls, retraction_length);

            if let Some(layer_height) = pls.layer_height {
                if layer_height < self.nozzle_diameter * 0.2 {
                    return SettingsValidationResult::Warning(SlicerWarnings::LayerSizeTooLow {
                        layer_height: self.layer_height,
                        nozzle_diameter: self.nozzle_diameter,
                    });
                } else if layer_height > self.nozzle_diameter * 0.8 {
                    return SettingsValidationResult::Warning(SlicerWarnings::LayerSizeTooHigh {
                        layer_height: self.layer_height,
                        nozzle_diameter: self.nozzle_diameter,
                    });
                }
            }

            if let Some(extruder_temp) = pls.extruder_temp {
                if extruder_temp < 140.0 {
                    return SettingsValidationResult::Warning(
                        SlicerWarnings::NozzleTemperatureTooLow {
                            temp: self.filament.extruder_temp,
                        },
                    );
                } else if extruder_temp > 260.0 {
                    return SettingsValidationResult::Warning(
                        SlicerWarnings::NozzleTemperatureTooHigh {
                            temp: self.filament.extruder_temp,
                        },
                    );
                }
            }

            let r = if let Some(extrusion_width) = pls.extrusion_width.clone() {
                let combined_extrustion: MovementParameter = MovementParameter::try_from(
                    inline_combine(extrusion_width, self.extrusion_width.clone().into()),
                )
                .expect("self is geneteed to be complete");
                check_extrusions(&combined_extrustion, self.nozzle_diameter)
            } else {
                SettingsValidationResult::NoIssue
            };

            match r {
                SettingsValidationResult::NoIssue => {}
                _ => return r,
            }
            let combined_acceleration: MovementParameter = pls
                .acceleration
                .clone()
                .map(|a| {
                    MovementParameter::try_from(inline_combine(a, self.acceleration.clone().into()))
                        .expect("self is geneteed to be complete")
                })
                .unwrap_or(self.acceleration.clone());

            let combined_speed: MovementParameter = pls
                .speed
                .clone()
                .map(|a| {
                    MovementParameter::try_from(inline_combine(a, self.speed.clone().into()))
                        .expect("self is geneteed to be complete")
                })
                .unwrap_or(self.speed.clone());

            let r = check_accelerations(
                &combined_acceleration,
                &combined_speed,
                self.print_x.min(self.print_y),
            );
            match r {
                SettingsValidationResult::NoIssue => {}
                _ => return r,
            }
        }

        SettingsValidationResult::NoIssue
    }
}

/// Possible results of validation the settings
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum SettingsValidationResult {
    /// No Issue
    NoIssue,
    /// A warning
    Warning(SlicerWarnings),
    /// An error
    Error(SlicerErrors),
}

/// Settings specific to a Layer
#[derive(Settings)]
pub struct LayerSettings {
    /// The height of the layers
    pub layer_height: f64,

    #[Optional]
    /// Inset the layer by the provided amount, if None on inset will be performed
    pub layer_shrink_amount: Option<f64>,

    /// The speeds used for movement
    #[Recursive(PartialMovementParameter)]
    pub speed: MovementParameter,

    /// The acceleration for movement
    #[Recursive(PartialMovementParameter)]
    pub acceleration: MovementParameter,

    /// The extrusion width of the layers
    #[Recursive(PartialMovementParameter)]
    pub extrusion_width: MovementParameter,

    /// Solid Infill type
    pub solid_infill_type: SolidInfillTypes,

    /// Partial Infill type
    pub partial_infill_type: PartialInfillTypes,

    /// The percentage of infill to use for partial infill
    pub infill_percentage: f64,

    /// Overlap between infill and interior perimeters
    pub infill_perimeter_overlap_percentage: f64,

    /// Controls the order of perimeters
    pub inner_perimeters_first: bool,

    /// Temperature of the bed
    pub bed_temp: f64,

    /// Temperature of the extuder
    pub extruder_temp: f64,

    #[Optional]
    #[Recursive(PartialRetractionWipeSettings)]
    /// Retraction Wipe
    pub retraction_wipe: Option<RetractionWipeSettings>,

    /// Retraction Distance
    pub retraction_length: f64,
}

/// A set of values for different movement types
#[cfg_attr(feature = "json_schema_gen", derive(JsonSchema))]
#[derive(Settings, Serialize, Deserialize, Debug, Clone)]
pub struct MovementParameter {
    /// Value for interior (perimeters that are inside the model
    pub interior_inner_perimeter: f64,

    /// Value for interior perimeters surface perimeter
    pub interior_surface_perimeter: f64,

    /// Value for exterior perimeters that are inside the model
    pub exterior_inner_perimeter: f64,

    /// Value for exterior surface perimeter
    pub exterior_surface_perimeter: f64,

    /// Value for solid top infill moves
    pub solid_top_infill: f64,

    /// Value for solid infill moves
    pub solid_infill: f64,

    /// Value for pertial infill moves
    pub infill: f64,

    /// Value for travel moves
    pub travel: f64,

    /// Value for bridging
    pub bridge: f64,

    /// Value for support structures
    pub support: f64,
}

impl MovementParameter {
    /// Returns the associated value to the move type provided
    pub const fn get_value_for_movement_type(&self, move_type: &MoveType) -> f64 {
        match move_type {
            MoveType::TopSolidInfill => self.solid_top_infill,
            MoveType::SolidInfill => self.solid_infill,
            MoveType::Infill => self.infill,
            MoveType::ExteriorSurfacePerimeter => self.exterior_surface_perimeter,
            MoveType::InteriorSurfacePerimeter => self.interior_surface_perimeter,
            MoveType::ExteriorInnerPerimeter => self.exterior_inner_perimeter,
            MoveType::InteriorInnerPerimeter => self.interior_inner_perimeter,
            MoveType::Bridging => self.bridge,
            MoveType::Support => self.support,
            MoveType::Travel => self.travel,
        }
    }
}

/// Settings for a filament
#[cfg_attr(feature = "json_schema_gen", derive(JsonSchema))]
#[derive(Settings, Serialize, Deserialize, Debug, Clone)]
pub struct FilamentSettings {
    /// Diameter of this filament in mm
    pub diameter: f64,

    /// Density of this filament in grams per cm^3
    pub density: f64,

    /// Cost of this filament in $ per kg
    pub cost: f64,

    /// Extruder temp for this filament
    pub extruder_temp: f64,

    /// Bed temp for this filament
    pub bed_temp: f64,
}

/// Settings for the fans
#[cfg_attr(feature = "json_schema_gen", derive(JsonSchema))]
#[derive(Settings, Serialize, Deserialize, Debug, Clone)]
pub struct FanSettings {
    #[cfg_attr(feature = "json_schema_gen", validate(range(max = 100.0)))]
    /// The default fan speed
    pub fan_speed: f64,

    /// Disable the fan for layers below this value
    pub disable_fan_for_layers: u32,

    /// Threshold to start slowing down based on layer print time in seconds
    pub slow_down_threshold: f64,

    /// Minimum speed to slow down to
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

/// Support settings
#[cfg_attr(feature = "json_schema_gen", derive(JsonSchema))]
#[derive(Settings, Serialize, Deserialize, Debug, Clone)]
pub struct SupportSettings {
    /// Angle to start production supports in degrees
    pub max_overhang_angle: f64,

    /// Spacing between the ribs of support
    pub support_spacing: f64,
}

/// The Settings for Skirt generation
#[cfg_attr(feature = "json_schema_gen", derive(JsonSchema))]
#[derive(Settings, Serialize, Deserialize, Debug, Clone)]
pub struct SkirtSettings {
    /// the number of layer to generate the skirt
    pub layers: u32,

    /// Distance from the models to place the skirt
    pub distance: f64,
}

/// The Settings for Skirt generation
#[cfg_attr(feature = "json_schema_gen", derive(JsonSchema))]
#[derive(Settings, Serialize, Deserialize, Debug, Clone)]
pub struct RetractionWipeSettings {
    /// The speed the retract wipe move
    pub speed: f64,

    /// The acceleration the retract wipe move
    pub acceleration: f64,

    /// Wipe Distance in mm
    pub distance: f64,
}

/// A partial complete settings file
#[cfg_attr(feature = "json_schema_gen", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialSettingsFile {
    /// Other files to load
    pub other_files: Option<Vec<String>>,

    ///The incomplete settings that this files contains that will be prioritized over the contents of the other files
    #[serde(flatten)]
    pub partial_settings: PartialSettings,
}

impl PartialSettingsFile {
    /// Convert a partial settings file into a complete settings file
    /// returns an error if a settings is not present in this or any sub file
    pub fn get_settings(mut self, path: PathBuf) -> Result<Settings, SlicerErrors> {
        let current_path =
            std::env::current_dir().map_err(|_| SlicerErrors::SettingsFilePermission)?;

        //set the directory of the current directory
        std::env::set_current_dir(&path).expect("Path checked before");
        trace!("Setting path to {:?}", path);

        self.combine_with_other_files()?;

        // reset path
        trace!("Setting path to {:?}", current_path);
        std::env::set_current_dir(current_path).expect("Path checked before");

        Settings::try_from(self.partial_settings).map_err(|err| {
            SlicerErrors::SettingsFileMissingSettings {
                missing_setting: err.0,
            }
        })
    }

    fn combine_with_other_files(&mut self) -> Result<(), SlicerErrors> {
        let files: Vec<String> = self
            .other_files
            .as_mut()
            .map(std::mem::take)
            .unwrap_or_default();

        for file in &files {
            let mut ps: PartialSettingsFile =
                deser_hjson::from_str(&std::fs::read_to_string(file).map_err(|_| {
                    SlicerErrors::SettingsRecursiveLoadError {
                        filepath: file.to_string(),
                    }
                })?)
                .map_err(|_| SlicerErrors::SettingsFileMisformat {
                    filepath: file.to_string(),
                })?;

            let mut path =
                PathBuf::from_str(file).map_err(|_| SlicerErrors::SettingsFileNotFound {
                    filepath: file.to_string(),
                })?;
            trace!("Setting path to {:?}", path);
            path.pop();

            let current_path =
                std::env::current_dir().map_err(|_| SlicerErrors::SettingsFilePermission)?;
            //set the directory of the current directory
            if path.exists() {
                trace!("Setting path to {:?}", path);
                std::env::set_current_dir(&path).expect("Path checked before");
            }

            ps.combine_with_other_files()?;

            // reset path
            trace!("Setting path to {:?}", current_path);
            std::env::set_current_dir(current_path).expect("Path checked before");

            self.partial_settings.combine(ps.partial_settings);
        }

        Ok(())
    }
}

/// The different types of layer ranges supported
#[cfg_attr(feature = "json_schema_gen", derive(JsonSchema))]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum LayerRange {
    /// A single single based on the index
    SingleLayer(u32),

    /// A range of layers based on index inclusive
    LayerCountRange {
        /// The start index
        start: u32,

        /// The end index
        end: u32,
    },

    /// A Range of layers based on the height of the bottom on the slice
    HeightRange {
        /// The start height
        start: f64,

        /// The end height
        end: f64,
    },
}

impl Display for LayerRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LayerRange::SingleLayer(layer) => write!(f, "SingleLayer({})", layer)?,
            LayerRange::LayerCountRange { start, end } => {
                write!(f, "LayerCountRange({}<=layer<={})", start, end)?
            }
            LayerRange::HeightRange { start, end } => {
                write!(f, "HeightRange({}<=height<={})", start, end)?
            }
        }
        Ok(())
    }
}

fn check_extrusions(
    extrusion_width: &MovementParameter,
    nozzle_diameter: f64,
) -> SettingsValidationResult {
    // infill
    if extrusion_width.infill < nozzle_diameter * 0.6 {
        return SettingsValidationResult::Warning(SlicerWarnings::ExtrusionWidthTooLow {
            extrusion_width: extrusion_width.infill,
            nozzle_diameter,
        });
    } else if extrusion_width.infill > nozzle_diameter * 2.0 {
        return SettingsValidationResult::Warning(SlicerWarnings::ExtrusionWidthTooHigh {
            extrusion_width: extrusion_width.infill,
            nozzle_diameter,
        });
    }

    // top infill
    if extrusion_width.solid_top_infill < nozzle_diameter * 0.6 {
        return SettingsValidationResult::Warning(SlicerWarnings::ExtrusionWidthTooLow {
            extrusion_width: extrusion_width.solid_top_infill,
            nozzle_diameter,
        });
    } else if extrusion_width.solid_top_infill > nozzle_diameter * 2.0 {
        return SettingsValidationResult::Warning(SlicerWarnings::ExtrusionWidthTooHigh {
            extrusion_width: extrusion_width.solid_top_infill,
            nozzle_diameter,
        });
    }

    // solid infill
    if extrusion_width.solid_infill < nozzle_diameter * 0.6 {
        return SettingsValidationResult::Warning(SlicerWarnings::ExtrusionWidthTooLow {
            extrusion_width: extrusion_width.solid_infill,
            nozzle_diameter,
        });
    } else if extrusion_width.solid_infill > nozzle_diameter * 2.0 {
        return SettingsValidationResult::Warning(SlicerWarnings::ExtrusionWidthTooHigh {
            extrusion_width: extrusion_width.solid_infill,
            nozzle_diameter,
        });
    }

    // bridge
    if extrusion_width.bridge < nozzle_diameter * 0.6 {
        return SettingsValidationResult::Warning(SlicerWarnings::ExtrusionWidthTooLow {
            extrusion_width: extrusion_width.bridge,
            nozzle_diameter,
        });
    } else if extrusion_width.bridge > nozzle_diameter * 2.0 {
        return SettingsValidationResult::Warning(SlicerWarnings::ExtrusionWidthTooHigh {
            extrusion_width: extrusion_width.bridge,
            nozzle_diameter,
        });
    }

    // support
    if extrusion_width.support < nozzle_diameter * 0.6 {
        return SettingsValidationResult::Warning(SlicerWarnings::ExtrusionWidthTooLow {
            extrusion_width: extrusion_width.support,
            nozzle_diameter,
        });
    } else if extrusion_width.support > nozzle_diameter * 2.0 {
        return SettingsValidationResult::Warning(SlicerWarnings::ExtrusionWidthTooHigh {
            extrusion_width: extrusion_width.support,
            nozzle_diameter,
        });
    }

    // interior_surface_perimeter
    if extrusion_width.interior_surface_perimeter < nozzle_diameter * 0.6 {
        return SettingsValidationResult::Warning(SlicerWarnings::ExtrusionWidthTooLow {
            extrusion_width: extrusion_width.interior_surface_perimeter,
            nozzle_diameter,
        });
    } else if extrusion_width.interior_surface_perimeter > nozzle_diameter * 2.0 {
        return SettingsValidationResult::Warning(SlicerWarnings::ExtrusionWidthTooHigh {
            extrusion_width: extrusion_width.interior_surface_perimeter,
            nozzle_diameter,
        });
    }

    // interior_inner_perimeter
    if extrusion_width.interior_inner_perimeter < nozzle_diameter * 0.6 {
        return SettingsValidationResult::Warning(SlicerWarnings::ExtrusionWidthTooLow {
            extrusion_width: extrusion_width.interior_inner_perimeter,
            nozzle_diameter,
        });
    } else if extrusion_width.interior_inner_perimeter > nozzle_diameter * 2.0 {
        return SettingsValidationResult::Warning(SlicerWarnings::ExtrusionWidthTooHigh {
            extrusion_width: extrusion_width.interior_inner_perimeter,
            nozzle_diameter,
        });
    }

    // exterior_inner_perimeter
    if extrusion_width.exterior_inner_perimeter < nozzle_diameter * 0.6 {
        return SettingsValidationResult::Warning(SlicerWarnings::ExtrusionWidthTooLow {
            extrusion_width: extrusion_width.exterior_inner_perimeter,
            nozzle_diameter,
        });
    } else if extrusion_width.exterior_inner_perimeter > nozzle_diameter * 2.0 {
        return SettingsValidationResult::Warning(SlicerWarnings::ExtrusionWidthTooHigh {
            extrusion_width: extrusion_width.exterior_inner_perimeter,
            nozzle_diameter,
        });
    }

    // exterior_surface_perimeter
    if extrusion_width.exterior_surface_perimeter < nozzle_diameter * 0.6 {
        return SettingsValidationResult::Warning(SlicerWarnings::ExtrusionWidthTooLow {
            extrusion_width: extrusion_width.exterior_surface_perimeter,
            nozzle_diameter,
        });
    } else if extrusion_width.exterior_surface_perimeter > nozzle_diameter * 2.0 {
        return SettingsValidationResult::Warning(SlicerWarnings::ExtrusionWidthTooHigh {
            extrusion_width: extrusion_width.exterior_surface_perimeter,
            nozzle_diameter,
        });
    }

    SettingsValidationResult::NoIssue
}

fn check_accelerations(
    acceleration: &MovementParameter,
    speed: &MovementParameter,
    min_bed_dimension: f64,
) -> SettingsValidationResult {
    // infill
    if (speed.infill * speed.infill) / (2.0 * acceleration.infill) > min_bed_dimension {
        return SettingsValidationResult::Warning(SlicerWarnings::AccelerationTooLow {
            acceleration: acceleration.infill,
            speed: speed.infill,
            bed_size: min_bed_dimension,
        });
    }

    // top infill
    if (speed.solid_top_infill * speed.solid_top_infill) / (2.0 * acceleration.solid_top_infill)
        > min_bed_dimension
    {
        return SettingsValidationResult::Warning(SlicerWarnings::AccelerationTooLow {
            acceleration: acceleration.solid_top_infill,
            speed: speed.solid_top_infill,
            bed_size: min_bed_dimension,
        });
    }

    // solid infill
    if (speed.solid_infill * speed.solid_infill) / (2.0 * acceleration.solid_infill)
        > min_bed_dimension
    {
        return SettingsValidationResult::Warning(SlicerWarnings::AccelerationTooLow {
            acceleration: acceleration.solid_infill,
            speed: speed.solid_infill,
            bed_size: min_bed_dimension,
        });
    }

    // bridge
    if (speed.bridge * speed.bridge) / (2.0 * acceleration.bridge) > min_bed_dimension {
        return SettingsValidationResult::Warning(SlicerWarnings::AccelerationTooLow {
            acceleration: acceleration.bridge,
            speed: speed.bridge,
            bed_size: min_bed_dimension,
        });
    }

    // support
    if (speed.support * speed.support) / (2.0 * acceleration.support) > min_bed_dimension {
        return SettingsValidationResult::Warning(SlicerWarnings::AccelerationTooLow {
            acceleration: acceleration.support,
            speed: speed.support,
            bed_size: min_bed_dimension,
        });
    }

    // interior_surface_perimeter
    if (speed.interior_surface_perimeter * speed.interior_surface_perimeter)
        / (2.0 * acceleration.interior_surface_perimeter)
        > min_bed_dimension
    {
        return SettingsValidationResult::Warning(SlicerWarnings::AccelerationTooLow {
            acceleration: acceleration.interior_surface_perimeter,
            speed: speed.interior_surface_perimeter,
            bed_size: min_bed_dimension,
        });
    }

    // interior_inner_perimeter
    if (speed.interior_inner_perimeter * speed.interior_inner_perimeter)
        / (2.0 * acceleration.interior_inner_perimeter)
        > min_bed_dimension
    {
        return SettingsValidationResult::Warning(SlicerWarnings::AccelerationTooLow {
            acceleration: acceleration.interior_inner_perimeter,
            speed: speed.interior_inner_perimeter,
            bed_size: min_bed_dimension,
        });
    }

    // exterior_inner_perimeter
    if (speed.exterior_inner_perimeter * speed.exterior_inner_perimeter)
        / (2.0 * acceleration.exterior_inner_perimeter)
        > min_bed_dimension
    {
        return SettingsValidationResult::Warning(SlicerWarnings::AccelerationTooLow {
            acceleration: acceleration.exterior_inner_perimeter,
            speed: speed.exterior_inner_perimeter,
            bed_size: min_bed_dimension,
        });
    }

    // exterior_surface_perimeter
    if (speed.exterior_surface_perimeter * speed.exterior_surface_perimeter)
        / (2.0 * acceleration.exterior_surface_perimeter)
        > min_bed_dimension
    {
        return SettingsValidationResult::Warning(SlicerWarnings::AccelerationTooLow {
            acceleration: acceleration.exterior_surface_perimeter,
            speed: speed.exterior_surface_perimeter,
            bed_size: min_bed_dimension,
        });
    }

    SettingsValidationResult::NoIssue
}

trait Combine {
    fn combine(&mut self, other: Self);
}

///Controls how to convert settings into a list of strings
pub trait SettingsPrint {
    ///Controls how to convert settings into a list of strings
    fn to_strings(&self) -> Vec<String>;
}

impl<T> Combine for Vec<T> {
    fn combine(&mut self, mut other: Self) {
        self.append(&mut other);
    }
}

impl<T> Combine for Option<T> {
    fn combine(&mut self, other: Self) {
        if self.is_none() {
            *self = other;
        }
    }
}

impl Combine for MultiPolygon {
    fn combine(&mut self, other: Self) {
        self.0.combine(other.0);
    }
}

fn inline_combine<T>(mut a: T, b: T) -> T
where
    T: Combine,
{
    a.combine(b);
    a
}

impl SettingsPrint for MultiPolygon {
    fn to_strings(&self) -> Vec<String> {
        vec![format!("{:?}", self)]
    }
}

impl SettingsPrint for String {
    fn to_strings(&self) -> Vec<String> {
        vec![self.replace('\n', "\\n").replace('\r', "")]
    }
}

impl<T> SettingsPrint for Vec<T>
where
    T: SettingsPrint,
{
    fn to_strings(&self) -> Vec<String> {
        let mut settings = vec![];
        for s in self {
            settings.append(&mut s.to_strings());
        }
        settings
    }
}

impl<T> SettingsPrint for Option<T>
where
    T: SettingsPrint,
{
    fn to_strings(&self) -> Vec<String> {
        self.iter().flat_map(|x| x.to_strings()).collect()
    }
}

impl<N, S> SettingsPrint for (N, S)
where
    N: Display,
    S: SettingsPrint,
{
    fn to_strings(&self) -> Vec<String> {
        let mut settings = vec![];
        let (n, s) = self;

        let mut line = format!("{} : [", n);
        for s in s.to_strings() {
            line += &format!("{},", s);
        }
        //remove last comma
        line.pop();
        line += "]";

        settings.push(line);

        settings
    }
}

/// Error for Partial Conversion to Full Type
/// String contains missing path
#[derive(Serialize, Deserialize, Debug)]
pub struct PartialConvertError(String);
