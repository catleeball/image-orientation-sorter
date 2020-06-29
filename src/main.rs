#[macro_use]
extern crate log;
extern crate stderrlog;

use failure::Error;
use std::fs;
use std::path::PathBuf;
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

    if opt.recursive {
        recursive_rw_loop(opt.input_dir)?;
    } else {
        rw_loop(opt.input_dir)?;
    }

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
    debug!("CLI options: {:?}", opt);
    return opt
}

/// Recursively walk given directory, operating on each image file.
fn recursive_rw_loop(input_dir: PathBuf) -> Result<(), Error> {
    debug!("Recursively walking directory tree starting at {}", input_dir.display());
    for f in WalkDir::new(input_dir) {
        debug!("{}", f?.path().display());
    }
    Ok(())
}

/// Walk given directory, operating on each image file.
fn rw_loop(input_dir: PathBuf) -> Result<(), Error> {
    debug!("Directory contents at {}", input_dir.display());
    for f in fs::read_dir(".")? {
        debug!("{}", f?.path().display());
    }
    Ok(())
}
