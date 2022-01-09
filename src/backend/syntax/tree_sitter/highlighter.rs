use super::language::Loaded;
use crate::backend::buffer::Buffer;
use crate::backend::highlight::{Highlighter, Range, TextHighlighting};
use std::rc::Rc;
use tree_sitter::{Parser, Query, QueryCursor, Tree};

#[derive(Debug)]
pub struct HighlightQuery {
    query: Query,
    capture_table: Vec<String>,
}

impl HighlightQuery {
    pub fn new(query: Query, capture_table: Vec<String>) -> Self {
        Self {
            query,
            capture_table
        }
    }
}

pub struct TreeSitterHighlighter {
    query: Rc<HighlightQuery>,
    parser: Parser,
    tree: Option<Tree>,
}

#[derive(Debug)]
pub enum IllegalQuery {
    NameNotFound(String, String),
    DuplicateId(String, u32),
}

impl TreeSitterHighlighter {
    pub fn make_capture_table(q: &Query) -> Result<Vec<String>, IllegalQuery> {
        let mut capture_table = Vec::new();
        for name in q.capture_names() {
            let capture_idx = q
                .capture_index_for_name(name)
                .ok_or_else(|| IllegalQuery::NameNotFound(format!("{:?}", q), name.clone()))?;
            if capture_table.len() <= capture_idx as usize {
                for _ in capture_table.len()..=capture_idx as usize {
                    capture_table.push(String::new());
                }
            }
            if !capture_table[capture_idx as usize].is_empty() {
                return Err(IllegalQuery::DuplicateId(format!("{:?}", q), capture_idx));
            }
            capture_table[capture_idx as usize] = name.clone();
        }
        Ok(capture_table)
    }

    pub fn new(ll: Loaded) -> Result<Self, tree_sitter::LanguageError> {
        let mut parser = Parser::new();
        parser.set_language(ll.language)?;
        Ok(TreeSitterHighlighter {
            parser,
            tree: None,
            query: ll.query,
        })
    }

    fn highlight_from_tree(&self, buf: &Buffer) -> TextHighlighting {
        let tree = self.tree.as_ref().unwrap();
        let mut cursor = QueryCursor::new(); // we need a way to map from the number of bytes to the line # and col #
        let mut ranges = Vec::new();
        for m in cursor.matches(&self.query.query, tree.root_node(), &buf.to_chunk()[..]) {
            for capture in m.captures {
                if let Ok(highlight) = self.query.capture_table[capture.index as usize].parse() {
                    ranges.push(Range {
                        start: buf.to_pos(capture.node.start_byte()),
                        stop: buf.to_pos(capture.node.end_byte()),
                        highlight,
                    });
                }
            }
        }
        TextHighlighting::from_ranges(buf.lines(), ranges)
    }
}

impl Highlighter for TreeSitterHighlighter {
    fn highlight(&mut self, buf: &Buffer) -> TextHighlighting {
        let buffer = buf.to_chunk();
        self.tree = self.parser.parse(&buffer[..], None);
        self.highlight_from_tree(buf)
    }
}
