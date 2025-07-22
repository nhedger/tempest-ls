use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Tempest Language Server",
    long_about = None
)]
struct Cli {
    /// Use stdio as the communication channel
    #[arg(long, conflicts_with_all = ["pipe", "socket"])]
    stdio: bool,

    /// Use a pipe (windows) or a socket file (unix) as the communication channel
    #[arg(long, conflicts_with_all = ["stdio", "socket"])]
    pipe: bool,

    /// Use a socket as the communication channel
    #[arg(long, conflicts_with_all = ["stdio", "pipe"])]
    socket: u16,
}

fn main() {
    let _args = Cli::parse();
    println!("Starting tempest language server...");
}