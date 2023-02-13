use std::io::{self, BufRead};

pub struct Candidate {
    pub title: String,
    pub authors: Vec<String>,
    pub series: Option<String>,
}

pub fn try_kindle_paste<I: BufRead>(vals: &mut I) -> std::io::Result<String> {
    for lin in vals.lines() {
        let line = lin?;
        if line.starts_with("==========") {
            return Ok(line);
        }
    }
    Ok("hello world".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::{assert_eq, assert_ne};

    #[test]
    fn it_works() {
        let mut buf = SAFARI.clone().as_bytes();
        let r = try_kindle_paste(&mut buf);

        assert_eq!(r.unwrap(), "hello world");
    }

    static SAFARI: &str = r#"
"Skip to the book library"


Filter

Sort by:
Recent

View


Library
All Titles
Books
Comics
Samples

Notes & Highlights
Stiletto: A Novel (The Rook Files Book 2)
Stiletto: A Novel (The Rook Files Book 2)
Stiletto: A Novel (The Rook Files Book 2)
O'Malley, Daniel; O'Malley, Daniel
The Joy of Abstraction: An Exploration of Math, Category Theory, and Life
The Joy of Abstraction: An Exploration of Math, Category Theory, and Life
The Joy of Abstraction: An Exploration of Math, Category Theory, and Life
Cheng, Eugenia
"#;

    static CHROME: &str = r#"
"Skip to the book library"
Search your Kindle

Filter

Sort by:
Recent

View


Library
All Titles
Books
Comics
Samples

Notes & Highlights
Stiletto: A Novel (The Rook Files Book 2)

O'Malley, Daniel; O'Malley, Daniel

The Joy of Abstraction: An Exploration of Math, Category Theory, and Life

Cheng, Eugenia
"#;
}
