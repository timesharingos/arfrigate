use arfrigate::{
    args::{Commands, parse_args},
    filter::IgnoreFilter,
};

fn main() {
    let cli = parse_args();
    match cli.command {
        Commands::Filter { dirs } => {
            let filter = IgnoreFilter::new_str(dirs);
            let valid_files: Vec<String> = filter
                .filter()
                .iter()
                .filter_map(|path| path.to_str().map(|path| path.to_string()))
                .collect();
            println!("{}", valid_files.join(" "));
        }
    }
}
