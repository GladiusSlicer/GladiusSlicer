# Gladius Slicer

An **In-Progress** Slicer for FDM 3D printing written in Rust with a focus on customization and modularity. This is the core application for the GUI see [here](https://github.com/GladiusSlicer/GladiusGUI).

Rust, LGPL. Copyright 2021 Lucas Ince/mrhatman

**Questions?** Please use the github discussion page.

**Want to contribute?** Open a PR. See contributing document for more information.

Gladius Slicer is currently in an alpha state and very little is stable so assume any new release will cause breaking bugs. If you need something stable, please open an issue or discussion, so we can plan out the interface. 

# Usage

This project is a command line application. That GUI project can be found [here](https://github.com/GladiusSlicer/GladiusGUI).

```
USAGE:
    gladius_slicer.exe [FLAGS] [OPTIONS] <INPUT>...

FLAGS:
    -m               Use the Message System (useful for interprocess communication)
    -v               Sets the level of verbosity
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -o <OUTPUT>              Sets the output file to use
    -s <SETTINGS>            Sets the settings file to use
    -j <THREAD_COUNT>        Sets the number of threads to use in the thread pool (defaults to number of CPUs)

ARGS:
    <INPUT>...    Sets the input file to use
```

### Input file examples:
* ` "{\"Auto\":\"test_3D_models\\3DBenchy.stl\"}" ` : Automatically Centers 3DBenchy file in center of the build area
* ` "{\"AutoTranslate\":[\"test_3D_models\\3DBenchy.stl\",-10,0]}" `  : Automatically centers 3DBenchy file in center of the build area offset -10 mm off center in the x dimension.
* ` "{\"AutoTranslate\":[\"test_3D_models\\3DBenchy.stl\",0,-80]} " "{\"AutoTranslate\":[\"test_3D_models\\3DBenchy.stl\",0,80]} " `: print 2 3DBenchy's 
* ` "{\"Raw\":[\"test_3D_models\\3DBenchy.stl\",[[1.0,0.0,0.0,124.0],[0.0,1.0,0.0,105.0],[0.0,0.0,1.0,0.0],[0.0,0.0,0.0,1.0]] }" `:3DBenchy with given transform matrix

### Settings file example
Settings files are hJSON, a human readable json that supports comments. Examples can be found in the settings folder.

# Current Status

 Generated GCode has been used for multiple prints (Benchy, xyz calibration cube, marvin) with a Prusa Mk3 and gives similar results to other slicers. 

### Finished
* Perimeters
* Solid infill (Linear) and Partial Infill (Linear, Rectilinear,Triangle, and Cubic)
* Brim and skirt support
* Roof and Floors
* Fan Control
* Speed Control
* Acceleration control
* Temperature Control
* Combinable/Chainable Settings Files
* STL File format
* Layer slow down for small layers
* Change settings based on layer
* Elephant foot compensation
* Many other feature ( see settings file for more information)
* Retraction Wipe 

### In Progress
* 3MF file format
  * This work should ideally be moved into its own repo/crate so other projects can use the code 
* Time Calculations
  * They are not very accurate, underestimates the time currently
  * Acceleration settings, Command processes time, etc are not current accounted for
* Plugins/Mods
  * Traits have been started but no loading yet.
  * The type definitions are currently part of the shared crate.
* Arc Optimization
  * Started but buggy and not active
* Code Documentation
  * The entire shared library is now documented 
  * Some other main crate code is documented
* Support Generation
  * Supports can be generated but they are poor
* Lightning Infill
  * Added and is functional but being watched for errors
* GUI
  * A GUI is in active development
  * See [here](https://github.com/GladiusSlicer/GladiusGUI)
### Coming Shortly

* Perimeter start options
* Percentage Complete Gcode
* Octoprint integration
* Support for non Merlin firmwares

### Eventually
* Multiple Extruder Support

### Supported Printers
* I only own a Prusa Mk3
* Friend owns a CR10
* Other Marlin firmware printers can easily be added
  * Additional access to community printers will be needed to support other firmwares/G-code flavors


# Dependencies

This project uses Cargo as the build engine so the dependencies can be found in Cargo.toml file. All dependencies should be compatible with LGPL license. 
