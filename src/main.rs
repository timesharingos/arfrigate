use arfrigate::{
    args::{Commands, parse_args},
    ignore::execute::run_filter,
};

fn main() {
    let cli = parse_args();
    match cli.command {
        Commands::Filter { dirs } => run_filter(dirs),
    }
}
