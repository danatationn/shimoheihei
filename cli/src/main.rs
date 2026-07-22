use clap::Parser;
use cli::{Cli, Commands, info, unpack::Unpack};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Info(args) => info::display_info(&args),
        Commands::Unpack(args) => {
        	let unpacker = Unpack::from_args(args.clone());
        	unpacker.unpack().unwrap();
        },
    }
}
