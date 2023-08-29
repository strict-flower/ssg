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
        let json_url = url.to_str().unwrap().replace(".md", ".json");
        let url = url.to_str().unwrap().replace(".md", "");
        let dest_path = self.dest.join(json_url);

        /*
        println!(
            "Processing {} -> {} [{}]",
            file.to_str().unwrap(),
            url,
            dest_path.to_str().unwrap()
        );
        */

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

        std::fs::write(dest_path.as_path(), serde_json::to_string(&article)?)?;

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

        let mut articles = vec![];
        let mut indexes = vec![];

        for x in res.iter() {
            match &x {
                PageNode::IndexPage(path, _) => indexes.push(path.join("index.json")),
                PageNode::Article(path, article) => articles.push(json! {
                    {
                        "created_at": article.created_at,
                        "modified_at": article.modified_at,
                        "title": article.title,
                        "path": path.to_path_buf()
                    }
                }),
            }
        }

        std::fs::write(
            self.dest.join(&current).join("index.json"),
            serde_json::to_string(&json! {
                {
                    "articles": articles,
                    "indexes": indexes
                }
            })?,
        )?;

        Ok(PageNode::IndexPage(current, res))
    }
}
