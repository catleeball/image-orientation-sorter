# image-orientation-sorter
Sort images in a directory by orientation: landscape, portrait, or square.

# Why?
I downloaded vertical and horizontal desktop wallpapers, but didn't sort them as I saved them. I want a vertical folder for my tall monitor and a horizontal folder for my wide monitor.

# Example
```
$ imageorisort /path/to/images .
Processed X landscapes, Y portraits, Z squares.
$ tree /path/to/images
images
├── landscape
│   └── wide.jpg
├── portrait
│   └── tall.jpg
└── square
```

# Usage
```
Image Orientation Sorter

USAGE:
    imgorisort [FLAGS] [OPTIONS] <input-dir> [output-dir]

FLAGS:
    -c, --copy            Copy (rather than move) images to output directory, sorted by orientation. Subdirectories
                          [portrait, landscape, square] may be created in this directory.
    -d, --dry-run         Do not actually move or copy any files. Print files to stdout unless --quiet is present.
    -h, --help            Prints help information
    -p, --prefix          Prepend 'portrait_', 'landscape_', or 'square_' to output image filenames.
    -q, --quiet           Do not print anything to stdout or stderr.
        --read-headers    Guess if a file is an image based on file header rather than file extension. Performs more
                          slowly than reading extensions.
    -r, --recursive       Recurse into subdirectories.
        --rename          Rename files without moving them, prepending 'portrait_', 'landscape_', or 'square_' to the
                          filename. If this option is present, ignore -c, -p, and output_dir.
    -V, --version         Prints version information
    -v, --verbose         Increase output verbosity by adding more flags: [-v|-vv|-vvv]

OPTIONS:
        --overwrite <overwrite>    Specify behavior when a file with the same name exists in the output directory.
                                   [default: rename]  [possible values: rename, overwrite, skip]

ARGS:
    <input-dir>     Directory containing image files to sort by orientation.
    <output-dir>    Directory to output sorted images into. [default: .]
```