# Image Duplicates

A simple FLTK-based GUI that scans a directory for visually similar images
according to their perceptual hash. After scanning, the GUI starts, displaying
each pair of images that fall within the similarity threshold. The user can then
select which image to keep, sending the other to the system trash. The user can
also keep both images if desired. Because calculating a large number of
perceptual hashes is computationally expensive, the program caches hash results
to a file, which is automatically updated when the user runs the program again.

![](https://github.com/4ffy/image-duplicate/blob/main/data/screenshot.png)

## Usage

`Usage: image-duplicate [OPTIONS] <PATH>`

See `image-duplicate --help` for details.

By default, the program loads an existing hash database if present, then scans
the directory for changes, removing any entries for files that no longer exist
and hashing any new images. The program then dumps the hash database to the
target directory, finds similar images, and starts the GUI for their handling.

Because calculating a large number of perceptual hashes is slow, the program
tries to speed up the process by hashing a number of images in parallel. By
default, this process uses as many threads on the system as possible. This can
be controlled via the `RAYON_NUM_THREADS` environment variable.

## Todo

 - Add a config file to control similarity threshold and database handling and
   such.
 - Support an ignore file so skip over known-similar images that I want to keep
   around.
 - Use `XDG_CACHE_HOME` or platform equivalent to store the cached hashes rather
   than putting them in the image directory. This requires some sort of path
   encoding.
 - Add a file browser or something so that the program doesn't have to be
   started from the command line.
 - Figure out if this even works on other platforms.

## Building

`cargo build`, I should hope. This program relies on bundled FLTK provided by
the `fltk-rs` crate, so you also need a C++ compiler usable by Cargo.

## Disclaimer

This is a personal program, uploaded because it could be useful to someone else.
This comes with the usual caveats. Do you trust me?

## License

GPLv3+. See LICENSE for details.
