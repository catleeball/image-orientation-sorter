#[macro_use]
extern crate log;
extern crate stderrlog;

use arraystring::{ArrayString, typenum::U4};
use aho_corasick::AhoCorasickBuilder;
use image::image_dimensions;
use failure::Error;
use smartstring::alias::String;
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
    make_output_orientation_dirs(&opts)?;
    let src_dest_map = read_files(&opts);
    debug!("File sources and destinations: {:#?}", src_dest_map);
    move_files(src_dest_map);
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
    if !opts.rename {
        let outstr = opts.output_dir.to_str().unwrap_or("");
        create_dir_all(format!("{}/{}", outstr, Orientation::Tall.to_arrstr()))?;
        create_dir_all(format!("{}/{}", outstr, Orientation::Wide.to_arrstr()))?;
        create_dir_all(format!("{}/{}", outstr, Orientation::Square.to_arrstr()))?;
    }
    Ok(())
}

/// Move files to new destination, suppress all errors, overwrite dest files, return no metadata.
fn move_files(mut src_dest_map: Vec<(PathBuf, PathBuf)>) -> () {
    src_dest_map.drain(..)
                .filter_map( |sd| rename(Path::new(&sd.0), Path::new(&sd.1)).ok() )
                .collect()
}

/// Recursively walk input directory, return a vector of image source paths to destination paths.
fn read_files(opts: &Opt) -> Vec<(PathBuf, PathBuf)> {
    let max_depth: usize = match opts.recursive {
        true => 255,
        false => 1,
    };
    WalkDir::new(&opts.input_dir)
        .min_depth(1)
        .max_depth(max_depth)
        .into_iter()
        .filter_entry( |inpath| has_image_extension(inpath.path()) )
        .filter_map( |inpath| inpath.ok() )
        .filter_map( |inpath| get_src_dest_paths(inpath.path(), opts.output_dir.to_owned()).ok() )
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
            outpath.push(Orientation::Wide.to_arrstr().as_str());
            outpath.push(imgfile);
            Ok( (inpath.to_path_buf(), outpath) )
        },
        Ordering::Less => {
            outpath.push(Orientation::Tall.to_arrstr().as_str());
            outpath.push(imgfile);
            Ok( (inpath.to_path_buf(), outpath) )
        }
        Ordering::Equal => {
            outpath.push(Orientation::Square.to_arrstr().as_str());
            outpath.push(imgfile);
            Ok( (inpath.to_path_buf(), outpath) )
        }
    }
}

/// Return true if the given path has an image file extension.
fn has_image_extension(path: &Path) -> bool {
    let extension = match path.extension() {
        Some(extension) => match extension.to_str() {
            Some(extension) => extension,
            None => return false
        },
        None => return false
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
    ac.is_match(extension)
}
