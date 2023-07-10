use regex::Regex;
use std::io::BufRead;

#[derive(Debug, PartialEq)]
pub struct Candidate {
    pub title: String,
    pub authors: Vec<String>,
    pub series: Option<String>,
    pub sequence_in_series: Option<u32>,
}

impl Candidate {
    pub fn new(title: &str, authors: Vec<String>) -> Self {
        let (title, series, sequence_in_series) = parse_title(&title.to_string());
        Candidate {
            title,
            authors,
            series,
            sequence_in_series,
        }
    }
}

/// This is going to need to expand to get all heuristic, I fear.
/// It may need to change to keep the original line, so we can do various
/// lookups agfainst APIs with variants on the original.
///
/// most likely, will have raw_title, raw_athors, and then a thing to generate a
/// probablistic sequence of things based on heuristics, for querying API to get metadata.
fn parse_title(line: &String) -> (String, Option<String>, Option<u32>) {
    let mut title = line.clone();
    let mut series = None;
    let mut sequence_in_series = None;

    let re = Regex::new(r"^(.*) \((.*?),? Book (\d+)\)$").unwrap();
    re.captures(&line).map(|cap| {
        title = cap[1].to_string();
        series = Some(cap[2].to_string());
        sequence_in_series = Some(cap[3].parse::<u32>().unwrap());
    });
    return (title, series, sequence_in_series);
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
                candidates.push(Candidate::new(title, authors));
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
    use pretty_assertions::assert_eq;

    #[test]
    fn test_safari() {
        let mut buf = SAFARI.clone().as_bytes();
        let r = parse_paste(&mut buf);
        assert!(matches!(r, Ok(_)));

        let vals = r.unwrap();
        assert_eq!(vals, expected());
    }

    #[test]
    fn test_chrome() {
        let mut buf = CHROME.clone().as_bytes();
        let r = parse_paste(&mut buf);
        assert!(matches!(r, Ok(_)));

        let vals = r.unwrap();
        assert_eq!(vals, expected());
    }

    fn expected() -> Vec<Candidate> {
        return vec![
            Candidate {
                title: "Stiletto: A Novel".to_string(),
                authors: vec![
                    "O'Malley, Daniel".to_string(),
                    "O'Malley, Daniel".to_string(),
                ],
                series: Some("The Rook Files".to_string()),
                sequence_in_series: Some(2),
            },
            Candidate {
                title: "The Joy of Abstraction: An Exploration of Math, Category Theory, and Life"
                    .to_string(),
                authors: vec!["Cheng, Eugenia".to_string()],
                series: None,
                sequence_in_series: None,
            },
            Candidate {
                title: "Assassin's Apprentice".to_string(),
                authors: vec!["Hobb, Robin".to_string()],
                series: Some("The Farseer Trilogy".to_string()),
                sequence_in_series: Some(1),
            },
        ];
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
Assassin's Apprentice (The Farseer Trilogy, Book 1)
Hobb, Robin
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

Assassin's Apprentice (The Farseer Trilogy, Book 1)

Hobb, Robin

"#;
}
