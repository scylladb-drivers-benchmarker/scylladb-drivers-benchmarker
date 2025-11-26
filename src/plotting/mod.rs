use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct FrontendArguments {
    #[arg(short, long)]
    pub benchmark_name: String,

    #[arg(short, long)]
    pub commit_hashes: Vec<String>,

    #[arg(short, long)]
    pub visualization_kind: String,
}
