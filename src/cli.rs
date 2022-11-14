use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    #[arg(long, default_value = "127.0.0.1")]
    pub local_address: String,

    #[arg(long, default_value = "53")]
    pub local_port: u16,

    #[arg(long, default_value = "1.1.1.1")]
    pub upstream_address: String,

    #[arg(long, default_value = "443")]
    pub upstream_port: u16,
}
