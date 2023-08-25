use std::fs::read_dir;
use std::path::PathBuf;

type SsgResult<T> = Result<T, Box<dyn std::error::Error>>;

struct Ssg {
    src: PathBuf,
    dest: PathBuf,
}

impl Ssg {
    pub fn new(src: PathBuf, dest: PathBuf) -> Ssg {
        Ssg { src, dest }
    }

    fn process_file(&self, file: PathBuf, base_path: PathBuf) -> SsgResult<()> {
        let relative_path = file.strip_prefix(&self.src)?;
        let url = base_path.join(relative_path);
        let url = url.to_str().unwrap().replace(".md", ".html");
        let dest_path = self.dest.join(&url);
        println!(
            "Processing {} -> {} [{}]",
            file.to_str().unwrap(),
            url,
            dest_path.to_str().unwrap()
        );
        Ok(())
    }

    fn process(&self, current: PathBuf) -> SsgResult<()> {
        for entry in read_dir(self.src.join(&current))? {
            let entry = entry?;
            let fname = entry.file_name();
            let fname = fname.to_str().unwrap();
            let ftype = entry.file_type()?;
            if ftype.is_file() && fname.ends_with(".md") {
                self.process_file(entry.path(), current.clone())?;
            } else if ftype.is_dir() {
                let next_cur = current.join(fname);
                self.process(next_cur)?;
            }
        }

        Ok(())
    }
}

fn main() -> SsgResult<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        println!("Usage: {} [source dir] [dest dir]", args[0]);
        std::process::exit(1);
    }
    let source_dir = PathBuf::from(&args[1]);
    let destination_dir = PathBuf::from(&args[2]);
    let ssg = Ssg::new(source_dir, destination_dir);
    ssg.process(PathBuf::new())?;
    Ok(())
}
