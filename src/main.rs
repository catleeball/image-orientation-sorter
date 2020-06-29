#[macro_use]
extern crate log;
extern crate stderrlog;

use std::path::PathBuf;
use structopt::StructOpt;

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
    #[structopt(short, long, parse(from_occurrences), help = "Increasingly verbose output to stderr specified by adding more flags. i.e. -v -vv .. -vvvvv")]
    verbose: usize,
    #[structopt(short, long, help = "Do not print anything to stdout or stderr.")]
    quiet: bool,
    #[structopt(short, long, help = "Do not actually move any files. Implies verbose unless --quiet is provided.")]
    dry_run: bool,
}

fn main() {
    let opt = Opt::from_args();
    println!("{:?}", opt);

    stderrlog::new()
        .module(module_path!())
        .quiet(opt.quiet)
        .verbosity(opt.verbose)
        .init()
        .unwrap();
}
