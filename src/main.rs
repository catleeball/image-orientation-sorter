use clap::{App, Arg, crate_version};

fn main() {
    let args = get_args();
    let imgdir: &str = args.value_of("Directory").unwrap_or("./");
    let recursive: bool = args.value_of("Recursive").unwrap_or("false").parse().unwrap();
    let mv: bool = args.is_present("Move");
    let prepend: &str = args.value_of("Prepend").unwrap_or("");
    let print: bool = args.is_present("Print");
    let verbose: bool = args.is_present("Verbose");
    let dryrun: bool = args.is_present("Dry Run");
    println!("{}\n{}\n{}\n{}\n{}\n{}\n{}", imgdir, recursive, mv, prepend, print, verbose, dryrun);
}

/// Return CLI arguments.
pub fn get_args() -> clap::ArgMatches<'static> {

    return App::new("imgorisort")
        .about("Image Orientation Sorter")
        .version(crate_version!())
        .arg(Arg::with_name("Directory")
            .help("Directory containing image files to sort by orientation.")
            .index(1)
            .required(true)
            .default_value("./"))
        .arg(Arg::with_name("Recursive")
            .help("Recurse into subdirectories.")
            .long("recursive")
            .short("r")
            .takes_value(false)
            .default_value("false"))
        .arg(Arg::with_name("Move")
            .help("Directory to move images into, sorted by orientation. Subdirectories [portrait, landscape, square] may be created in this directory.")
            .long("move")
            .short("m")
            .takes_value(true)
            .default_value("./"))
        .arg(Arg::with_name("Prepend orientation to filename")
            .help("Prepend 'portrait', 'landscape', or 'square' to image filenames.")
            .long("prepend")
            .short("p")
            .takes_value(false)
            .default_value("false"))
        .arg(Arg::with_name("Print orientations")
            .help("Print to stdout each filename and its orientation")
            .long("print")
            .short("p")
            .takes_value(false)
            .default_value("true"))
        .arg(Arg::with_name("Verbose")
            .help("Print absolute source path and destination path of each file")
            .long("verbose")
            .short("v")
            .takes_value(false)
            .default_value("false"))
        .arg(Arg::with_name("Dry Run")
            .help("Do not actually move any files. Implies --print and --verbose.")
            .long("dry-run")
            .short("d")
            .takes_value(false)
            .default_value("false"))
        .get_matches();
}
