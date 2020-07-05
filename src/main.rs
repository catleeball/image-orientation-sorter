#[macro_use]
extern crate log;
extern crate stderrlog;

use aho_corasick::AhoCorasickBuilder;
use clap::arg_enum;
use image::image_dimensions;
use failure::Error;
use smartstring::alias::String;
use std::cmp::Ordering;
use std::fs::{create_dir_all, rename};
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use walkdir::WalkDir;

static IMG_EXTS: [&str; 8] = ["jpg", "jpeg", "png", "gif", "webp", "ico", "tiff", "bmp"];
static ORI: [&str; 3] = ["portrait", "landscape", "square"];
static OVERWRITE_BEHAVIORS: &[&str; 3] = &["rename", "overwrite", "skip"];

#[derive(Debug, StructOpt)]
#[structopt(name = "imgorisort", about = "Image Orientation Sorter")]
struct Opt {
    #[structopt(parse(from_os_str), help = "Directory containing image files to sort by orientation.")]
    input_dir: PathBuf,
    #[structopt(parse(from_os_str), default_value = ".", help = "Directory to output sorted images into.")]
    output_dir: PathBuf,
    #[structopt(short, long, help = "Recurse into subdirectories.")]
    recursive: bool,
    #[structopt(short = "c", long = "copy", help = "Copy (rather than move) images to output directory, sorted by orientation. Subdirectories [portrait, landscape, square] may be created in this directory.")]
    copy: bool,
    #[structopt(short, long, help = "Prepend 'portrait_', 'landscape_', or 'square_' to output image filenames.")]
    prefix: bool,
    #[structopt(long, help = "Rename files without moving them, prepending 'portrait_', 'landscape_', or 'square_' to the filename. If this option is present, ignore -c, -p, and output_dir.")]
    rename: bool,
    #[structopt(long, help = "Guess if a file is an image based on file header rather than file extension. Performs more slowly than reading extensions.")]
    read_headers: bool,
    #[structopt(short, long, parse(from_occurrences), help = "Increase output verbosity by adding more flags: [-v|-vv|-vvv]")]
    verbose: usize,
    #[structopt(short, long, help = "Do not print anything to stdout or stderr.")]
    quiet: bool,
    #[structopt(short, long, help = "Do not actually move or copy any files. Print files to stdout unless --quiet is present.")]
    dry_run: bool,
    #[structopt(long, default_value = OVERWRITE_BEHAVIORS[0], possible_values = OVERWRITE_BEHAVIORS, case_insensitive = true, help = "Specify behavior when a file with the same name exists in the output directory. Possible options: [rename (adds a number to the end of the filename, keeping both files), overwrite (replace file in destination directory), skip (do not move file, leave in original location).]")]
    overwrite: String,
}

fn main() -> Result<(), Error> {
    let opts: Opt = init();
    if opts.dry_run || opts.copy || opts.read_headers || opts.rename {
        exit_no_impl();
    }
    if !opts.dry_run && !opts.copy && !opts.rename {
        make_output_orientation_dirs(&opts)?;
    }
    let src_dest_map = read_files(opts.input_dir, opts.output_dir, opts.recursive);
    debug!("{:?}", src_dest_map);
    move_files(src_dest_map, opts.overwrite)?;
    if !opts.quiet { println!("Operation complete!"); }
    Ok(())
}

fn exit_no_impl() {
    error!("Not yet implemented.");
    std::process::exit(1);
}

fn make_output_orientation_dirs(opts: &Opt) -> Result<(), Error> {
    let outstr = opts.output_dir.to_str().unwrap_or("");
    let tall = format!("{}/{}", outstr, ORI[0]);
    let wide = format!("{}/{}", outstr, ORI[1]);
    let square = format!("{}/{}", outstr, ORI[2]);
    create_dir_all(tall)?;
    create_dir_all(wide)?;
    create_dir_all(square)?;
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

/// Move files based on the supplied OverwriteBehavior.
fn move_files(src_dest_map: Vec<(PathBuf, PathBuf)>, overwrite: &str) -> Result<(), Error> {
    match overwrite {
        "overwrite" => move_files_quiet_overwrite(src_dest_map),
        "rename" => {exit_no_impl();},
        "skip" => {exit_no_impl();},
    }
    Ok(())
}

/// Move files to new destination, suppress all errors, overwrite dest files, return no metadata.
fn move_files_quiet_overwrite(mut src_dest_map: Vec<(PathBuf, PathBuf)>) -> () {
    src_dest_map.drain(..)
                .filter_map( |sd| rename(Path::new(&sd.0), Path::new(&sd.1)).ok() )
                .collect()
}

// fn move_files_overwrite(src_dest_map: Vec<(PathBuf, PathBuf)>) {
//     let mv_errors: Vec<(std::io::Error, &PathBuf, &PathBuf)> = Vec::with_capacity(src_dest_map.len());
//     let (pass, fail): (Vec<_>, Vec<_>) = src_dest_map
//         .iter()
//         .map( |sd| rename(Path::new(&sd.0), Path::new(&sd.1)) )
//         .partition(Result::is_ok);
//     let errors: Vec<_> = fail
//         .iter()
//         .map(|e| e.as_ref().unwrap_err())
//         .collect();
// }

/// Recursively walk input directory, return a vector of image source paths to destination paths.
fn read_files(input_path: PathBuf, output_path: PathBuf, recursive: bool) -> Vec<(PathBuf, PathBuf)> {
    trace!("Walking dir: {}", input_path.display());
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
        .collect()
}

/// Find destination path based on image orientation.
fn get_src_dest_paths(inpath: &Path, mut outpath: PathBuf) -> Result<(PathBuf, PathBuf), std::io::ErrorKind> {
    let imgfile = match inpath.file_name() {
        Some(imgfile) => imgfile,
        None => {
            debug!("Not found. {}", inpath.display());
            return Err(std::io::ErrorKind::InvalidInput);
        },
    };
    let (x, y) = match image_dimensions(inpath) {
        Ok(xy) => xy,
        Err(e) => {
            warn!("Dimensions not found in {}. Error {}", inpath.display(), e);
            return Err(std::io::ErrorKind::InvalidData);
        }
    };
    match x.cmp(&y) {
        Ordering::Greater => {
            outpath.push(ORI[0]);
            outpath.push(imgfile);
            Ok( (inpath.to_path_buf(), outpath) )
        },
        Ordering::Less => {
            outpath.push(ORI[1]);
            outpath.push(imgfile);
            Ok( (inpath.to_path_buf(), outpath) )
        }
        Ordering::Equal => {
            outpath.push(ORI[2]);
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