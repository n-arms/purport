use super::buffer::Buffer;
use super::highlight::{Highlighter, TextHighlighting, Range};

use tree_sitter::{Language, LanguageError, Parser, Query, QueryCursor, Tree};

pub struct TreeSitterHighlighter {
    parser: Parser,
    tree: Option<Tree>,
    highlight_query: Query,
    capture_table: Vec<String>,
}

#[derive(Debug)]
pub enum TreeSitterError {
    NameNotInQuery(String, String),
    DuplicateIdInQuery(String, u32),
    LangErr(LanguageError),
}

impl TreeSitterHighlighter {
    pub fn new(l: Language, highlight_query: Query) -> Result<Self, TreeSitterError> {
        let mut parser = Parser::new();
        parser.set_language(l).map_err(TreeSitterError::LangErr)?;
        let mut capture_table = Vec::new();
        for name in highlight_query.capture_names() {
            let capture_idx = highlight_query
                .capture_index_for_name(name)
                .ok_or_else(|| {
                    TreeSitterError::NameNotInQuery(format!("{:?}", highlight_query), name.clone())
                })?;
            if capture_table.len() <= capture_idx as usize {
                for _ in capture_table.len()..=capture_idx as usize {
                    capture_table.push(String::new());
                }
            }
            if !capture_table[capture_idx as usize].is_empty() {
                return Err(TreeSitterError::DuplicateIdInQuery(
                    format!("{:?}", highlight_query),
                    capture_idx,
                ));
            }
            capture_table[capture_idx as usize] = name.clone();
        }
        Ok(TreeSitterHighlighter {
            parser,
            tree: None,
            highlight_query,
            capture_table,
        })
    }
    fn highlight_from_tree(&self, buf: &Buffer) -> TextHighlighting {
        let tree = self.tree.as_ref().unwrap();
        let mut cursor = QueryCursor::new(); // we need a way to map from the number of bytes to the line # and col #
        let mut ranges = Vec::new();
        for m in cursor.matches(
            &self.highlight_query,
            tree.root_node(),
            buf.bytes().as_slice(),
        ) {
            for capture in m.captures {
                if let Ok(highlight) = self.capture_table[capture.index as usize].parse() {
                    ranges.push(Range {
                        start:
                        buf.to_pos(capture.node.start_byte()),
                        stop:
                        buf.to_pos(capture.node.end_byte()),
                        highlight
                    });
                } else {
                    eprintln!(
                        "could not highlight type {:?}",
                        self.capture_table[capture.index as usize]
                    );
                }
            }
        }
        TextHighlighting::from_ranges(buf.lines.len(), ranges)
    }
}

impl Highlighter for TreeSitterHighlighter {
    fn highlight(&mut self, buf: &Buffer) -> TextHighlighting {
        let mut buffer = Vec::new();
        for line in &buf.lines {
            buffer.reserve(line.len());
            for byte in line.bytes() {
                buffer.push(byte);
            }
        }
        self.tree = self.parser.parse(&buffer[..], None);
        self.highlight_from_tree(buf)
    }
}
