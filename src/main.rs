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
    directory: PathBuf,
    #[structopt(short, long, help = "Recurse into subdirectories.")]
    recursive: bool,
    #[structopt(short = "m", long = "move", help = "Directory to move images into, sorted by orientation. Subdirectories [portrait, landscape, square] may be created in this directory.")]
    mv: bool,
    #[structopt(short, long, help = "Prepend 'portrait', 'landscape', or 'square' to image filenames.")]
    prefix: bool,
    #[structopt(short, long, parse(from_occurrences), help = "Increasingly verbose output to stderr specified by adding more flags. i.e. -v -vv -vvv")]
    verbose: usize,
    #[structopt(short, long, help = "Do not print anything to stdout or stderr.")]
    quiet: bool,
    #[structopt(short, long, help = "Do not actually move any files. Implies verbose unless --quiet is provided.")]
    dry_run: bool,
}

fn main() -> Result<(), Error>{
    let opt = Opt::from_args();
    stderrlog::new()
        .module(module_path!())
        .quiet(opt.quiet)
        .verbosity(opt.verbose)
        .init()
        .unwrap();
    
    debug!("CLI options: {:?}", opt);

    if opt.recursive {
        recursive_rw_loop(opt.directory)?;
    } else {
        rw_loop(opt.directory)?;
    }

    if !opt.quiet { println!("Operation complete!"); }

    Ok(())
}

fn recursive_rw_loop(directory: PathBuf) -> Result<(), Error> {
    debug!("Recursively walking directory tree starting at {}", directory.display());
    for dir in WalkDir::new(directory) {
        debug!("  {}", dir?.path().display());
    }
    Ok(())
}

fn rw_loop(directory: PathBuf) -> Result<(), Error> {
    debug!("Directory contents at {}", directory.display());
    for dir in fs::read_dir(".")? {
        debug!("  {}", dir?.path().display());
    }
    Ok(())
}
