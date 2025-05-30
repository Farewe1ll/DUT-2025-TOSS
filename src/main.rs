use clap::Parser;

mod cli;
mod database;
mod embeddings;
mod llm;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let args = cli::Cli::parse();
	match args.command {
		cli::Commands::Ask { query } => {
			// Retrieve relevant content from database
			let references = database::retrieve(&query).await?;

			// Generate answer using LLM with context
			let answer = llm::answer_with_context(&query, references).await?;
			println!("Answer: {}", answer);
		}
		cli::Commands::Remember { content } => {
			// Store the content in the database
			let stored_content = database::insert(&content).await?;
			println!("✅ Content remembered successfully!");
			println!("ID: {}", stored_content.id);
		}
	}
	Ok(())
}