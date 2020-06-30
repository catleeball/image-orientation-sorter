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
imgorisort 0.1.0
Image Orientation Sorter

USAGE:
    imgorisort [FLAGS] <input-dir> <output-dir>

FLAGS:
    -d, --dry-run         Do not actually move or copy any files. Implies -vvv unless --quiet is present.
    -h, --help            Prints help information
    -c, --copy            Copy (rather than move) images to output directory, sorted by orientation. Subdirectories
                          [portrait, landscape, square] may be created in this directory.
    -p, --prefix          Prepend 'portrait', 'landscape', or 'square' to output image filenames.
    -q, --quiet           Do not print anything to stdout or stderr.
        --read-headers    Guess if a file is an image based on file header rather than file extension. Performs more
                          slowly than reading extensions.
    -r, --recursive       Recurse into subdirectories.
    -V, --version         Prints version information
    -v, --verbose         Increase output verbosity by adding more flags: [-v|-vv|-vvv]

ARGS:
    <input-dir>     Directory containing image files to sort by orientation.
    <output-dir>    Directory to output sorted images into.
```