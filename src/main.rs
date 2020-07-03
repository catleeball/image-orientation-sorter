#[macro_use]
extern crate log;
extern crate stderrlog;

use aho_corasick::AhoCorasickBuilder;
use image::image_dimensions;
use failure::Error;
use rayon::prelude::*;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use walkdir::WalkDir;

const IMG_EXTS: [&str; 8] = ["gif", "jpeg", "ico", "png", "tiff", "webp", "bmp", "jpeg_rayon"];

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
    let imgs = read_files(opt.input_dir, opt.output_dir, opt.recursive);
    debug!("{:?}", imgs);
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
    trace!("Options initialized: {:?}", opt);
    return opt
}

/// Recursively walk input directory, return a vector of paths to image files.
fn read_files(input_path: PathBuf, output_path: PathBuf, recursive: bool) -> Vec<(PathBuf, PathBuf)> {
    trace!("Walking directory tree starting at {}", input_path.display());
    let max_depth: usize = match recursive {
        true => 255,
        false => 1,
    };
    WalkDir::new(input_path)
        .min_depth(1)
        .max_depth(max_depth)
        .into_iter()
        .filter_entry( |inpath| has_image_extension(inpath.path()) )
        .filter_map( |inpath| inpath.ok() )
        .map( |inpath| get_src_dest_paths(inpath.path(), output_path.to_owned()) )
        .collect()
}

/// Create destination path: (output path + orientation dir).
fn get_src_dest_paths(inpath: &Path, mut outpath: PathBuf) -> (PathBuf, PathBuf) {
    let imgfile = inpath.file_name().unwrap();
    let (x, y) = image_dimensions(inpath).ok().unwrap();
    match x.cmp(&y) {
        Ordering::Greater => {
            outpath.push("wide");
            outpath.push(imgfile);
            (inpath.to_path_buf(), outpath)
        },
        Ordering::Less => {
            outpath.push("tall");
            outpath.push(imgfile.to_os_string());
            (inpath.to_path_buf(), outpath)
        }
        Ordering::Equal => {
            outpath.push("square");
            outpath.push(imgfile);
            (inpath.to_path_buf(), outpath)
        }
    }
}

/// Return true if the given path has an image file extension.
fn has_image_extension(path: &Path) -> bool {
    let ext = match path.extension() {
        Some(ext) => match ext.to_str() {
            Some(ext) => ext,
            None => return false
        },
        None => return false
    };
    let ac = AhoCorasickBuilder::new()
        .ascii_case_insensitive(true)
        .build(&IMG_EXTS);
    ac.is_match(ext)
}