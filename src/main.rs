#[macro_use]
extern crate log;
extern crate stderrlog;

use arraystring::{ArrayString, typenum::U4};
use aho_corasick::AhoCorasickBuilder;
use image::image_dimensions;
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
            Orientation::Tall =>   unsafe {FourChar::from_str_unchecked("tall")},
            Orientation::Wide =>   unsafe {FourChar::from_str_unchecked("wide")},
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

fn main() -> std::io::Result<()> {
    let opts: Opt = init();
    if !opts.rename {
        create_orientation_dirs(&opts)?;
    } else {
        drop(&opts.output_dir);
    };
    let images = image_paths(&opts);
    let dests = get_dsts(&opts, &images);
    let num_moved = mv_files(&images, dests);
    if !opts.quiet {
        println!("Processed {} files successfully.", num_moved);
    }
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
fn create_orientation_dirs(opts: &Opt) -> std::io::Result<()> {
    // TODO: Instead of panicking when output dirs cannot be written, prompt user
    //       asking if they would like to rename in-place instead. Output error messages,
    //       and kindly suggest to the user to chown dirs with permission errors.
    let outstr = opts.output_dir.to_str().unwrap_or("");
    create_dir_all(format!("{}/{}", outstr, Orientation::Tall.to_arrstr()))?;
    create_dir_all(format!("{}/{}", outstr, Orientation::Wide.to_arrstr()))?;
    create_dir_all(format!("{}/{}", outstr, Orientation::Square.to_arrstr()))?;
    Ok(())
}

/// Walk the input directory, possibly recursively, and return paths of image files.
fn image_paths(opts: &Opt) -> Vec<PathBuf> {
    let max_depth: usize = match opts.recursive {
        true => 255,
        false => 1,
    };
    // If the supplied input path is a file, operate on it alone.
    let min_depth: usize = match opts.input_dir.is_dir() {
        true => 1,
        false => 0,
    };
    WalkDir::new(&opts.input_dir)
        .min_depth(min_depth)
        .max_depth(max_depth)
        .into_iter()
        .filter_map( |dir| dir.ok() )
        .filter( |dir| {
            trace!("Checking extension for {:?}", dir ) ;
            has_image_extension(dir.path())
        })
        .map( |dir| dir.into_path() )
        .collect()
}

/// Given a set of image paths, find where they should be moved to (including in-place renaming).
///
/// Source files with a destination of None will not be acted upon.
fn get_dsts(opts: &Opt, imgs: &Vec<PathBuf>) -> Vec<Option<PathBuf>> {
    imgs.iter()
        .map(|img| dst_path(opts, img))
        .collect()
}

/// Find destination path based on image orientation.
fn dst_path(opts: &Opt, img_path: &Path) -> Option<PathBuf> {
    let imgfile = img_path.file_name().unwrap();
    let ori: Orientation = match image_orientation(img_path) {
        Some(ori) => ori,
        None => return None
    };
    match opts.rename {
        true => match prepend_orientation(img_path) {
            Some(renamed) => {
                trace!("Rename {:?} to {:?}", img_path, renamed);
                Some(renamed)
            },
            None => return None
        },
        false => {
            let mut out = opts.output_dir.to_owned();
            out.push(ori.to_arrstr().as_str());
            out.push(imgfile);
            trace!("Move {:?} to {:?}", img_path, out);
            Some(out)
        },
    }
}

/// Iterate source and destination path vectors, moving matching indexes.
fn mv_files(src_paths: &Vec<PathBuf>, dst_paths: Vec<Option<PathBuf>>) -> u16 {
    if src_paths.len() != dst_paths.len() {
        // TODO: While this hopefully won't happen if all errors are propagated forward as Nones,
        //       consider writing function to normalize src & dst vectors based on path similarity.
        // TODO: Be sure to test this case when writing tests.
        panic!("Source files do not match calculated destination files.");
    }
    src_paths
        .iter()
        .zip(dst_paths.iter())
        .filter_map( |sd| {
            match sd.1.is_none() {
                true => None,
                false => Some(sd),
            }
        })
        .map( |sd| {
            trace!("Attempting fs::rename on {:?}", sd);
            match rename(&sd.0, sd.1.as_ref().unwrap() ) {
                Ok(_) => 1,
                Err(e) => {
                    error!("Failed to move\n  {:?}\nto\n  {:?}\nError: {:?}.", sd.0, sd.1, e);
                    0
                }
            }
        })
        .fold(0, |acc, ret| acc + ret)
}

/// Determine the orientation of an image.
fn image_orientation(img_path: &Path) -> Option<Orientation> {
    let (x, y) = match image_dimensions(img_path) {
        Ok(xy) => {xy},
        Err(e) => {
            warn!("Error finding orientation of image: {:?}. Image will not be moved or renamed. Error: {:?}", img_path, e);
            return None
        }
    };
    match x.cmp(&y) {
        Ordering::Less    => { Some(Orientation::Tall) },
        Ordering::Greater => { Some(Orientation::Wide) },
        Ordering::Equal   => { Some(Orientation::Square) },
    }
}

/// Prepend the image orientation to its filename.
fn prepend_orientation(p: &Path) -> Option<PathBuf> {
    let mut name = p.to_owned();
    let ori = match image_orientation(p) {
        Some(ori) => ori,
        None => return None
    };
    name.set_file_name(
        format!(
            "{}_{}",
            ori.to_arrstr().as_str(),
            p.file_name().unwrap().to_str().unwrap()
        )
    );
    Some(name)
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
