#![feature(plugin)]
#![plugin(afl_plugin)]

extern crate afl;
extern crate indolentjson;


use indolentjson::nodes::*;


use std::io::{self, Read};



fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();

    JsonNode::from_input(&input.as_bytes()).ok();
}
