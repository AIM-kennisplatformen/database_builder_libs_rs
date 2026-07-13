use std::collections::HashMap;

use quick_xml::{Reader, events::Event};
use rootcause::{
    option_ext::OptionExt,
    prelude::{Report, ResultExt},
};

#[derive(Debug, Default)]
pub struct Node {
    pub name: String,
    pub attributes: HashMap<String, String>,
    children: Vec<Node>,
    content: Vec<Content>,
}

#[derive(Debug)]
enum Content {
    Text(String),
    Child(usize),
}

pub fn parse_tree(xml: &str) -> Result<Node, Report> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut stack = vec![Node {
        name: "document".into(),
        ..Node::default()
    }];

    loop {
        match reader
            .read_event()
            .context("failed to read TEI XML event")?
        {
            Event::Start(event) => stack.push(node_from_start(event)?),
            Event::Empty(event) => append_child(
                stack.last_mut().expect("document node"),
                node_from_empty(event)?,
            ),
            Event::Text(event) => {
                stack
                    .last_mut()
                    .expect("document node")
                    .content
                    .push(Content::Text(
                        event
                            .unescape()
                            .context("failed to decode TEI XML text")?
                            .into_owned(),
                    ))
            }
            Event::CData(event) => {
                stack
                    .last_mut()
                    .expect("document node")
                    .content
                    .push(Content::Text(
                        String::from_utf8_lossy(event.as_ref()).into_owned(),
                    ))
            }
            Event::End(_) => {
                let node = stack.pop().context("unexpected TEI XML closing tag")?;
                append_child(stack.last_mut().context("missing TEI XML root")?, node);
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(stack.pop().expect("document node"))
}

fn node_from_start(event: quick_xml::events::BytesStart<'_>) -> Result<Node, Report> {
    node_from_event(event)
}

fn node_from_empty(event: quick_xml::events::BytesStart<'_>) -> Result<Node, Report> {
    node_from_event(event)
}

fn node_from_event(event: quick_xml::events::BytesStart<'_>) -> Result<Node, Report> {
    let mut node = Node {
        name: local_name(event.name().as_ref()),
        ..Node::default()
    };
    for attribute in event.attributes().with_checks(false) {
        let attribute = attribute.context("failed to read TEI XML attribute")?;
        node.attributes.insert(
            local_name(attribute.key.as_ref()),
            attribute
                .unescape_value()
                .context("failed to decode TEI XML attribute")?
                .into_owned(),
        );
    }
    Ok(node)
}

fn append_child(parent: &mut Node, node: Node) {
    let index = parent.children.len();
    parent.children.push(node);
    parent.content.push(Content::Child(index));
}

pub fn child<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    node.children.iter().find(|child| child.name == name)
}

pub fn children_named<'a>(node: &'a Node, name: &str) -> Vec<&'a Node> {
    node.children
        .iter()
        .filter(|child| child.name == name)
        .collect()
}

pub fn descendant<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    (node.name == name).then_some(node).or_else(|| {
        node.children
            .iter()
            .find_map(|child| descendant(child, name))
    })
}

pub fn text(node: &Node) -> String {
    let mut value = String::new();
    for part in &node.content {
        match part {
            Content::Text(text) => value.push_str(text),
            Content::Child(index) => value.push_str(&text(&node.children[*index])),
        }
    }
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn local_name(name: &[u8]) -> String {
    String::from_utf8_lossy(name)
        .rsplit(':')
        .next()
        .unwrap_or_default()
        .to_owned()
}

impl Node {
    pub fn descendants_named(&self, name: &str) -> Vec<&Node> {
        let mut result = Vec::new();
        if self.name == name {
            result.push(self);
        }
        for child in &self.children {
            result.extend(child.descendants_named(name));
        }
        result
    }
}
