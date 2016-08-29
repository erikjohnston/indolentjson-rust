#![feature(test)]

extern crate indolentjson;
extern crate test;

use indolentjson::nodes::*;
use test::{black_box, Bencher};

const TEST_STRING : &'static str = r#"{
    "A longish bit of JSON": true,
    "containing": {
        "whitespace": " ",
        "unicode escapes ": "\uFFFF\u0FFF\u007F\uDBFF\uDFFF",
        "other sorts of esacpes": "\b\t\n\f\r\"\\\/",
        "unicode escapes for the other sorts of escapes":
            "\u0008\u0009\u000A\u000C\u000D\u005C\u0022",
        "numbers": [0, 1, 1e4, 1.0, -1.0e7 ],
        "and more": [ true, false, null ]
    }
}"#;

#[bench]
fn remove_key(b : &mut Bencher) {
    let test_string = black_box(TEST_STRING.as_bytes());
    let value = Value::from_slice(test_string);
    b.bytes = value.as_bytes().len() as u64;
    b.iter(|| {
        let mut cloned = value.clone();
        cloned.remove_key(b"containing");
    });
}

#[bench]
fn discard_key(b : &mut Bencher) {
    let test_string = black_box(TEST_STRING.as_bytes());
    let value = Value::from_slice(test_string);
    b.bytes = value.as_bytes().len() as u64;
    b.iter(|| {
        let mut cloned = value.clone();
        cloned.discard_key(b"containing");
        cloned
    });
}

#[bench]
fn add_key(b : &mut Bencher) {
    let test_string = black_box(TEST_STRING.as_bytes());
    let value = Value::from_slice(test_string);
    let sub_value = Value::from_slice(test_string);
    b.bytes = value.as_bytes().len() as u64;
    b.iter(|| {
        let mut cloned = value.clone();
        cloned.add_key(b"test", &sub_value);
        cloned
    });
}

#[bench]
fn prepend_key(b : &mut Bencher) {
    let test_string = black_box(TEST_STRING.as_bytes());
    let value = Value::from_slice(test_string);
    let sub_value = Value::from_slice(test_string);
    b.bytes = value.as_bytes().len() as u64;
    b.iter(|| {
        let mut cloned = value.clone();
        cloned.prepend_key(b"test", &sub_value);
        cloned
    });
}
