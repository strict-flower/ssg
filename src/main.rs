use comrak::ComrakOptions;
use std::cmp::{Ord, Ordering, PartialOrd};
use std::fs::read_dir;
use std::path::{Path, PathBuf};

type SsgResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, PartialEq, Eq)]
enum PageNode {
    IndexPage(PathBuf, Vec<PageNode>),
    Article(PathBuf),
}

impl PartialOrd for PageNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (&self, &other) {
            (&PageNode::IndexPage(path1, _), &PageNode::IndexPage(path2, _)) => {
                match path1.cmp(path2) {
                    Ordering::Equal => None,
                    o => Some(o),
                }
            }
            (&PageNode::IndexPage(path1, _), &PageNode::Article(path2)) => {
                if path1 == path2 {
                    None
                } else {
                    Some(Ordering::Greater) // an article must be less than index page
                }
            }
            (&PageNode::Article(path1), &PageNode::IndexPage(path2, _)) => {
                if path1 == path2 {
                    None
                } else {
                    Some(Ordering::Less) // an article must be less than index page
                }
            }
            (&PageNode::Article(path1), &PageNode::Article(path2)) => Some(path1.cmp(path2)),
        }
    }
}

impl Ord for PageNode {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.partial_cmp(other) {
            None => Ordering::Equal,
            Some(ordering) => ordering,
        }
    }
}

#[derive(Debug)]
struct Ssg {
    src: PathBuf,
    dest: PathBuf,
    option: ComrakOptions,
    template: String,
}

impl Ssg {
    pub fn new(src: PathBuf, dest: PathBuf, template_file: &Path) -> SsgResult<Ssg> {
        let mut option = ComrakOptions::default();
        option.extension.strikethrough = true;
        option.extension.footnotes = true;
        option.extension.autolink = true;
        option.extension.table = true;
        option.extension.description_lists = true;
        option.extension.front_matter_delimiter = Some("---".to_string());
        option.render.unsafe_ = true;

        Ok(Ssg {
            src,
            dest,
            option,
            template: String::from_utf8(std::fs::read(template_file)?)?,
        })
    }

    fn process_file(&self, file: PathBuf, base_path: PathBuf) -> SsgResult<PageNode> {
        let relative_path = file.strip_prefix(&self.src)?;
        let relative_path = relative_path.strip_prefix(&base_path)?;
        let url = base_path.join(relative_path);
        let url = url.to_str().unwrap().replace(".md", ".html");
        let dest_path = self.dest.join(&url);
        println!(
            "Processing {} -> {} [{}]",
            file.to_str().unwrap(),
            url,
            dest_path.to_str().unwrap()
        );
        let markdown = String::from_utf8(std::fs::read(&file)?)?;
        let html = self.template.replace(
            "{{ article }}",
            &comrak::markdown_to_html(&markdown, &self.option),
        );
        std::fs::write(dest_path.as_path(), html.into_bytes())?;
        Ok(PageNode::Article(url.into()))
    }

    fn process(&self, current: PathBuf) -> SsgResult<PageNode> {
        let dest_dir = self.dest.join(&current);
        std::fs::create_dir_all(dest_dir)?;
        let mut res = vec![];
        for entry in read_dir(self.src.join(&current))? {
            let entry = entry?;
            let fname = entry.file_name();
            let fname = fname.to_str().unwrap();
            let ftype = entry.file_type()?;
            if ftype.is_file() && fname.ends_with(".md") {
                res.push(self.process_file(entry.path(), current.clone())?);
            } else if ftype.is_dir() {
                let next_cur = current.join(fname);
                res.push(self.process(next_cur)?);
            }
        }

        res.sort();

        Ok(PageNode::IndexPage(current.clone(), res))
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
    let template_file = Path::new("assets/template.html");
    let ssg = Ssg::new(source_dir, destination_dir, template_file)?;
    dbg!(ssg.process(PathBuf::new())?);
    Ok(())
}
