# Changelog

## [Unreleased]
### New Features
- Sub settings ( Acceleration.Bridge) can be set independently now
- Add in support for Math expressions in the Gcode file
- Added a profile for the Voron0.1
- Add support for aux fans
- `max_temp` setting so printers can specify their max hotend temperature
- Added a profile for the Bambu Lab x1c
- Added a profile for Bambu Lab PLA
- Added a json schema
- Json can be passed into the command line using the -S command
- Other Settings files in settings files are now in reference to that files path
- Add simple-input command
- Add command to output settings
- Settings now are outputted to gcode
 

### Fixes

## [0.4.0]
### New Features
- Added more instruction sets (before/after layer change and object change)
- Formalized instruction set replacements
- Added more move types
- Added variable extrusion width based on move type
- Added Setting validator that check for common issues with settings
- Added warnings
- Added Acceleration, Feedrate, and Jerk settings
- Added Retraction Wipe
- Added Solid Infill types
- Added bounds checking
- Added Binary Optimization for delay and object change
- Massive refactor of tower slicing logic
- Upgrade dependencies
- Change Polygon libraries 

### Fixes
- Fixed issue when slicing multiple objects and outputting bincode overlapping and conflicting 
- Removed all unwraps. Converted some to errors and some to expect. deny(clippy::unwrap_used) is added to prevent unwrap being added in future
- Added checks for models and moves to make sure they don't go out of bounds
- Skirt won't leave bounds now
- Fixed issue where retraction speed wasn't set correctly

## [0.3.0]
### New Features
- Support feature for the GUI
  - A sub-crate was created to handle shared types for the GUI
  - Added Messages mode that outputs messages in Bincode for IPC
  - Console hidden
  - Shared Library is documented
  - All Common errors should now return slicer error rather than panicking
- Added ABS and TPU settings files

### Fixes
- Other files for settings are now relative to the main settings file
- File extensions are case-insensitive now
- Added fix for winding order issue 
- Verbose command line parameter now works
- Setting work on Linux where files are case sensitive
- Fixed spelling errors in settings files

## [0.2.0]
### New Features
- Updated 3MF support to include multiple object and multi model loading
- Added brim support ( optional setting brim_width)
- Added settings dependent on Layer height
- Added temp tower example
- Infill hugs walls now to reduce retractions
- Basic Supports where added ( Experimental Only )
- Reorganization code to better support plugins in the future
- Layer Shrink feature added ( Useful for expanding filaments or on first layer to combat elephants foot)
- Added Lightning Infill ( Experimental Only )

### Fixes
- Fixed issue with the optimizer removing state changes before short moves
- Fixed issue with monotone panicking
- Fixed issue with 0 fill prints
- Fixed Issue with large and small amounts of infill layers

## [0.1.1]
- Release to include binaries 

## [0.1.0]
- Initial Release
