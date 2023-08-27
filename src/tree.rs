use crate::Article;
use serde::Serialize;
use std::cmp::{Ord, Ordering, PartialOrd};
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq, Serialize, Clone)]
pub enum PageNode {
    IndexPage(PathBuf, Vec<PageNode>),
    Article(PathBuf, Article),
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
            (&PageNode::IndexPage(path1, _), &PageNode::Article(path2, _)) => {
                if path1 == path2 {
                    None
                } else {
                    Some(Ordering::Greater) // an article must be less than index page
                }
            }
            (&PageNode::Article(path1, _), &PageNode::IndexPage(path2, _)) => {
                if path1 == path2 {
                    None
                } else {
                    Some(Ordering::Less) // an article must be less than index page
                }
            }
            (&PageNode::Article(path1, article1), &PageNode::Article(path2, article2)) => {
                Some(if article1.created_at == article2.created_at {
                    path1.cmp(path2)
                } else {
                    article1.created_at.cmp(&article2.created_at)
                })
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
