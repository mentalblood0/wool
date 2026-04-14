use anyhow::{Error, Result};
use fallible_iterator::FallibleIterator;
use serde::{Deserialize, Serialize};

use crate::content::Content;
use crate::read_transaction_methods::ReadTransactionMethods;
use crate::thesis::Thesis;

#[derive(Serialize, Deserialize)]
pub struct GraphGeneratorConfig {
    pub wrap_width: u16,
}

pub enum Stage {
    BeforeFirstLine,
    Middle,
    AfterLastLine,
}

pub struct GraphGenerator<'a> {
    pub config: &'a GraphGeneratorConfig,
    pub read_able_transaction: &'a dyn ReadTransactionMethods<'a>,
    pub theses_iterator: Box<dyn FallibleIterator<Item = Thesis, Error = Error> + 'a>,
    pub stage: Stage,
}

impl<'a> GraphGenerator<'a> {
    pub fn new(
        config: &'a GraphGeneratorConfig,
        read_able_transaction: &'a dyn ReadTransactionMethods<'a>,
    ) -> Result<Self> {
        Ok(Self {
            config,
            read_able_transaction,
            theses_iterator: Box::new(read_able_transaction.iter_theses()?),
            stage: Stage::BeforeFirstLine,
        })
    }
}

impl<'a> GraphGenerator<'a> {
    fn wrap(&self, text: &str) -> String {
        let wrap_width = self.config.wrap_width as usize;
        if wrap_width == 0 {
            return String::new();
        }

        let mut result = String::with_capacity(text.len() + (text.len() / wrap_width) * 5);
        let mut current_line = String::new();
        let mut current_line_size = 0;
        let mut first_line = true;

        for word in text.split_whitespace() {
            let word_size = word.len();

            if current_line.is_empty() {
                current_line.reserve(word_size);
                current_line.push_str(word);
                current_line_size = word_size;
            } else if current_line_size + 1 + word_size <= wrap_width {
                current_line.push(' ');
                current_line.push_str(word);
                current_line_size += 1 + word_size;
            } else {
                if !first_line {
                    result.push_str("<br/>");
                }
                result.push_str(&current_line);
                first_line = false;
                current_line = String::with_capacity(word_size);
                current_line.push_str(word);
                current_line_size = word_size;
            }
        }

        if !current_line.is_empty() {
            if !first_line {
                result.push_str("<br/>");
            }
            result.push_str(&current_line);
        }

        result
    }
}

impl<'a> FallibleIterator for GraphGenerator<'a> {
    type Item = String;
    type Error = Error;

    fn next(&mut self) -> Result<Option<Self::Item>> {
        Ok(match self.stage {
            Stage::BeforeFirstLine => {
                self.stage = Stage::Middle;
                Some("digraph sweater {".to_string())
            }
            Stage::Middle => {
                if let Some(thesis) = self.theses_iterator.next()? {
                    let thesis_id_string = thesis.id().to_string();
                    let node_header_text = if let Some(ref alias) = thesis.alias {
                        html_escape::encode_text(&alias.0).to_string()
                    } else {
                        thesis_id_string.clone()
                    };
                    match thesis.content {
                        Content::Text(ref text) => {
                            let node_body_text =
                                self.wrap(&self.read_able_transaction.compose_with_aliases(text)?);
                            let node_header = format!(
                                r#"<TR><TD BORDER="1" SIDES="b">{node_header_text}</TD></TR>"#,
                            );
                            let node_label = format!(
                                r#"<TABLE BORDER="2" CELLSPACING="0" CELLPADDING="8">{}<TR><TD BORDER="0">{}</TD></TR></TABLE>"#,
                                node_header, node_body_text
                            );
                            Some(
                                format!(
                                    "\n\t\"{}\" [label=<{}>, shape=plaintext];", // node definition
                                    thesis_id_string, node_label
                                ) + &thesis // node references arrows definitions
                                    .references()
                                    .iter()
                                    .map(|referenced_thesis_id| {
                                        format!(
                                            "\n\t\"{thesis_id_string}\" -> \"{}\" \
                                             [arrowhead=none, color=\"grey\" style=dotted];",
                                            referenced_thesis_id.to_string()
                                        )
                                    })
                                    .collect::<Vec<_>>()
                                    .join(""),
                            )
                        }
                        Content::Relation(ref relation) => {
                            let node_label = format!(
                                r#"<TABLE CELLSPACING="0" CELLPADDING="8" STYLE="dashed"><TR><TD SIDES="b" STYLE="dashed">{node_header_text}</TD></TR><TR><TD BORDER="0">{}</TD></TR></TABLE>"#,
                                relation.kind.0
                            );
                            Some(format!(
                                "\n\t\"{thesis_id_string}\" [label=<{node_label}>, \
                                 shape=plaintext];\n\t\"{}\" -> \"{}\" [dir=back, \
                                 arrowtail=tee];\n\t\"{}\" -> \"{}\";",
                                relation.from.to_string(), // arrow to relation node
                                thesis_id_string,
                                thesis_id_string, // arrow from relation node
                                relation.to.to_string()
                            ))
                        }
                    }
                } else {
                    self.stage = Stage::AfterLastLine;
                    Some("\n}".to_string())
                }
            }
            Stage::AfterLastLine => None,
        })
    }
}
