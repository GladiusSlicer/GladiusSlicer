#[derive(Clone, Debug, PartialEq)]
pub enum SlicerErrors {
    ObjectFileNotFound{filepath:String},
    SettingsFileNotFound{filepath:String},
    SettingsFileMisformat{filepath:String},
    SettingsFileMissingSettings{missing_setting:String},
    StlLoadError,
    ThreemfLoadError,
    ThreemfUnsupportedType,
    TowerGeneration,
    UnspecifiedError(String)
}

impl SlicerErrors {
    pub fn show_error_message(&self){
        let (error_code,message) =  self.get_code_and_message();
        println!("\n");
        println!("**************************************************");
        println!("\tGladius Slicer Ran into an error");
        println!("\tError Code: {:#X}",error_code);
        println!("\t{}",message);
        println!("**************************************************");
        println!("\n\n\n");
    }

    fn get_code_and_message(&self) -> (u32,String){
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
                (0x1007,format!("Error Creating Tower. Model most likely needs repair. Please Repair and run again."))
            }
            SlicerErrors::UnspecifiedError(err_string) => {
                (0xFFFFFFFF,format!("Third Party Error. {}",err_string))
            }
        }
    }

}