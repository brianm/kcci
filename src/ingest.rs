/*
   Copyright 2023 Brian McCallister

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

use linked_hash_set::LinkedHashSet;
use regex::Regex;
use std::io::BufRead;

#[derive(Debug, PartialEq)]
pub struct Candidate {
    raw_title: String,
    raw_authors: Vec<String>,
}

impl Candidate {
    pub fn new(raw_line: &str, authors: Vec<String>) -> Self {
        Candidate {
            raw_title: String::from(raw_line),
            raw_authors: authors,
        }
    }

    pub fn title(&self) -> String {
        let (title, _, _) = parse_title(&self.raw_title);
        title
    }

    pub fn authors(&self) -> Vec<String> {
        let mut s = LinkedHashSet::new();
        for a in &self.raw_authors {
            s.insert(a.to_string());
        }
        s.into_iter().collect::<Vec<_>>()
    }

    pub fn series(&self) -> Option<(String, Option<u32>)> {
        let (_, series, num) = parse_title(&self.raw_title);
        series.map(|s| (s, num))
    }
}

/// This is going to need to expand to get all heuristic, I fear.
/// It may need to change to keep the original line, so we can do various
/// lookups agfainst APIs with variants on the original.
///
/// most likely, will have raw_title, raw_athors, and then a thing to generate a
/// probablistic sequence of things based on heuristics, for querying API to get metadata.
fn parse_title(line: &str) -> (String, Option<String>, Option<u32>) {
    let mut title = line.to_owned();
    let mut series = None;
    let mut sequence_in_series = None;

    let re = Regex::new(r"^(.*) \((.*?),? Book (\d+)\)$").unwrap();
    if let Some(cap) = re.captures(line) {
        title = cap[1].to_string();
        series = Some(cap[2].to_string());
        sequence_in_series = Some(cap[3].parse::<u32>().unwrap());
        return (title, series, sequence_in_series);
    }
    (title, series, sequence_in_series)
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
                let authors = line.split(';').map(|s| s.trim().to_string()).collect();
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
    fn test_extract_parts() {
        let c = Candidate {
            raw_authors: vec![
                "O'Malley, Daniel".to_string(),
                "O'Malley, Daniel".to_string(),
            ],
            raw_title: "Stiletto: A Novel (The Rook Files Book 2)".to_string(),
        };
        assert_eq!(c.authors(), vec!["O'Malley, Daniel".to_string()]); // removed dup author
        assert_eq!(c.title(), "Stiletto: A Novel");
        assert_eq!(c.series(), Some(("The Rook Files".to_string(), Some(2))));
    }

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
                raw_title: "Stiletto: A Novel (The Rook Files Book 2)".to_string(),
                raw_authors: vec![
                    "O'Malley, Daniel".to_string(),
                    "O'Malley, Daniel".to_string(),
                ],
            },
            Candidate {
                raw_title:
                    "The Joy of Abstraction: An Exploration of Math, Category Theory, and Life"
                        .to_string(),
                raw_authors: vec!["Cheng, Eugenia".to_string()],
            },
            Candidate {
                raw_title: "Assassin's Apprentice (The Farseer Trilogy, Book 1)".to_string(),
                raw_authors: vec!["Hobb, Robin".to_string()],
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
