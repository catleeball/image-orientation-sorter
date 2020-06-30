#[macro_use]
extern crate log;
extern crate stderrlog;

use aho_corasick::AhoCorasickBuilder;
use failure::Error;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use walkdir::WalkDir;

#[derive(Debug, StructOpt)]
#[structopt(name = "imgorisort", about = "Image Orientation Sorter")]
struct Opt {
    #[structopt(parse(from_os_str), help = "Directory containing image files to sort by orientation.")]
    input_dir: PathBuf,
    #[structopt(parse(from_os_str), help = "Directory to output sorted images into.")]
    output_dir: PathBuf,
    #[structopt(short, long, help = "Recurse into subdirectories.")]
    recursive: bool,
    #[structopt(short = "c", long = "copy", help = "Copy (rather than move) images to output directory, sorted by orientation. Subdirectories [portrait, landscape, square] may be created in this directory.")]
    mv: bool,
    #[structopt(short, long, help = "Prepend 'portrait', 'landscape', or 'square' to output image filenames.")]
    prefix: bool,
    #[structopt(long, help = "Guess if a file is an image based on file header rather than file extension. Performs more slowly than reading extensions.")]
    read_headers: bool,
    #[structopt(short, long, parse(from_occurrences), help = "Increase output verbosity by adding more flags: [-v|-vv|-vvv]")]
    verbose: usize,
    #[structopt(short, long, help = "Do not print anything to stdout or stderr.")]
    quiet: bool,
    #[structopt(short, long, help = "Do not actually move or copy any files. Implies -vvv unless --quiet is present.")]
    dry_run: bool,
}

fn main() -> Result<(), Error>{
    let opt = init();
    let _ = read_files(opt.input_dir, opt.recursive)?;
    if !opt.quiet { println!("Operation complete!"); }
    Ok(())
}

/// Get CLI options, initilize logging.
fn init() -> Opt {
    let opt = Opt::from_args();
    stderrlog::new()
        .module(module_path!())
        .quiet(opt.quiet)
        .verbosity(opt.verbose)
        .init()
        .unwrap();
    trace!("CLI options: {:?}", opt);
    return opt
}

/// Recursively walk given directory, operating on each image file.
fn read_files(input_dir: PathBuf, recursive: bool) -> Result<(), Error> {
    trace!("Walking directory tree starting at {}", input_dir.display());
    let max_depth: usize = match recursive {
        true => 255,
        false => 1,
    };
    let mut images_paths: Vec<&Path> = Vec::new();
    for f in WalkDir::new(input_dir)
        .min_depth(1)    
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|i| i.ok()){
            if has_image_extension(f.file_name().to_str().unwrap_or("")) {
                images_paths.push(f.path().)
            }
        }
    Ok(())
}

fn has_image_extension(path: &str) -> bool {
    if path.is_empty() { return false }
    const IMG_EXTS: [&str; 8] = ["gif", "jpeg", "ico", "png", "tiff", "webp", "bmp", "jpeg_rayon"];
    let ac = AhoCorasickBuilder::new()
        .ascii_case_insensitive(true)
        .build(&IMG_EXTS);
    return ac.is_match(path);
}