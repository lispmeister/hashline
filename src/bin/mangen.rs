use clap::CommandFactory;
use std::fs;

// Include cli module from main crate
#[path = "../cli.rs"]
mod cli;

fn main() -> std::io::Result<()> {
    let cmd = cli::Cli::command();

    fs::create_dir_all("man")?;

    let man = clap_mangen::Man::new(cmd);
    let mut buf = Vec::new();
    man.render(&mut buf)?;
    fs::write("man/hashline.1", buf)?;

    println!("Generated man/hashline.1");
    Ok(())
}
