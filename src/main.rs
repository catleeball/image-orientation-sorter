#[macro_use]
extern crate log;
extern crate stderrlog;

use aho_corasick::AhoCorasickBuilder;
use clap::arg_enum;
use image::image_dimensions;
use failure::Error;
use std::cmp::Ordering;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use walkdir::WalkDir;

const IMG_EXTS: [&str; 8] = ["gif", "jpeg", "ico", "png", "tiff", "webp", "bmp", "jpeg_rayon"];

arg_enum! {
    #[derive(Debug)]
    enum OverwriteBehavior {
        Append,
        Overwrite,
        Skip,
    }
}

#[derive(Debug)]
struct Img {
    src: PathBuf,
    dst: PathBuf,
}

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
    #[structopt(possible_values = &OverwriteBehavior::variants(), case_insensitive = true, help = "Specify behavior when a file with the same name exists in the output directory. Possible options: [append (adds a number to the end of the filename, keeping both files), overwrite (replace file in destination directory), skip (do not move file, leave in original location.)]")]
    overwrite: OverwriteBehavior,
}

fn main() -> Result<(), Error> {
    let opts: Opt = init();
    let src_dest_map: Vec<Img> = read_files(opts.input_dir, opts.output_dir, opts.recursive);
    debug!("{:?}", src_dest_map);
    if !opts.quiet { println!("Operation complete!"); }
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

// fn move_files(mut src_dest_map: Vec<Img>, overwrite: OverwriteBehavior) -> Result<(), Error> {
//     src_dest_map.iter()
//                 .map(|img| fs::rename());
//     Ok(())
// }

/// Recursively walk input directory, return a vector of image source paths to destination paths.
fn read_files(input_path: PathBuf, output_path: PathBuf, recursive: bool) -> Vec<Img> {
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
        .filter_map( |inpath| get_src_dest_paths(inpath.path(), output_path.to_owned()).ok() )
        .map( |(srcpath, dstpath)| Img { src: srcpath, dst: dstpath } )
        .collect()
}

/// Find destination path based on image orientation.
fn get_src_dest_paths(inpath: &Path, mut outpath: PathBuf) -> Result<(PathBuf, PathBuf), std::io::ErrorKind> {
    let imgfile = match inpath.file_name() {
        Some(imgfile) => imgfile,
        None => {
            debug!("Recoverable error: Could not find filename for source image path. {}", inpath.display());
            return Err(std::io::ErrorKind::InvalidInput);
        },
    };
    let (x, y) = match image_dimensions(inpath) {
        Ok(xy) => xy,
        Err(e) => {
            warn!("Could not parse dimensions of image {}. Error {}", inpath.display(), e);
            return Err(std::io::ErrorKind::InvalidData);
        }
    };
    match x.cmp(&y) {
        Ordering::Greater => {
            outpath.push("wide");
            outpath.push(imgfile);
            Ok( (inpath.to_path_buf(), outpath) )
        },
        Ordering::Less => {
            outpath.push("tall");
            outpath.push(imgfile);
            Ok( (inpath.to_path_buf(), outpath) )
        }
        Ordering::Equal => {
            outpath.push("square");
            outpath.push(imgfile);
            Ok( (inpath.to_path_buf(), outpath) )
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

// /// Removing; this is covered in the filter mapping in get_src_dest_paths()'s filter_map()s.
// fn validate_input_paths(src: PathBuf, dst: PathBuf) -> Result<(), Error> {
//     if !src.exists() {
//         error!("Source path does not exist, or is not readable. {}", src.display());
//         std::process::exit(1);
//     }
//     if !dst.exists() {
//         debug!("Destination path not found, creating {}.", dst.display());
//         fs::create_dir_all(dst)?
//     }
//     Ok(())
// }