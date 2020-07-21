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

// ==================== Unit tests ==================== //

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{TempDir, tempdir};
    // use std::panic;
    use image::RgbImage;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn test_opts() -> Opt {
        Opt {
            input_dir:  tempdir().unwrap().path().to_path_buf(),
            output_dir: tempdir().unwrap().path().to_path_buf(),
            recursive:  false,
            rename:     false,
            verbose:    5,
            quiet:      false,
            overwrite:  false,
        }
    }

    /// Initialize module exactly once for all test runs.
    pub fn init() {
        INIT.call_once(|| {
            stderrlog::new()
                .module(module_path!())
                .quiet(false)
                .verbosity(5)
                .init()
                .unwrap();
        });
    }

    /// Create test directory structure.
    /// root
    /// ├── level_one_a
    /// │   └── level_two
    /// │       └── level_two
    /// │           └── level_three
    /// └── level_one_b
    fn test_dir_tree() -> TempDir {
        let root = tempdir().unwrap();
        let rootstr = root.path().to_str().unwrap();
        create_dir_all(format!("{}{}", rootstr, "/level_one_a/level_two/level_three")).unwrap();
        create_dir_all(format!("{}{}", rootstr, "/level_one_b")).unwrap();
        return root
    }

    /// Traverse a directory tree and write_test_images in each directory.
    fn populate_dir_tree(root: &Path) {
        for dir in WalkDir::new(root)
            .min_depth(0)
            .max_depth(5)
            .into_iter()
            .filter_map(|d| d.ok())
            .filter(|d| d.file_type().is_dir())
        {
            write_test_images(
                dir.into_path().to_str().unwrap_or("UNWRAP_DIR_NAME_FAILED_IN_POPULATE_DIR_TREE")
            );
        }
    }

    /// Write a wide, tall, and square image to a given path.
    fn write_test_images<S>(pathstr: S)
    where S: Into<String> + std::fmt::Display
    {
        RgbImage::new(2, 3).save( Path::new( &format!("{}{}", pathstr, "/w.png") ) ).unwrap();
        RgbImage::new(3, 2).save( Path::new( &format!("{}{}", pathstr, "/t.png") ) ).unwrap();
        RgbImage::new(2, 2).save( Path::new( &format!("{}{}", pathstr, "/s.png") ) ).unwrap();
    }

    // fn setup_test() {// Do setup.}
    // fn teardown_test() {// Do teardown.}
    // fn run_test<T>(test: T) -> ()
    // where T: FnOnce() -> () + panic::UnwindSafe {
    //     setup_test();
    //     let result = panic::catch_unwind( || { test() } );
    //     teardown_test();
    //     assert!(result.is_ok())
    // } // https://link.medium.com/bpO6CcH8f8

    #[test]
    fn test_create_orientation_dirs() {
        init();
        let opts = test_opts();
        let ret = create_orientation_dirs(&opts);
        assert_eq!(ret.is_ok(), true);
        let mut wts: (u8, u8, u8) = (0, 0, 0);
        for dir in WalkDir::new(&opts.output_dir).min_depth(0).max_depth(5).into_iter().filter_map(|e| e.ok()) {
            debug!("Walking testdir...");
            debug!("  {:?}", dir);
            if dir.file_type().is_dir() {
                debug!("  Is directory: {:?}", dir);
                if dir.path().ends_with("wide") { wts.0 += 1 }
                if dir.path().ends_with("tall") { wts.1 += 1 }
                if dir.path().ends_with("sqr")  { wts.2 += 1 }
            }
        }
        debug!("wide, tall, square: {:?}", wts);
        assert_eq!( true, wts.0 == 1 && wts.1 == 1 && wts.2 == 1 );
    }

    #[test]
    fn test_image_paths() {
        init();
        let mut opts = test_opts();
        let root = test_dir_tree();
        populate_dir_tree(root.path());
        opts.input_dir = root.path().to_owned();
        debug!("Dir tree at: {:?}", opts.input_dir);

        // Non-recursive walk. Expect 3 images in root dir.
        let src_paths = image_paths(&opts);
        debug!("Src paths: {:#?}", src_paths);
        assert_eq!(src_paths.len(), 3);
        drop(src_paths);

        // Recursive walk. Expect 15 images in dir tree.
        opts.recursive = true;
        let src_paths = image_paths(&opts);
        debug!("Src paths: {:#?}", src_paths);
        assert_eq!(src_paths.len(), 15);
    }
    
    #[test]
    fn test_get_dsts() {
        init();
        let mut opts = test_opts();
        let root = test_dir_tree();
        populate_dir_tree(root.path());
        opts.input_dir = root.path().to_owned();

        // Non-recursive walk. Expect 3 images.
        let src_paths = image_paths(&opts);
        assert_eq!(src_paths.len(), 3);
        let dst_paths = get_dsts(&opts, &src_paths);
        assert_eq!(dst_paths.len(), 3);
        drop(src_paths);
        drop(dst_paths);

        // Recursive walk. Expect 15 images.
        opts.recursive = true;
        let src_paths = image_paths(&opts);
        assert_eq!(src_paths.len(), 15);
        let dst_paths = get_dsts(&opts, &src_paths);
        assert_eq!(dst_paths.len(), 15);
        drop(src_paths);
        drop(dst_paths);

        // Recursive walk with overwrite. Expect 3 images (all images in tree are named 'w.png', 't.png', or 's.png').
        // already set // opts.recursive = true;
        opts.overwrite = true;
        let src_paths = image_paths(&opts);
        assert_eq!(src_paths.len(), 3);
        let dst_paths = get_dsts(&opts, &src_paths);
        assert_eq!(dst_paths.len(), 3);
        drop(src_paths);
        drop(dst_paths);
    }

    // fn test_dst_path() {}
    // fn test_mv_files() {}
    // fn test_image_orientation() {}
    // fn test_prepend_orientation() {}
    // fn test_make_uniq() {}
}