# Changelog



## [Unreleased]
### New Features
- Updated 3MF support to include multiple object and multi model loading
- Added brim support ( optional setting brim_width)
- Added settings dependent on Layer height
- Added temp tower example
- Infill hugs walls now to reduce retractions
- Basic Supports where added ( Experimental Only )
- Reorganization code to better support plugins in the future
- Layer Shrink feature added ( Useful for expanding filaments or on first layer to combat elephants foot)

### Fixes
- Fixed issue with the optimizer removing state changes before short moves
- Fixed issue with monotone panicing
- Fixed issue with 0 fill prints
- Fixed Issue with large and small amounts of infill layers

## [0.1.1]
- Release to include binaries 

## [0.1.0]
- Initial Release