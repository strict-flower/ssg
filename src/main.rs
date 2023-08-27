use crate::article::Article;
use crate::ssg::Ssg;
use std::path::PathBuf;

mod article;
mod ssg;
mod tree;

pub type SsgResult<T> = Result<T, Box<dyn std::error::Error>>;

fn main() -> SsgResult<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        println!("Usage: {} [source dir] [dest dir]", args[0]);
        std::process::exit(1);
    }
    let source_dir = PathBuf::from(&args[1]);
    let destination_dir = PathBuf::from(&args[2]);
    let ssg = Ssg::new(source_dir, destination_dir.clone())?;
    ssg.process(PathBuf::from(""))?;
    Ok(())
}
