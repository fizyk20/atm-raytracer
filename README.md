# Atmospheric Raytracer

## What is it?

It's a utility for generating panoramas out of elevation maps. You input a set of elevation maps in
the DTED format, latitude and longitude of the viewing location, viewing altitude, direction, field
of view, and it generates a panorama in the PNG format.

It was created as a means for debunking flat-earthers' arguments, so it supports generating
panoramas as they would look if the Earth was flat, as well. Thanks to this feature, you can
simulate a view of a specific location, compare it to actual photos and see which model fits better
;)

## Usage

Typical usage would be:

1. Download some DTED elevation map files, for example from https://earthexplorer.usgs.gov/
2. Put the DTED files in a single folder
3. Run `cargo run --release -- PARAMETERS`, where the parameters are:

* `-t, --terrain PATH` - path to a folder containing files in DTED format

View options:

* `-a, --alt ALT` - viewpoint altitude in meters ASL
* `-l, --lat DEG` - viewpoint latitude in degrees
* `-g, --lon DEG` - viewpoint longitude in degrees
* `-d, --dir DEG` - viewing direction azimuth in degrees (0 = north, 90 = east etc.)
* `-e, --elev DEG` - viewing direction elevation in degrees; 0 means the observer's eye looks horizontally, -1 is one degree below horizontal etc.
* `-f, --fov DEG` - horizontal field of view in degrees (the vertical FOV is scaled based on the output image width and height)
* `-m ,--maxdist DIST` - cutoff distance - objects further than this won't be visible

Simulation environment options:

* `-R, --radius RADIUS` - the Earth's simulated radius, conflicts with `--flat`
* `--flat` - simulate a flat Earth; conflicts with `--radius`
* `--atmosphere PATH` - path to a file describing the atmosphere configuration; to be documented
* `-s, --straight` - propagate light rays along straight lines (by default the rays are bent according to the atmospheric temperature and pressure)
* `--step STEP` - when simulating a light ray, a single simulation step will be by this many meters

Output options:

* `-o, --output PATH` - the resulting image will be saved under this name
* `-w, --width PIXELS` - the output image width in pixels
* `-h, --height PIXELS` - the output image height in pixels
