use crate::article::Article;
use crate::tree::PageNode;
use crate::SsgResult;
use chrono::offset::Utc;
use chrono::DateTime;
use comrak::ComrakOptions;
use regex::Regex;
use serde_json::json;
use std::fs::{read_dir, Metadata};
use std::path::PathBuf;

#[derive(Debug)]
pub struct Ssg {
    src: PathBuf,
    dest: PathBuf,
    option: ComrakOptions,
    ssg_option_regex: Regex,
    tag_element_regex: Regex,
}

impl Ssg {
    pub fn new(src: PathBuf, dest: PathBuf) -> SsgResult<Ssg> {
        let mut option = ComrakOptions::default();
        option.extension.strikethrough = true;
        option.extension.footnotes = true;
        option.extension.autolink = true;
        option.extension.table = true;
        option.extension.description_lists = true;
        option.extension.front_matter_delimiter = Some("---".to_string());
        option.render.unsafe_ = true;

        let ssg_option_regex = Regex::new(r"ssg-([\w\-]+): *([^\n]+) *")?;
        let tag_element_regex = Regex::new(r"#([^#,]+)(?:,\s*)?")?;

        Ok(Ssg {
            src,
            dest,
            option,
            ssg_option_regex,
            tag_element_regex,
        })
    }

    fn process_markdown_file(
        &self,
        file: PathBuf,
        base_path: PathBuf,
        metadata: Metadata,
    ) -> SsgResult<PageNode> {
        let mut title = file
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .strip_suffix(".md")
            .unwrap();
        let relative_path = file.strip_prefix(&self.src)?;
        let relative_path = relative_path.strip_prefix(&base_path)?;
        let url = base_path.join(relative_path);
        let url = url.to_str().unwrap().replace(".md", "");

        let markdown = std::fs::read_to_string(&file)?;
        let mut tags = vec![];

        let created_at: DateTime<Utc> = metadata.created()?.into();
        let mut created_at = created_at.timestamp();

        let modified_at: DateTime<Utc> = metadata.modified()?.into();
        let mut modified_at = modified_at.timestamp();

        if self.ssg_option_regex.is_match(&markdown) {
            for caps in self.ssg_option_regex.captures_iter(&markdown) {
                let key = caps.get(1).unwrap().as_str();
                let value = caps.get(2).unwrap().as_str();
                if key == "tags" {
                    tags = self
                        .tag_element_regex
                        .captures_iter(value)
                        .map(|x| x.get(1).unwrap().as_str().to_string())
                        .collect();
                } else if key == "title" {
                    title = value;
                } else if key == "created-at" {
                    created_at = value.parse()?;
                    modified_at = value.parse()?;
                } else if key == "modified-at" {
                    modified_at = value.parse()?;
                }
            }
        }

        let markdown = markdown.replace(r"\.", r"\\.").replace(r"\,", r"\\,");

        let html = comrak::markdown_to_html(&markdown, &self.option);
        let html = html.replace("href=\"http", "href=\"https://href.li/?http");

        let article = Article {
            url: url.clone(),
            title: title.to_string(),
            body: html,
            created_at,
            modified_at,
            tags,
        };

        Ok(PageNode::Article(url.clone().into(), article))
    }

    pub fn process(&self, current: PathBuf) -> SsgResult<PageNode> {
        let dest_dir = self.dest.join(&current);
        std::fs::create_dir_all(dest_dir)?;
        let mut res = vec![];
        for entry in read_dir(self.src.join(&current))? {
            let entry = entry?;
            let fname = entry.file_name();
            let fname = fname.to_str().unwrap();
            let ftype = entry.file_type()?;
            if ftype.is_file() && fname.ends_with(".md") {
                res.push(self.process_markdown_file(
                    entry.path(),
                    current.clone(),
                    entry.metadata()?,
                )?);
            } else if ftype.is_dir() {
                let next_cur = current.join(fname);
                res.push(self.process(next_cur)?);
            }
        }

        res.sort();
        res.reverse();

        let articles: Vec<&PageNode> = res
            .iter()
            .filter(|x| matches!(x, PageNode::Article(_, _)))
            .collect();
        let indexes: Vec<&PageNode> = res
            .iter()
            .filter(|x| matches!(x, PageNode::IndexPage(_, _)))
            .collect();

        let mut articles_json = vec![];
        let mut indexes_json = vec![];

        for (pos, x) in articles.iter().enumerate() {
            if let PageNode::Article(path, article) = &x {
                articles_json.push(json! {
                    {
                        "created_at": article.created_at,
                        "modified_at": article.modified_at,
                        "title": article.title,
                        "path": path.to_path_buf(),
                    }
                });

                let next_path = if pos != 0 {
                    if let PageNode::Article(path_next, _) = articles[pos - 1] {
                        path_next.to_path_buf()
                    } else {
                        PathBuf::new()
                    }
                } else {
                    PathBuf::new()
                };
                let prev_path = if pos + 1 < articles.len() {
                    if let PageNode::Article(path_prev, _) = articles[pos + 1] {
                        path_prev.to_path_buf()
                    } else {
                        PathBuf::new()
                    }
                } else {
                    PathBuf::new()
                };
                let dest_path = &self
                    .dest
                    .join(PathBuf::from([path.to_str().unwrap(), ".json"].concat()));
                std::fs::write(
                    dest_path.as_path(),
                    serde_json::to_string(&json! {
                        {
                            "article": &article,
                            "prev_path": prev_path,
                            "next_path": next_path
                        }
                    })?,
                )?;
            }
        }

        for x in indexes.iter() {
            if let PageNode::IndexPage(path, _) = &x {
                indexes_json.push(path.join("index.json"));
            }
        }

        std::fs::write(
            self.dest.join(&current).join("index.json"),
            serde_json::to_string(&json! {
                {
                    "articles": articles_json,
                    "indexes": indexes_json
                }
            })?,
        )?;

        Ok(PageNode::IndexPage(current, res))
    }
}
