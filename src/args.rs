use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author="Eslzzyl")]
#[command(version)]
#[command(about="imagebed: a simple image-hosting service", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Delete all the files in file storage
    Clear,
}