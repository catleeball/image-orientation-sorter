# image-orientation-sorter
Quickly sort images into directories by orientation: tall, wide, and square.

Supported filetypes:
- jpg
- png
- gif
- webp
- ico
- tiff
- bmp

This small tool's first release includes only the basic functionality: move images to direcotires named after their orientations.

# Example
```
$ tree /path/to/images
/path/to/images
├── foo
│   └── square.jpg
├── portrait.jpg
└── landscape.jpg
$ imgorisort -r /path/to/images
$ tree /path/to/images
/path/to/images
├── foo
├── wide
│   └── landscape.jpg
├── tall
│   └── portrait.jpg
└── sqr
    └── square.jpg
```

# Usage
```
imgorisort 0.1.2
Image Orientation Sorter

USAGE:
    imgorisort [FLAGS] <input-dir> [output-dir]

FLAGS:
    -h, --help         Prints help information
    -q, --quiet        Do not print anything to stdout or stderr.
    -r, --recursive    Recurse into subdirectories.
    -V, --version      Prints version information
    -v, --verbose      Increase output verbosity by adding more flags: [-v|-vv|-vvv]

ARGS:
    <input-dir>     Directory containing image files to sort by orientation.
    <output-dir>    Directory to output sorted images into. [default: .]
```