use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
pub struct Article {
    pub url: String,
    pub title: String,
    pub body: String,
    pub created_at: i64,
    pub modified_at: i64,
    pub tags: Vec<String>,
}

impl Article {}
