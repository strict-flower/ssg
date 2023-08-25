use chrono::offset::{FixedOffset, Utc};
use chrono::DateTime;
use comrak::ComrakOptions;
use serde::Serialize;
use std::cmp::{Ord, Ordering, PartialOrd};
use std::fs::{read_dir, Metadata};
use std::path::{Path, PathBuf};

type SsgResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, PartialEq, Eq, Serialize)]
enum PageNode {
    IndexPage(PathBuf, Vec<PageNode>),
    Article(PathBuf, String, String),
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
            (&PageNode::IndexPage(path1, _), &PageNode::Article(path2, _, _)) => {
                if path1 == path2 {
                    None
                } else {
                    Some(Ordering::Greater) // an article must be less than index page
                }
            }
            (&PageNode::Article(path1, _, _), &PageNode::IndexPage(path2, _)) => {
                if path1 == path2 {
                    None
                } else {
                    Some(Ordering::Less) // an article must be less than index page
                }
            }
            (&PageNode::Article(path1, _, _), &PageNode::Article(path2, _, _)) => {
                Some(path1.cmp(path2))
            }
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
            template: std::fs::read_to_string(template_file)?,
        })
    }

    fn process_file(
        &self,
        file: PathBuf,
        base_path: PathBuf,
        metadata: Metadata,
    ) -> SsgResult<PageNode> {
        let title = file
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .strip_suffix(".md")
            .unwrap();
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
        let markdown = std::fs::read_to_string(&file)?;
        let html = comrak::markdown_to_html(&markdown, &self.option);
        let html = html.replace("href=\"", "href=\"https://href.li/?");
        let html = self.template.replace("{{ article }}", &html);
        let html = html.replace("{{ title }}", title);

        let created_at: DateTime<Utc> = metadata.created()?.into();
        let created_at = created_at
            .with_timezone(&FixedOffset::east_opt(32400).unwrap())
            .format("%Y/%m/%d %H:%I")
            .to_string();

        let modified_at: DateTime<Utc> = metadata.modified()?.into();
        let modified_at = modified_at
            .with_timezone(&FixedOffset::east_opt(32400).unwrap())
            .format("%Y/%m/%d %H:%I")
            .to_string();

        let html = html.replace("{{ created_at }}", &created_at);
        let html = html.replace("{{ modified_at }}", &modified_at);
        std::fs::write(dest_path.as_path(), html.into_bytes())?;
        Ok(PageNode::Article(url.into(), created_at, modified_at))
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
                res.push(self.process_file(entry.path(), current.clone(), entry.metadata()?)?);
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
    let template_file = Path::new("assets/template_article.html");
    let index_template_file = Path::new("assets/template_index.html");
    let ssg = Ssg::new(source_dir, destination_dir.clone(), template_file)?;
    let index = ssg.process(PathBuf::from(""))?;
    let index_template = std::fs::read_to_string(index_template_file)?;
    let index_html = index_template.replace("{{ data }}", &serde_json::to_string(&index)?);

    std::fs::write(
        destination_dir.join("index.html").as_path(),
        index_html.into_bytes(),
    )?;

    Ok(())
}
