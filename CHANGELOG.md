# Changelog

## [Unreleased]
### New Features
- Added more instruction sets (before/after layer change and object change)
- Formalized instruction set replacements
- Added more move types
- Added variable extrusion width based on move type
### Fixes


## [0.3.0]
### New Features
- Support feature for the GUI
  - A sub-crate was created to handle shared types for the GUI
  - Added Messages mode that outputs messages in Bincode for IPC
  - Console hidden
  - Shared Library is documented
  - All Common errors should now return slicer error rather than panicing
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
- Fixed issue with monotone panicing
- Fixed issue with 0 fill prints
- Fixed Issue with large and small amounts of infill layers

## [0.1.1]
- Release to include binaries 

## [0.1.0]
- Initial Release