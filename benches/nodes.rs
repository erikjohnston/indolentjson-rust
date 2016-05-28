#![feature(test)]

extern crate indolentjson;
extern crate test;

use indolentjson::compact::*;
use indolentjson::parse::*;
use indolentjson::nodes::*;
use test::Bencher;


const TEST_STRING : &'static [u8] = br#"{
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


#[bench]
fn parse_nodes(b : &mut Bencher) {
    let mut compacted: Vec<u8> = Vec::new();
    let mut nodes: Vec<Node> = Vec::new();
    let mut parse_stack: Vec<Stack> = Vec::new();

    compact(TEST_STRING, &mut compacted).unwrap();
    parse(&compacted[..], &mut nodes, &mut parse_stack).unwrap();

    let mut collection = NodeCollection::from_nodes(&compacted, &nodes);
    let first_node = collection.next().unwrap();

    b.iter(|| { JsonNode::from_node_ext(&first_node) });
}


#[bench]
fn parse_bytes_uncompact(b : &mut Bencher) {
    b.iter(|| { JsonNode::from_input(TEST_STRING) });
}

#[bench]
fn parse_bytes_compact(b : &mut Bencher) {
    let mut compacted = Vec::new();

    compact(TEST_STRING, &mut compacted).unwrap();

    b.iter(|| { JsonNode::from_input(&compacted) });
}
