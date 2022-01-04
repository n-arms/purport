use super::buffer::Buffer;
use crate::frontend::ui::{Colour, IsNotColour};
use regex::{Regex};

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::str;
use toml::{de, Value};

pub trait Highlighter {
    fn highlight(&mut self, buf: &Buffer) -> TextHighlighting;
}

#[derive(Clone, Debug)]
pub struct RegexHighlighter {
    pub operator: Regex,
    pub number: Regex,
}

impl Highlighter for RegexHighlighter {
    fn highlight(&mut self, _text: &Buffer) -> TextHighlighting {
        /*
        let numbers = text.lines.iter().enumerate().flat_map(|(i, line)| {
            self.number
                .find_iter(line.skip(0))
                .map(move |m| (i, Match::start(&m)))
        });
        let operators = text.lines.iter().enumerate().flat_map(|(i, line)| {
            self.operator
                .find_iter(line.skip(0))
                .map(move |m| (i, Match::start(&m)))
        });
        let text = text
            .lines
            .iter()
            .enumerate()
            .flat_map(|(i, line)| {
                self.operator
                    .find_iter(line.skip(0))
                    .map(move |m| (i, Match::end(&m)))
            })
            .chain(text.lines.iter().enumerate().flat_map(|(i, line)| {
                self.number
                    .find_iter(line.skip(0))
                    .map(move |m| (i, Match::end(&m)))
            }));
        let mut h: HashMap<usize, HashMap<usize, HighlightType>> = HashMap::new();
        for (i, n) in numbers {
            if let Some(l) = h.get_mut(&i) {
                l.insert(n, HighlightType::Number);
            } else {
                h.insert(i, {
                    let mut sub = HashMap::new();
                    sub.insert(n, HighlightType::Number);
                    sub
                });
            }
        }
        for (i, o) in operators {
            if let Some(l) = h.get_mut(&i) {
                l.insert(o, HighlightType::Operator);
            } else {
                h.insert(i, {
                    let mut sub = HashMap::new();
                    sub.insert(o, HighlightType::Operator);
                    sub
                });
            }
        }
        for (i, t) in text {
            if let Some(l) = h.get_mut(&i) {
                l.insert(t, HighlightType::Text);
            } else {
                h.insert(i, {
                    let mut sub = HashMap::new();
                    sub.insert(t, HighlightType::Operator);
                    sub
                });
            }
        }
        TextHighlighting(h)
        */
        todo!()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TextHighlighting(HashMap<usize, HashMap<usize, HighlightType>>);

impl TextHighlighting {
    fn row_len(&self, row: usize) -> usize {
        self.0.get(&row).map_or(0, HashMap::len)
    }

    // the problem is at or before here
    pub fn get_line(&self, row: usize) -> Option<LineHighlighting> {
        let mut start: Option<HighlightType> = None;
        for i in (0..row).rev() {
            if self.row_len(i) != 0 {
                //start = self.0.get(&i).and_then(|row| row.values().last()).copied();
                if let Some(row) = self.0.get(&i) {
                    let mut largest = 0;
                    for k in row.keys() {
                        largest = *k.max(&largest);
                    }
                    start = row.get(&largest).copied();
                }
                break;
            }
        }
        Some(LineHighlighting(
            start.unwrap_or(HighlightType::Text),
            self.0.get(&row)?.clone(),
        ))
    }

    pub fn from_ranges(len: usize, range: Vec<Range>) -> Self {
        let mut h = HashMap::new();
        for i in 0..len {
            h.insert(i, HashMap::new());
        }
        for Range {
            start: (start_row, start_col),
            stop: (end_row, end_col),
            highlight,
        } in range
        {
            h.get_mut(&start_row).unwrap().insert(start_col, highlight);
            if !h[&end_row].contains_key(&end_col) {
                h.get_mut(&end_row)
                    .unwrap()
                    .insert(end_col, HighlightType::Text);
            }
        }
        TextHighlighting(h)
    }
}

#[derive(Clone, Debug)]
pub struct LineHighlighting(HighlightType, HashMap<usize, HighlightType>);

impl LineHighlighting {
    pub fn get(&self, idx: usize) -> Option<HighlightType> {
        self.1
            .get(&idx)
            .copied()
            .or_else(|| if idx == 0 { Some(self.0) } else { None })
    }
}

impl Default for LineHighlighting {
    fn default() -> Self {
        LineHighlighting(HighlightType::Text, HashMap::new())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Range {
    pub start: (usize, usize),
    pub stop: (usize, usize),
    pub highlight: HighlightType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HighlightType {
    Number,
    Operator,
    Text,
    Keyword,
    Variable,
}

#[derive(Debug)]
pub struct IsNotHighlightType(String);

impl str::FromStr for HighlightType {
    type Err = IsNotHighlightType;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "number" => Ok(HighlightType::Number),
            "operator" => Ok(HighlightType::Operator),
            "text" => Ok(HighlightType::Text),
            "variable" => Ok(HighlightType::Variable),
            "keyword" => Ok(HighlightType::Keyword),
            _ => Err(IsNotHighlightType(s.to_string())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    highlighting: HashMap<HighlightType, Colour>,
}

impl Theme {
    pub fn get(&self, h: HighlightType) -> Colour {
        self.highlighting.get(&h).copied().unwrap()
    }
}

impl Default for Theme {
    fn default() -> Self {
        let mut highlighting = HashMap::new();
        highlighting.insert(HighlightType::Number, Colour::Blue);
        highlighting.insert(HighlightType::Operator, Colour::Green);
        highlighting.insert(HighlightType::Variable, Colour::Cyan);
        highlighting.insert(HighlightType::Keyword, Colour::Yellow);
        highlighting.insert(HighlightType::Text, Colour::Reset);
        Theme { highlighting }
    }
}

#[derive(Debug)]
pub enum Error {
    Toml(de::Error),
    Colour(IsNotColour),
    HighlightType(IsNotHighlightType),
    IsntTable,
    IsntString,
}

impl str::FromStr for Theme {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let toml = s.parse::<Value>().map_err(Error::Toml)?;
        if let Value::Table(t) = toml {
            let mut highlighting = HashMap::new();
            for (k, v) in t {
                if let Value::String(s) = v {
                    highlighting.insert(
                        k.parse::<HighlightType>().map_err(Error::HighlightType)?,
                        s.parse::<Colour>().map_err(Error::Colour)?,
                    );
                } else {
                    return Err(Error::IsntString);
                }
            }
            Ok(Theme { highlighting })
        } else {
            Err(Error::IsntTable)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse() {
        let theme: Theme = "
        operator = \"red\"
        number = \"green\"
        "
        .parse()
        .unwrap();

        let mut highlighting = HashMap::new();
        highlighting.insert(HighlightType::Operator, Colour::Red);
        highlighting.insert(HighlightType::Number, Colour::Green);
        assert_eq!(theme, Theme { highlighting });
    }
}
