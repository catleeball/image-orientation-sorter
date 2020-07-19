#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
extern crate stderrlog;

use aho_corasick::{AhoCorasick, AhoCorasickBuilder};
use arraystring::{ArrayString, typenum::U4};
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

lazy_static! {
    static ref AC: AhoCorasick = unsafe {
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
                FourChar::from_str_unchecked("bmp"),])
    };
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
    #[structopt(short, long, parse(from_occurrences), help = "Increase output verbosity by adding more flags: [-v|-vv|-vvv|-vvvv|-vvvvv]")]
    verbose: usize,
    #[structopt(short, long, help = "Do not print anything to stdout or stderr.")]
    quiet: bool,
    #[structopt(long, help = "Overrwite files in the destination directory if file names are the same. Without this flag set, the default behavior is to append a number to make the filename unique.")]
    overwrite: bool,
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
    let moved: u32 = mv_files(&images, dests, &opts);
    if !opts.quiet {
        println!("Processed {} files successfully.", moved);
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
        .filter( |dir| dir.file_type().is_file() && has_image_extension(dir.path()) )
        .map( |dir| dir.into_path() )
        .collect()
}

/// Given a set of image paths, find where they should be moved to (including in-place renaming).
#[inline]
fn get_dsts(opts: &Opt, imgs: &Vec<PathBuf>) -> Vec<Option<PathBuf>> {
    imgs.iter()
        .map(|img| dst_path(opts, img))
        .collect()
}

/// Find destination path based on image orientation.
#[inline]
fn dst_path(opts: &Opt, img_path: &Path) -> Option<PathBuf> {
    let imgfile = img_path.file_name().unwrap();
    let ori: Orientation = match image_orientation(img_path) {
        Some(ori) => ori,
        None => return None
    };
    match opts.rename {
        true => match prepend_orientation(img_path) {
            Some(renamed) => Some(renamed),
            None => return None
        },
        false => {
            let mut out = opts.output_dir.to_owned();
            out.push(ori.to_arrstr().as_str());
            out.push(imgfile);
            if out.as_path() == img_path {
                drop(out);
                debug!("File already in destination, skipping. {:?}", img_path);
                return None
            };
            Some(out)
        },
    }
}

/// Iterate source and destination path vectors, moving matching indexes.
// TODO: Break these long filters/maps into functions.
fn mv_files(src_paths: &Vec<PathBuf>, dst_paths: Vec<Option<PathBuf>>, opts: &Opt) -> u32 {
    if src_paths.len() != dst_paths.len() {
        panic!("Source files do not match calculated destination files.\nSource files: {:?}\nDestinations: {:?}", src_paths, dst_paths);
    }
    src_paths
        .iter()
        .zip(dst_paths.iter())
        .filter( |sd| !sd.1.is_none() )
        .filter_map( |sd| {
            let dst = sd.1.to_owned().unwrap();
            if !opts.overwrite {
                if dst.exists() {
                    Some( (sd.0, make_uniq(dst)) )
                } else {
                    Some( (sd.0, dst) )
                }
            } else {
                Some( (sd.0, dst) )
            }
        })
        .map( |sd| {
            match rename(&sd.0, &sd.1) {
                Ok(_) => {
                    debug!("Moved {:?} to {:?}", &sd.0, &sd.1);
                    1
                },
                Err(e) => {
                    error!("Failed to move\n  {:?}\nto\n  {:?}\nError: {:?}.", sd.0, sd.1, e);
                    0
                }
            }
        })
        .fold(0, |acc, ret| acc + ret)
}

/// Determine the orientation of an image.
#[inline]
fn image_orientation(img_path: &Path) -> Option<Orientation> {
    let (x, y): (u32, u32) = match image_dimensions(img_path) {
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
    let ori: Orientation = match image_orientation(p) {
        Some(ori) => ori,
        None => return None
    };

    let mut new_name = p.to_owned();
    new_name.set_file_name(
        format!(
            "{}_{}",
            ori.to_arrstr().as_str(),
            p.file_name().unwrap().to_str().unwrap()));

    if new_name.as_path() != p {
        if new_name.exists() {
            new_name = make_uniq(new_name);
        }
        trace!("Renamed {:?} to {:?}", p, new_name);
        Some(new_name)
    } else {
        error!("Rename failed. Rename operation produced identical paths. {:?}, {:?}", new_name.file_name(), p.file_name());
        None
    }
}

/// Try to make a filename unique by appending an integer to the end of a filename.
// TODO: Do this smarter and/or allow user to configure alternative suffix (timestamp? uuid?)
#[inline]
#[cold]
fn make_uniq(fpath: PathBuf) -> PathBuf {
    let mut i: u16 = 0;
    let mut new_name: PathBuf = fpath.to_owned();
    while new_name.exists() {
        i += 1;
        new_name.set_file_name(
            format!("{}_{}.{}",
                new_name.file_stem().unwrap().to_str().unwrap(),
                i,
                new_name.extension().unwrap().to_str().unwrap()));
    }
    drop(i);
    trace!("Renamed file to: {:?}", fpath);
    new_name
}

/// Return true if the given path has an image file extension.
#[inline]
fn has_image_extension(path: &Path) -> bool {
    let extension: &str = match path.extension() {
        Some(extension) => match extension.to_str() {
            Some(extension) => extension,
            None => return false,
        },
        None => return false
    };
    let is_img: bool = AC.is_match(extension);
    debug!("{:?} is an image? -> {:?}", path, is_img);
    is_img
}

// Unit tests ====================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile;

    fn test_opts() -> Opt {
        Opt {
            input_dir:  tempfile::tempdir().unwrap().path().to_path_buf(),
            output_dir: tempfile::tempdir().unwrap().path().to_path_buf(),
            recursive:  false,
            rename:     false,
            verbose:    5,
            quiet:      false,
            overwrite:  false,
        }
    }

    #[test]
    fn test_create_orientation_dirs() {
        let opts = test_opts();
        let ret = create_orientation_dirs(&opts);
        assert_eq!(ret.is_ok(), true);
        let dirs_exist: bool =
            Path::new("/tmp/wide").exists() &&
            Path::new("/tmp/tall").exists() &&
            Path::new("/tmp/sqr").exists();
        assert_eq!(dirs_exist, true)
    }
}