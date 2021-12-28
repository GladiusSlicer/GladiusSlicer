#Gladius Slicer

**In-Progress** Slicer for FDM 3D printing written in Rust with a focus on customization and modularity.

Rust, LGPL. Copyright 2021 Lucas Ince/mrhatman

Questions? Please use the github discussion page.

Want to contribute? Open a PR. 

Gladius Slicer is currently in a Alpha state and very little is stable so assume any new release will cause breaking bugs. If you need something stable, please open an issue or discussion, so we can plan out the interface. 

# Usage

This project is a command line application. A GUI is planned but not my expertise. A seperate project will be created when a GUI is started.

```
USAGE:
    gladius_slicer.exe [FLAGS] [OPTIONS] <INPUT>...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v               Sets the level of verbosity

OPTIONS:
    -o <OUTPUT>            Sets the output file to use
    -s <SETTINGS>          Sets the settings file to use
    -c, --config <FILE>    Sets a custom config file

ARGS:
    <INPUT>...    Sets the input file to use

```

###Input file examples:
* " {\"Auto\":\"test_3D_models\\3DBenchy.stl\"}" : Automatically Centers 3DBenchy file in center of the build area
* " {\"AutoTranslate\":[\"test_3D_models\\3DBenchy.stl\",-10,0]} "  : Automatically centers 3DBenchy file in center of the build area offset -10 mm off center in the x dimension.
* " {\"AutoTranslate\":[\"test_3D_models\\3DBenchy.stl\",0,-80]} " "{\"AutoTranslate\":[\"test_3D_models\\3DBenchy.stl\",0,80]} " : print 2 3DBenchy's 
* " [[1.0,0.0,0.0,124.1760025024414],[0.0,1.0,0.0,105.0009994506836],[0.0,0.0,1.0,2.7030678211303893e-6],[0.0,0.0,0.0,1.0]] ": print 3DBenchy with given transform matrix

###Settings file example
Settings files are hJSON, a human readable json that supports comments. Examples can be found in the settings folder.

# Current Status

 Generated GCode has been used for multiple prints (Benchy, xyz calibration cube,marvin) with a Prusa Mk3 and gives similar results to other slicers. 

###Finished
* Perimeters
* Solid infill (Linear) and Partial Infill (Linear, Rectilinear,Trianlge, and Cubic)
* Roof and Floors
* Fan Control
* Speed Control
* Acceleration control
* Temperature Control
* Combinable/Chainable Settings Files
* STL File format
* Layer slow down for small layers
* Many other feature ( see settings file for more information)
* 
###In Progress
* 3MF file format
  * Fails with multiple drives
  * Only loads 1 model from the file
* Time Calculations
  * They are not very accurate, underestimates the time currently
* Plugins/Mods
  * Traits have been started but no loading yet.
* Arc Optimization
  * Started but buggy and not active

###Coming Shortly
* Support Generation
* Brim support
* Perimeter start options
* Percentage Complete Gcode
* Handle Errors Better

###Supported Printers
* I only own a Prusa Mk3
* Other printer can easily be added


# Dependencies

This project uses Cargo as the build engine so the dependencies can be found in Cargo.toml file. All dependencies should be compatible with LGPL license. 