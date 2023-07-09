use std::io::{self, BufRead};

#[derive(Debug, PartialEq)]
pub struct Candidate {
    pub title: String,
    pub authors: Vec<String>,
    pub series: Option<String>,
}

enum PasteParseState {
    AwaitNotesAndHighlights,
    ExpectTitle {
        previous_line: String,
    },
    ExpectAuthor {
        title: String,
        previous_line: String,
    }, // value is previous line
}

pub fn parse_paste<I: BufRead>(vals: &mut I) -> std::io::Result<Vec<Candidate>> {
    let mut state = PasteParseState::AwaitNotesAndHighlights;
    let mut candidates = Vec::new();

    for lin in vals.lines() {
        let line = lin?;
        match state {
            PasteParseState::AwaitNotesAndHighlights => {
                if line == "Notes & Highlights" {
                    state = PasteParseState::ExpectTitle {
                        previous_line: line,
                    };
                }
            }
            PasteParseState::ExpectTitle { ref previous_line } => {
                if line.is_empty() {
                    continue;
                }
                if &line == previous_line {
                    continue;
                }
                state = PasteParseState::ExpectAuthor {
                    title: line.clone(),
                    previous_line: line,
                };
            }
            PasteParseState::ExpectAuthor {
                ref title,
                ref previous_line,
            } => {
                if line.is_empty() {
                    continue;
                }
                if &line == previous_line {
                    continue;
                }
                let authors = line.split(";").map(|s| s.trim().to_string()).collect();
                candidates.push(Candidate {
                    title: title.clone(),
                    authors,
                    series: None,
                });
                state = PasteParseState::ExpectTitle {
                    previous_line: line,
                };
            }
        }
    }
    Ok(candidates)
}

//
/**
 * Basic Logic:
 * - read to "Notes & Highlights"
 * - Skip blank lines
 * - Expect alternating Title | Author
 * - Any line may duplicate the previous line, if so, ignore it
 */
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::{assert_eq, assert_ne};

    #[test]
    fn test_safari() {
        let mut buf = SAFARI.clone().as_bytes();
        let r = parse_paste(&mut buf);
        assert!(matches!(r, Ok(_)));

        let vals = r.unwrap();

        let expected = vec![
            Candidate {
                title: "Stiletto: A Novel (The Rook Files Book 2)".to_string(),
                authors: vec![
                    "O'Malley, Daniel".to_string(),
                    "O'Malley, Daniel".to_string(),
                ],
                series: None,
            },
            Candidate {
                title: "The Joy of Abstraction: An Exploration of Math, Category Theory, and Life"
                    .to_string(),
                authors: vec!["Cheng, Eugenia".to_string()],
                series: None,
            },
        ];

        assert_eq!(vals, expected);
    }

    fn test_chrome() {
        let mut buf = CHROME.clone().as_bytes();
        let r = parse_paste(&mut buf);
        assert!(matches!(r, Ok(_)));

        let vals = r.unwrap();

        let expected = vec![
            Candidate {
                title: "Stiletto: A Novel (The Rook Files Book 2)".to_string(),
                authors: vec![
                    "O'Malley, Daniel".to_string(),
                    "O'Malley, Daniel".to_string(),
                ],
                series: None,
            },
            Candidate {
                title: "The Joy of Abstraction: An Exploration of Math, Category Theory, and Life"
                    .to_string(),
                authors: vec!["Cheng, Eugenia".to_string()],
                series: None,
            },
        ];

        assert_eq!(vals, expected);
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
