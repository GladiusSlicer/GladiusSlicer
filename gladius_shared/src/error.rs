use geo::MultiPolygon;
use serde::{Deserialize, Serialize};

/// Errors that can be generated during the slicing process
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SlicerErrors {
    /// Thefile for the object/Model can not be found in the file system
    ObjectFileNotFound {
        /// File that was not found
        filepath: String,
    },

    /// The file for the settings can not be found in the file system
    SettingsFileNotFound {
        /// File that was not found
        filepath: String,
    },

    /// The settings file can't be parsed
    SettingsFileMisformat {
        /// File that was misformatted
        filepath: String,
    },

    /// A setting is missing from the settings file
    SettingsFileMissingSettings {
        /// Setting that was missing
        missing_setting: String,
    },

    /// Error loading the STL
    StlLoadError,

    /// Error Loading the 3MF file
    ThreemfLoadError,

    /// Error loading the 3MF file, usually a compatibility error
    ThreemfUnsupportedType,

    /// Error during tower generation
    TowerGeneration,

    /// No input models provided
    NoInputProvided,

    /// All input must be UTF8
    InputNotUTF8,

    /// Input string is misformated
    InputMisformat,

    /// Model would cause moves outside build area
    ModelOutsideBuildArea,

    /// Generated move outside build area
    MovesOutsideBuildArea,

    /// settings file could not be loaded
    SettingsRecursiveLoadError {
        /// File that was not found
        filepath: String,
    },

    /// Error during tower generation
    SliceGeneration,

    /// File permission issue will settings file or folder
    SettingsFilePermission,

    /// Failed to create new file
    FileCreateError {
        /// File that was not created
        filepath: String,
    },

    /// Failed to write to file
    FileWriteError {
        /// File that was no able to write to
        filepath: String,
    },

    /// Error because settings less than zero
    SettingLessThanZero {
        /// The setting name
        setting: String,

        /// The current value
        value: f64,
    },

    /// Error because settings less than or equal to zero
    SettingLessThanOrEqualToZero {
        /// The setting name
        setting: String,

        /// The current value
        value: f64,
    },

    /// The file format is not supported
    FileFormatNotSupported {
        /// File with invalid Format
        filepath: String,
    },

    /// Another error, here for plugins to use
    UnspecifiedError(String),

    /// If a model is in an area of the bed that is rserved, contains the area that it intersected
    InExcludeArea(MultiPolygon)
}

impl SlicerErrors {
    /// Return the error code and pretty error message
    pub fn get_code_and_message(&self) -> (u32, String) {
        match self {
            SlicerErrors::UnspecifiedError(err_string) => {
                (0xFFFF_FFFF, format!("Third Party Error: {}.", err_string))
            }
            SlicerErrors::ObjectFileNotFound { filepath } => {
                (0x1000, format!("Could not load object file \"{}\". It was not found in the filesystem. Please check that the file exists and retry.", filepath))
            }
            SlicerErrors::SettingsFileNotFound {filepath} => {
                (0x1001, format!("Could not load settings file \"{}\". It was not found in the filesystem. Please check that the file exists and retry.", filepath))
            }
            SlicerErrors::StlLoadError => {
                (0x1002, "There was a issue loading the STL file.".to_string())
            }
            SlicerErrors::ThreemfLoadError => {
                (0x1003, "There was a issue loading the 3MF file. This file format is still in development. Please report this issue to github.".to_string())
            }
            SlicerErrors::ThreemfUnsupportedType => {
                (0x1004, "There was a issue loading the 3MF file. This file is unsupported by our zip reader dependency. Work is going towards upgrading support for these files.".to_string())
            }
            SlicerErrors::SettingsFileMisformat { filepath } => {
                (0x1005, format!("Could not load settings file \"{}\". It was formatted incorrectly.", filepath))
            }
            SlicerErrors::SettingsFileMissingSettings { missing_setting } => {
                (0x1006, format!("Could not load settings file. Was missing settings {}.", missing_setting))
            }
            SlicerErrors::TowerGeneration  => {
                (0x1007, "Error Creating Tower. Model most likely needs repair. Please Repair and run again.".to_string())
            }
            SlicerErrors::NoInputProvided  => {
                (0x1008, "No Input Provided.".to_string())
            }
            SlicerErrors::InputMisformat  => {
                (0x1009, "Input Incorrectly Formatted".to_string())
            }
            SlicerErrors::SettingsRecursiveLoadError { filepath } => {
                (0x100A, format!("Failed to load additional settings file {}", filepath))
            }
            SlicerErrors::SliceGeneration => {
                (0x100B, "There was a issue ordering the polygon for slicing. Try repairing your Model.".to_string())
            }
            SlicerErrors::SettingLessThanZero { setting, value } => {
                (0x100C, format!("The setting {} must be greater than or equal to 0. It is currently {}.", setting, value))
            }
            SlicerErrors::SettingLessThanOrEqualToZero { setting, value } => {
                (0x100D, format!("The setting {} must be greater than to 0. It is currently {}.", setting, value))
            }
            SlicerErrors::InputNotUTF8 => {
                (0x100E, "Input String must be UTF8.".to_string())
            }
            SlicerErrors::SettingsFilePermission => {
                (0x100F, "File permission issue will settings file or folder.".to_string())
            }
            SlicerErrors::FileCreateError { filepath } => {
                (0x1010, format!("Could not create file \"{}\".", filepath))
            }
            SlicerErrors::FileWriteError { filepath } => {
                (0x1011, format!("Could not write to file \"{}\".", filepath))
            }
            SlicerErrors::FileFormatNotSupported { filepath} => {
                (0x1012, format!("The file {} has an invalid or unsupported format", filepath))
            }
            SlicerErrors::ModelOutsideBuildArea => {
                (0x1013, "Model is outside printers build area.".to_string())
            }
            SlicerErrors::MovesOutsideBuildArea => {
                (0x1014, "Slicer generated move outside build area.".to_string())
            }
            SlicerErrors::InExcludeArea(area) => {
                (0x1015, format!("A model intersected with this excluded area: {:?}", area))
            },
        }
    }
}
