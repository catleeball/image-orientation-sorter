# image-orientation-sorter
Sort images in a directory by orientation: landscape, portrait, or square.

# Why?
I downloaded vertical and horizontal desktop wallpapers, but didn't sort them as I saved them. I want a vertical folder for my tall monitor and a horizontal folder for my wide monitor.

# Usage
```
$ imageorisort /path/to/images
Processed X landscapes, Y portraits, Z squares.
$ tree /path/to/images
images
├── landscape
│   └── wide.jpg
├── portrait
│   └── tall.jpg
└── square
```