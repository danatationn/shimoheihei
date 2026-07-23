use clap::Parser;
use cli::{Cli, Commands, list, info, unpack::Unpack};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Info(args) => info::from(&args).unwrap(),
    	Commands::List(args) => list::from(&args).unwrap(),
        Commands::Unpack(args) => {
        	let unpacker = Unpack::from(args.clone());
        	unpacker.unpack().unwrap();
        },
    }
}
