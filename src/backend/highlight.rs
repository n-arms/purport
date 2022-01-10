use super::buffer::Buffer;
use crate::frontend::ui::{Colour, IsNotColour};
use std::collections::HashMap;
use std::hash::Hash;
use std::str;
use toml::{de, Value};

pub trait Factory {
    fn make(&self) -> anyhow::Result<Box<dyn Highlighter>>;
}

pub trait Highlighter {
    fn highlight(&mut self, buf: &Buffer) -> TextHighlighting;
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TextHighlighting(HashMap<usize, HashMap<usize, Type>>);

impl TextHighlighting {
    fn row_len(&self, row: usize) -> usize {
        self.0.get(&row).map_or(0, HashMap::len)
    }

    pub fn get_line(&self, row: usize) -> Option<LineHighlighting> {
        let mut start: Option<Type> = None;
        for i in (0..row).rev() {
            if self.row_len(i) != 0 {
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
            start.unwrap_or(Type::Text),
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
                h.get_mut(&end_row).unwrap().insert(end_col, Type::Text);
            }
        }
        TextHighlighting(h)
    }
}

#[derive(Clone, Debug)]
pub struct LineHighlighting(Type, HashMap<usize, Type>);

impl LineHighlighting {
    pub fn get(&self, idx: usize) -> Option<Type> {
        self.1
            .get(&idx)
            .copied()
            .or_else(|| if idx == 0 { Some(self.0) } else { None })
    }
}

impl Default for LineHighlighting {
    fn default() -> Self {
        LineHighlighting(Type::Text, HashMap::new())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Range {
    pub start: (usize, usize),
    pub stop: (usize, usize),
    pub highlight: Type,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Type {
    Number,
    Operator,
    Text,
    Keyword,
    Variable,
}

#[derive(Debug)]
pub struct IsNotType(String);

impl str::FromStr for Type {
    type Err = IsNotType;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "number" => Ok(Type::Number),
            "operator" => Ok(Type::Operator),
            "text" => Ok(Type::Text),
            "variable" => Ok(Type::Variable),
            "keyword" => Ok(Type::Keyword),
            _ => Err(IsNotType(s.to_string())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    highlighting: HashMap<Type, Colour>,
}

impl Theme {
    pub fn get(&self, h: Type) -> Colour {
        self.highlighting.get(&h).copied().unwrap()
    }
}

impl Default for Theme {
    fn default() -> Self {
        let mut highlighting = HashMap::new();
        highlighting.insert(Type::Number, Colour::Blue);
        highlighting.insert(Type::Operator, Colour::Green);
        highlighting.insert(Type::Variable, Colour::Cyan);
        highlighting.insert(Type::Keyword, Colour::Yellow);
        highlighting.insert(Type::Text, Colour::Reset);
        Theme { highlighting }
    }
}

#[derive(Debug)]
pub enum Error {
    Toml(de::Error),
    Colour(IsNotColour),
    Type(IsNotType),
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
                        k.parse::<Type>().map_err(Error::Type)?,
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
        highlighting.insert(Type::Operator, Colour::Red);
        highlighting.insert(Type::Number, Colour::Green);
        assert_eq!(theme, Theme { highlighting });
    }
}
