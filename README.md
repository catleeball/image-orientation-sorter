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

### Move images

Move images to 'tall', 'wide', and 'square' directories.

```
$ tree /path/to/images
/path/to/images
├── foo
│   └── square.jpg
├── portrait.jpg
└── landscape.jpg

$ imgorisort -r --rename /path/to/images
Processed 3 files successfully.

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

### Rename images

Rename images, adding 'tall', 'square', or 'wide' to the beginning of the filename.

Using --rename will not move images to new directories, in contrast to the above example.

```
$ tree /path/to/images
/path/to/images
├── foo
│   └── square.jpg
├── portrait.jpg
└── landscape.jpg

$ imgorisort -r /path/to/images
Processed 3 files successfully.

$ tree /path/to/images
/path/to/images
├── foo
│   └── sqr_square.jpg
├── tall_portrait.jpg
└── wide_landscape.jpg
```

# Usage

Run `imgorisort --help` to see the usage text:

```
imgorisort 0.2.0
Image Orientation Sorter

USAGE:
    imgorisort [FLAGS] <input-dir> [output-dir]

FLAGS:
    -h, --help         Prints help information
    -q, --quiet        Do not print anything to stdout or stderr.
    -r, --recursive    Recurse into subdirectories.
        --rename       Prepend image orientation to filename instead of moving file.
    -V, --version      Prints version information
    -v, --verbose      Increase output verbosity by adding more flags: [-v|-vv|-vvv]

ARGS:
    <input-dir>     Directory containing image files to sort by orientation.
    <output-dir>    Directory to output sorted images into. [default: .]
```