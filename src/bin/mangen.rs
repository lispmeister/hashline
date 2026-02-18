use clap::CommandFactory;
use std::fs;

// Include cli module from main crate
#[path = "../cli.rs"]
mod cli;

fn main() -> std::io::Result<()> {
    let cmd = cli::Cli::command();

    fs::create_dir_all("man")?;

    // Top-level page
    let man = clap_mangen::Man::new(cmd.clone());
    let mut buf = Vec::new();
    man.render(&mut buf)?;
    fs::write("man/hashline.1", &buf)?;
    println!("Generated man/hashline.1");

    // Per-subcommand pages
    for sub in cmd.get_subcommands() {
        let name = sub.get_name();
        if name == "help" {
            continue;
        }
        let full_name = format!("hashline-{}", name);
        let man = clap_mangen::Man::new(sub.clone()).title(full_name.clone());
        let mut buf = Vec::new();
        man.render(&mut buf)?;
        let path = format!("man/{}.1", full_name);
        fs::write(&path, &buf)?;
        println!("Generated {}", path);
    }

    Ok(())
}
