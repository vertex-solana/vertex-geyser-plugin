use ::{clap::Parser, clap_derive::Parser, vertex_geyser_plugin::config::Config};

#[derive(Debug, Parser)]
struct Args {
    #[clap(short, long, default_value_t = String::from("config.json"))]
    config: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let _config = Config::load_from_file(args.config)?;
    println!("Config is OK!");
    Ok(())
}
