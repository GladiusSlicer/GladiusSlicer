use serde::{Deserialize, Serialize};

///Errors that can be generated during the slicing process
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SlicerErrors {
    ///Thefile for the object/Model can not be found in the file system
    ObjectFileNotFound {
        ///File that was not found
        filepath: String,
    },

    ///The file for the settings can not be found in the file system
    SettingsFileNotFound {
        ///File that was not found
        filepath: String,
    },

    ///The settings file can't be parsed
    SettingsFileMisformat {
        ///File that was misformatted
        filepath: String,
    },

    ///A setting is missing from the settings file
    SettingsFileMissingSettings {
        ///Setting that was missing
        missing_setting: String,
    },

    ///Error loading the STL
    StlLoadError,

    ///Error Loading the 3MF file
    ThreemfLoadError,

    /// Error loading the 3MF file, usally a compatiblity error
    ThreemfUnsupportedType,

    ///Error during tower generation
    TowerGeneration,

    ///No inout models provided
    NoInputProvided,

    ///Input string is misformated
    InputMisformat,

    ///settings file could not be loaded
    SettingsRecursiveLoadError {
        ///File that was not found
        filepath: String,
    },

    ///Error during tower generation
    SliceGeneration,

    ///Another error, here for plugins to use
    UnspecifiedError(String),
}

impl SlicerErrors {
    ///Return the error code and pretty error message
    pub fn get_code_and_message(&self) -> (u32, String) {
        match self {
            SlicerErrors::ObjectFileNotFound { filepath } => {
                (0x1000,format!("Could not load object file \"{}\". It was not found in the filesystem. Please check that the file exists and retry.",filepath))
            }
            SlicerErrors::SettingsFileNotFound {filepath} => {
                (0x1001,format!("Could not load settings file \"{}\". It was not found in the filesystem. Please check that the file exists and retry.",filepath))
            }
            SlicerErrors::StlLoadError => {
                (0x1002,"There was a issue loading the STL file.".to_string())
            }
            SlicerErrors::ThreemfLoadError => {
                (0x1003,"There was a issue loading the 3MF file. This file format is still in development. Please report this issue to github.".to_string())
            }
            SlicerErrors::ThreemfUnsupportedType => {
                (0x1004,"There was a issue loading the 3MF file. This file is unsupported by our zip reader dependency. Work is going towards upgrading support for these files.".to_string())
            }
            SlicerErrors::SettingsFileMisformat { filepath } => {
                (0x1005,format!("Could not load settings file \"{}\". It was formatted incorrectly.",filepath))
            }
            SlicerErrors::SettingsFileMissingSettings { missing_setting } => {
                (0x1006,format!("Could not load settings file. Was missing settings {}.",missing_setting))
            }
            SlicerErrors::TowerGeneration  => {
                (0x1007,"Error Creating Tower. Model most likely needs repair. Please Repair and run again.".to_string())
            }
            SlicerErrors::NoInputProvided  => {
                (0x1008,"No Input Provided.".to_string())
            }
            SlicerErrors::InputMisformat  => {
                (0x1009,"Input Incorrectly Formatted".to_string())
            }
            SlicerErrors::SettingsRecursiveLoadError { filepath } => {
                (0x100A,format!("Failed to load addional settings file {}",filepath))
            }
            SlicerErrors::SliceGeneration => {
                (0x100B,"There was a issue ordering the polygon for slicing. Try repairing your Model.".to_string())
            }
            SlicerErrors::UnspecifiedError(err_string) => {
                (0xFFFFFFFF,format!("Third Party Error. {}",err_string))
            }
        }
    }
}
