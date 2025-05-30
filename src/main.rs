use clap::Parser;
mod cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	println!("Hello, world!");

	let args = cli::Cli::parse();
	match args.command {
		cli::Commands::Ask {query} => {
			let answer = "haha";
			println!("Answer: {}", answer);
		},
		cli::Commands::Remember {content} => {
			println!("hey, please remember!")
		}
	}
	Ok(())
}