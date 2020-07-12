#[macro_use]
extern crate log;
extern crate stderrlog;

use arraystring::{ArrayString, typenum::U4};
use aho_corasick::AhoCorasickBuilder;
use image::image_dimensions;
use failure::Error;
use std::cmp::Ordering;
use std::fs::{create_dir_all, rename};
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use walkdir::WalkDir;

type FourChar = ArrayString<U4>;

enum Orientation { Tall, Wide, Square }
impl Orientation {
    fn to_arrstr(&self) -> FourChar {
        match self {
            Orientation::Tall => unsafe {FourChar::from_str_unchecked("tall")},
            Orientation::Wide => unsafe {FourChar::from_str_unchecked("wide")},
            Orientation::Square => unsafe {FourChar::from_str_unchecked("sqr")},
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "imgorisort", about = "Image Orientation Sorter")]
struct Opt {
    #[structopt(parse(from_os_str), help = "Directory containing image files to sort by orientation.")]
    input_dir: PathBuf,
    #[structopt(parse(from_os_str), default_value = ".", help = "Directory to output sorted images into.")]
    output_dir: PathBuf,
    #[structopt(short, long, help = "Recurse into subdirectories.")]
    recursive: bool,
    #[structopt(long, help = "Prepend image orientation to filename instead of moving file.")]
    rename: bool,
    #[structopt(short, long, parse(from_occurrences), help = "Increase output verbosity by adding more flags: [-v|-vv|-vvv]")]
    verbose: usize,
    #[structopt(short, long, help = "Do not print anything to stdout or stderr.")]
    quiet: bool,
}

fn main() -> Result<(), Error> {
    let opts: Opt = init();
    if !opts.rename { make_output_orientation_dirs(&opts)?; } 
    let num_moved = iterate_files(&opts);
    if !opts.quiet { println!("Processed {} files successfully.", num_moved) }
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

/// Create directories to place each orientation of image into.
fn make_output_orientation_dirs(opts: &Opt) -> Result<(), Error> {
    let outstr = opts.output_dir.to_str().unwrap_or("");
    create_dir_all(format!("{}/{}", outstr, Orientation::Tall.to_arrstr()))?;
    create_dir_all(format!("{}/{}", outstr, Orientation::Wide.to_arrstr()))?;
    create_dir_all(format!("{}/{}", outstr, Orientation::Square.to_arrstr()))?;
    Ok(())
}

/// Walk intput dir and move files to output dir. Quietly ignore all errors.
fn iterate_files(opts: &Opt) -> u32 {
    let max_depth: usize = match opts.recursive {
        true => 255,
        false => 1,
    };
    WalkDir::new(&opts.input_dir)
        .min_depth(1)
        .max_depth(max_depth)
        .into_iter()
        .filter_entry( |inpath| {
            trace!("Checking img extension for {}", inpath.path().to_str().unwrap_or("") ) ;
            has_image_extension(inpath.path())
        })
        .filter_map( |inpath| inpath.ok())
        .filter_map( |inpath| {
            if opts.rename {
                trace!("Finding rename path for {}", inpath.path().to_str().unwrap_or("") );
                get_src_dest_paths(&opts, inpath.path()).ok()
            } else {
                trace!("Finding move path for {}", inpath.path().to_str().unwrap_or("") );
                get_src_dest_paths(&opts, inpath.path()).ok()
            }})
        .filter_map( |sd| {
            trace!("Attempting fs::rename on {:?}", sd);
            rename(&sd.0, &sd.1).ok()
        })
        .fold(0, |i, _| i + 1)
}

/// Prepend the image orientation to its filename.
fn prepend_orientation(p: &Path) -> Result<PathBuf, std::io::ErrorKind> {
    let mut name = p.to_owned();
    let ori = match image_orientation(p) {
        Ok(ori) => ori,
        Err(_) => {
            warn!("Dimensions not found in {}.", p.display());
            return Err(std::io::ErrorKind::InvalidData);
        }
    };
    name.set_file_name(
        format!(
            "{}_{}",
            ori.to_arrstr().as_str(),
            p.file_name().unwrap().to_str().unwrap()
        )
    );
    Ok(name.to_path_buf())
}

fn image_orientation(img_path: &Path) -> Result<Orientation, std::io::ErrorKind> {
    let (x, y) = match image_dimensions(img_path) {
        Ok(xy) => {xy},
        Err(_) => return Err(std::io::ErrorKind::InvalidData),
    };
    match x.cmp(&y) {
        Ordering::Greater => { Ok(Orientation::Wide)   },
        Ordering::Less    => { Ok(Orientation::Tall)   },
        Ordering::Equal   => { Ok(Orientation::Square) },
    }
}

/// Find destination path based on image orientation.
fn get_src_dest_paths(opts: &Opt, inpath: &Path) -> Result<(PathBuf, PathBuf), std::io::ErrorKind> {
    // TODO: Use relative dest paths when dest is inside src dir.
    // TODO: Maybe make separate get src & get dest functions, zip them, then operate on them for extensibility.
    let imgfile = match inpath.file_name() {
        Some(imgfile) => imgfile,
        None => {
            debug!("Not found. {}", inpath.display());
            return Err(std::io::ErrorKind::InvalidInput);
        },
    };
    let ori: Orientation = match image_orientation(inpath) {
        Ok(ori) => ori,
        Err(e) => {
            warn!("Image orientation matching error {:?}", e);
            return Err(e)
        },
    };
    let out = match opts.rename {
        true => match prepend_orientation(inpath) {
            Ok(prepended_inpath) => {
                trace!("Rename {:?} to {:?}", inpath, prepended_inpath);
                prepended_inpath},
            Err(e) => {
                warn!("Error prepending orientation to filename {:?}, error: {:?}", inpath, e);
                return Err(e)
            },
        },
        false => {
            let mut out = opts.output_dir.to_owned();
            out.push(ori.to_arrstr().as_str());
            out.push(imgfile);
            trace!("Move {:?} to {:?}", inpath, out);
            out
        },
    };
    Ok( (inpath.to_path_buf(), out) )
}

/// Return true if the given path has an image file extension.
fn has_image_extension(path: &Path) -> bool {
    let extension = match path.extension() {
        Some(extension) => match extension.to_str() {
            Some(extension) => {
                trace!("{:?} has extension {:?}", path, extension);
                extension
            },
            None => {
                trace!("{:?} has no extension str.", path);
                return false
            },
        },
        None => {
            trace!("{:?} has no extension.", path); return false
        }
    };
    let ac = unsafe {
        AhoCorasickBuilder::new()
            .dfa(true)
            .byte_classes(false)
            .ascii_case_insensitive(true)
            .build(&[
                FourChar::from_str_unchecked("jpg"),
                FourChar::from_str_unchecked("jpeg"),
                FourChar::from_str_unchecked("png"),
                FourChar::from_str_unchecked("gif"),
                FourChar::from_str_unchecked("webp"),
                FourChar::from_str_unchecked("ico"),
                FourChar::from_str_unchecked("tiff"),
                FourChar::from_str_unchecked("bmp"),]) };
    trace!{"Image extension matched: {:?}", ac.is_match(extension)}
    ac.is_match(extension)
}
