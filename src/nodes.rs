use ::parse::*;

use std::borrow::Cow;
use std::marker::PhantomData;
use std::str;

use itertools::Itertools;

use linear_map::LinearMap;


#[derive(Debug, Clone, Copy)]
pub struct NodeExt<'a, 'b: 'a> {
    raw: &'b [u8],
    children: &'b [Node],
    node: &'b Node,
    ph: PhantomData<&'a u8>
}

impl<'a, 'b: 'a> NodeExt<'a, 'b> {
    pub fn len(&self) -> usize {
        self.node.length_in_bytes as usize
    }

    pub fn node(&'a self) -> &'b Node {
        self.node
    }

    pub fn raw(&'a self) -> &'b [u8] {
        &self.raw
    }

    pub fn children(&'a self) -> NodeCollection<'b> {
        NodeCollection {
            raw: &self.raw[1..self.len()],
            nodes: &self.children,
            next_offset: 0,
            next_index: 0,
        }
    }
}

pub struct NodeCollection<'a> {
    raw: &'a [u8],
    nodes: &'a [Node],
    next_offset: usize,
    next_index: usize,
}


impl<'a> NodeCollection<'a> {
    pub fn from_nodes(compacted: &'a [u8], nodes: &'a [Node]) -> NodeCollection<'a> {
        NodeCollection {
            raw: compacted,
            nodes: nodes,
            next_offset: 0,
            next_index: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}

impl<'a> Iterator for NodeCollection<'a> {
    type Item = NodeExt<'a, 'a>;

    fn next(&mut self) -> Option<NodeExt<'a, 'a>> {
        if self.next_index < self.nodes.len() {
            let node = &self.nodes[self.next_index];
            let num_children = node.children as usize;

            let n = NodeExt {
                raw: &self.raw[self.next_offset..self.next_offset+node.length_in_bytes as usize],
                node: node,
                children: &self.nodes[self.next_index+1..self.next_index + 1 + num_children],
                ph: PhantomData,
            };

            self.next_offset += node.length_in_bytes as usize + 1;
            self.next_index += num_children + 1;

            Some(n)
        } else {
            None
        }
    }
}


#[derive(Debug, Clone)]
pub struct ObjectNode<'a> {
    raw: Option<Cow<'a, str>>,
    map: LinearMap<StringNode<'a>, JsonNode<'a>>,
}

#[derive(Debug, Clone)]
pub struct ArrayNode<'a> {
    raw: Option<Cow<'a, str>>,
    nodes: Vec<JsonNode<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StringNode<'a> {
    raw: Option<Cow<'a, str>>,
}

#[derive(Debug, Clone)]
pub struct NumberNode<'a> {
    raw: Option<Cow<'a, str>>,
}


impl<'a> ObjectNode<'a> {
    pub fn to_string(&'a self) -> Cow<'a, str> {
        if let Some(ref raw) = self.raw {
            Cow::Borrowed(&raw)
        } else {
            Cow::Owned(format!("{{{}}}",
                self.map.iter()
                        .map(|e| format!("{}:{}", e.0.to_string(), e.1.to_string()))
                        .join(",")
            ))
        }
    }

    pub fn from_node_ext(node: &NodeExt<'a, 'a>) -> ObjectNode<'a> {
        ObjectNode {
            raw: Some(Cow::Borrowed(unsafe {str::from_utf8_unchecked(&node.raw())})),
            map: {
                let mut vec = LinearMap::with_capacity(node.children().len());
                let mut children = node.children();
                while let Some((k, v)) = children.next().and_then(|k| children.next().map(|v| (k,v))) {
                    vec.insert(
                        match JsonNode::from_node_ext(&k) {
                            JsonNode::String(string_node) => string_node,
                            _ => panic!("Invalid key type"),
                        },
                        JsonNode::from_node_ext(&v),
                    );
                }
                vec
            }
        }
    }

    pub fn map(&self) -> &LinearMap<StringNode<'a>, JsonNode<'a>> {
        &self.map
    }

    pub fn map_mut(&mut self) -> &mut LinearMap<StringNode<'a>, JsonNode<'a>> {
        self.raw = None;
        &mut self.map
    }
}

impl<'a> ArrayNode<'a> {
    pub fn to_string(&'a self) -> Cow<'a, str> {
        if let Some(ref raw) = self.raw {
            Cow::Borrowed(&raw)
        } else {
            Cow::Owned(
                format!("[{}]", self.nodes.iter().map(|n| n.to_string()).join(","))
            )
        }
    }
}

impl<'a> StringNode<'a> {
    pub fn to_string(&'a self) -> Cow<'a, str> {
        if let Some(ref raw) = self.raw {
            Cow::Borrowed(&raw)
        } else {
            unreachable!()  // TODO
        }
    }

    pub fn from_str(s: &str) -> StringNode<'a> {
        StringNode {
            raw: Some(Cow::Owned(format!(r#""{}""#, s))),
        }
    }
}

impl<'a> NumberNode<'a> {
    pub fn to_string(&'a self) -> Cow<'a, str> {
        if let Some(ref raw) = self.raw {
            Cow::Borrowed(&raw)
        } else {
            unreachable!()  // TODO
        }
    }
}

#[derive(Debug, Clone)]
pub enum JsonNode<'a> {
    Object(ObjectNode<'a>),
    Array(ArrayNode<'a>),
    String(StringNode<'a>),
    Number(NumberNode<'a>),
    Boolean(bool),
    Null,
}

impl<'a> JsonNode<'a> {
    pub fn from_node_ext(node: &NodeExt<'a, 'a>) -> JsonNode<'a> {
        match node.raw()[0] {
            b't' => JsonNode::Boolean(true),
            b'f' => JsonNode::Boolean(false),
            b'n' => JsonNode::Null,
            b'"' => JsonNode::String(StringNode{
                raw: Some(Cow::Borrowed(unsafe {str::from_utf8_unchecked(&node.raw())})),
            }),
            b'{' => JsonNode::Object(ObjectNode::from_node_ext(node)),
            b'[' => JsonNode::Array(ArrayNode {
                raw: Some(Cow::Borrowed(unsafe {str::from_utf8_unchecked(&node.raw())})),
                nodes: {
                    let mut vec = Vec::with_capacity(node.children().len());
                    for node in node.children() {
                        vec.push(JsonNode::from_node_ext(&node))
                    }
                    vec
                },
            }),
            b'-' | b'0'...b'9' => JsonNode::Number(NumberNode {
                raw: Some(Cow::Borrowed(unsafe {str::from_utf8_unchecked(&node.raw())})),
            }),
            _ => panic!("WUT?"),
        }
    }

    pub fn to_string(&'a self) -> Cow<'a, str> {
        match *self {
            JsonNode::Object(ref node) => {
                node.to_string()
            }
            JsonNode::Array(ref node) => {
                node.to_string()
            }
            JsonNode::String(ref node) => {
                node.to_string()
            }
            JsonNode::Number(ref node) => {
                node.to_string()
            }
            JsonNode::Boolean(true) => Cow::Borrowed("true"),
            JsonNode::Boolean(false) => Cow::Borrowed("false"),
            JsonNode::Null => Cow::Borrowed("null"),
        }
    }
}

#[cfg(test)]
mod tests {
    use compact::*;
    use parse::*;
    use super::*;

    use std::str;

    #[test]
    fn remove_key() {
        let test_string: &str = r#"{
            "A longish bit of JSON": true,
            "containing": {
                "whitespace": " ",
                "unicode escapes ": "\uFFFF\u0FFF\u007F\uDBFF\uDFFF",
                "other sorts of esacpes": "\b\t\n\f\r\"\\\/",
                "unicode escapes for the other sorts of escapes":
                    "\u0008\u0009\u000A\u000C\u000D\u005C\u0022",
                "numbers": [0, 1, 1e4, 1.0, -1.0e7 ],
                "and more": [ true, false, null ]
            },
            "and_even_more": "blah"
        }"#;

        let mut compacted: Vec<u8> = Vec::new();
        let mut nodes: Vec<Node> = Vec::new();
        let mut parse_stack: Vec<Stack> = Vec::new();

        compact(test_string.as_bytes(), &mut compacted).unwrap();
        parse(&compacted[..], &mut nodes, &mut parse_stack).unwrap();

        let mut collection = NodeCollection::from_nodes(&compacted, &nodes);
        let first_node = collection.next().unwrap();

        let json_node = JsonNode::from_node_ext(&first_node);

        let mut obj = match json_node {
            JsonNode::Object(obj) => obj,
            _ => panic!("Expeected object")
        };

        let compacted_str = str::from_utf8(&compacted).unwrap();

        assert_eq!(obj.to_string(), compacted_str);

        obj.map_mut();
        assert_eq!(obj.to_string(), compacted_str);

        {
            let mut map = obj.map_mut();
            map.remove(&StringNode::from_str("containing"));
        }

        assert_eq!(obj.to_string(), r#"{"A longish bit of JSON":true,"and_even_more":"blah"}"#);
    }

}
