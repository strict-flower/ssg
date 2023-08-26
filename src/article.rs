use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Article {
    pub url: String,
    pub title: String,
    pub body: String,
    pub created_at: String,
    pub modified_at: String,
    pub tags: Vec<String>,
}

impl Article {}
