use parse::{Node, Stack, parse};
use compact::compact;

use std::ops::Range;
use std::ptr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value {
    nodes: Vec<Node>,
    compacted: Vec<u8>,
}

impl Value {
    pub unsafe fn from_parts(nodes: Vec<Node>, compacted: Vec<u8>) -> Value {
        Value {
            nodes: nodes,
            compacted: compacted,
        }
    }

    pub fn from_slice(bytes: &[u8]) -> Value {
        let mut compacted: Vec<u8> = Vec::new();
        let mut nodes: Vec<Node> = Vec::new();
        let mut parse_stack: Vec<Stack> = Vec::new();
        compact(bytes, &mut compacted).unwrap();
        parse(&compacted[..], &mut nodes, &mut parse_stack).unwrap();

        Value {
            nodes: nodes,
            compacted: compacted,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.compacted
    }

    pub fn get_key(&self, key: &[u8]) -> Option<(&[Node], &[u8])> {
        if let Some((nodes_range, compacted_range, value_range)) = self.find_key_parts(key) {
            Some((
                &self.nodes[nodes_range.start + 1..nodes_range.end],
                &self.compacted[compacted_range.start + value_range.start..compacted_range.start + value_range.end],
            ))
        } else {
            None
        }
    }

    pub fn prepend_key(&mut self, key: &[u8], value: &Value) {
        if self.compacted[0] == b'{' {
            self.compacted.insert(1, b'"');
            push_all_at(&mut self.compacted, 2, &key);
            push_all_at(&mut self.compacted, key.len() + 2, br#"":"#);
            push_all_at(&mut self.compacted, key.len() + 4, &value.compacted);

            if self.nodes.len() > 1 {
                self.compacted.insert(key.len() + value.compacted.len() + 4, b',');
            }

            self.nodes[0].children += 2 + value.nodes[0].children;
            self.nodes[0].length_in_bytes = self.compacted.len() as u32;

            push_all_at(&mut self.nodes, 1, &value.nodes);
            self.nodes.insert(1, Node {
                children: 0,
                length_in_bytes: key.len() as u32 + 2
            });
        }
    }

    pub fn add_key(&mut self, key: &[u8], value: &Value) {
        if self.compacted[0] == b'{' {
            let current_len = self.compacted.len();
            self.compacted.reserve(current_len + 4 + key.len() + value.compacted.len());
            self.compacted.pop();
            if self.nodes.len() > 1 {
                self.compacted.push(b',');
            }
            self.compacted.push(b'"');
            let len = self.compacted.len();
            push_all_at(&mut self.compacted, len, key);
            self.compacted.push(b'"');
            self.compacted.push(b':');
            let len = self.compacted.len();
            push_all_at(&mut self.compacted, len, &value.compacted);
            self.compacted.push(b'}');

            self.nodes[0].children += 2 + value.nodes[0].children;
            self.nodes[0].length_in_bytes = self.compacted.len() as u32;
            self.nodes.push(Node {
                children: 0,
                length_in_bytes: key.len() as u32 + 2
            });
            let len = self.nodes.len();
            push_all_at(&mut self.nodes, len, &value.nodes);

        }
    }

    pub fn remove_key(&mut self, key: &[u8]) -> Option<Value> {
        if let Some((nodes_range, compacted_range, value_range)) = self.find_key_parts(key) {
            let new_compacted = self.compacted.drain(compacted_range)
                                              .skip(value_range.start)  // Skip key + ':'
                                              .take(value_range.len())  // Skip ',' if trailing
                                              .collect();
            let new_nodes = self.nodes.drain(nodes_range).skip(1).collect();

            let len = self.nodes.len() as u32;
            self.nodes[0].children = len - 1;
            self.nodes[0].length_in_bytes = self.compacted.len() as u32;

            Some(Value {
                nodes: new_nodes,
                compacted: new_compacted,
            })
        } else {
            None
        }
    }

    pub fn discard_key(&mut self, key: &[u8]) {
        if let Some((nodes_range, compacted_range, _)) = self.find_key_parts(key) {
            self.compacted.drain(compacted_range);
            self.nodes.drain(nodes_range);

            let len = self.nodes.len() as u32;
            self.nodes[0].children = len - 1;
            self.nodes[0].length_in_bytes = self.compacted.len() as u32;
        }
    }

    fn find_key_parts(&self, key: &[u8]) -> Option<(Range<usize>, Range<usize>, Range<usize>)> {
        if self.compacted[0] == b'{' {
            let mut it = self.nodes[1..].iter().enumerate();
            let mut offset = 1;
            let mut discard = 0;

            let mut is_first = true;

            loop {
                let (key_idx, key_node) = match it.nth(discard) {
                    Some(n) => n,
                    None => break,
                };

                let value_node = match it.next() {
                    Some(n) => n.1,
                    None => break,
                };

                let key_val_len = (key_node.length_in_bytes + value_node.length_in_bytes + 2) as usize;

                if self.compacted[offset] == b'"' && key == &self.compacted[offset+1..offset-1+key_node.length_in_bytes as usize] {
                    let is_last = self.compacted[offset + key_val_len - 1] != b',';

                    let start_offset = if is_first || !is_last {
                        offset
                    } else {
                        offset - 1
                    };

                    let end_offset = if is_last {
                        offset + key_val_len - 1
                    } else {
                        offset + key_val_len
                    };

                    return Some((
                        key_idx + 1 .. key_idx + value_node.children as usize + 3,
                        start_offset..end_offset,
                        offset - start_offset + key_node.length_in_bytes as usize + 1..offset - start_offset + key_val_len - 1,
                    ));
                } else {
                    offset += key_val_len as usize;
                    is_first = false;
                }

                discard = value_node.children as usize;
            }
        }

        return None;
    }
}

pub fn push_all_at<T>(v: &mut Vec<T>, offset: usize, s: &[T]) where T: Copy {
    match (v.len(), s.len()) {
        (_, 0) => (),
        (current_len, _) => {
            v.reserve(s.len());
            unsafe {
                v.set_len(current_len + s.len());
                let to_move = current_len - offset;
                let src = v.as_mut_ptr().offset(offset as isize);
                if to_move > 0 {
                    let dst = src.offset(s.len() as isize);
                    ptr::copy(src, dst, to_move);
                }
                ptr::copy_nonoverlapping(s.as_ptr(), src, s.len());
            }
        },
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use std::str;

    const TEST_STRING : &'static [u8] = br#"{
        "ab": 12,
        "cd": 34,
        "ef": 56
    }"#;

    #[test]
    fn test_first() {
        let mut value = Value::from_slice(TEST_STRING);

        value.remove_key(b"ab");

        assert_eq!(&value.compacted, br#"{"cd":34,"ef":56}"#);
    }

    #[test]
    fn test_middle() {
        let mut value = Value::from_slice(TEST_STRING);

        value.remove_key(b"cd");

        assert_eq!(&value.compacted, br#"{"ab":12,"ef":56}"#);
    }

    #[test]
    fn test_last() {
        let mut value = Value::from_slice(TEST_STRING);

        value.remove_key(b"ef");

        assert_eq!(&value.compacted, br#"{"ab":12,"cd":34}"#);
    }

    #[test]
    fn test_first_and_last() {
        let mut value = Value::from_slice(br#"{"ab":12}"#);

        value.remove_key(b"ab");

        assert_eq!(&value.compacted, br#"{}"#);
    }

    #[test]
    fn test_multiple() {
        let mut value = Value::from_slice(TEST_STRING);

        value.remove_key(b"ab");
        value.remove_key(b"cd");

        assert_eq!(&value.compacted, br#"{"ef":56}"#);
    }

    #[test]
    fn test_all() {
        let mut value = Value::from_slice(TEST_STRING);

        value.remove_key(b"ab");
        value.remove_key(b"cd");
        value.remove_key(b"ef");

        assert_eq!(&value.compacted, br#"{}"#);
    }

    #[test]
    fn removed_value() {
        let mut value = Value::from_slice(TEST_STRING);

        {
            let removed_value = value.remove_key(b"ab").unwrap();
            assert_eq!(&removed_value.compacted, br#"12"#);
            assert_eq!(&value.compacted, br#"{"cd":34,"ef":56}"#);
        }

        {
            let removed_value = value.remove_key(b"ef").unwrap();
            assert_eq!(&removed_value.compacted, br#"56"#);
            assert_eq!(&value.compacted, br#"{"cd":34}"#);
        }
    }

    #[test]
    fn push_all() {
        let mut test_vec = vec![1,2,3,4,5];
        push_all_at(&mut test_vec, 1, &[7, 8, 9]);

        assert_eq!(&test_vec, &[1,7,8,9,2,3,4,5]);
    }

    #[test]
    fn prepend_key() {
        let mut value = Value::from_slice(TEST_STRING);
        let sub_value = Value::from_slice(br#"{"test":12345}"#);

        assert_eq!(value.nodes.len(), 7);

        value.prepend_key(b"wibble", &sub_value);

        assert_eq!(
            str::from_utf8(&value.compacted[..]).unwrap(),
            &r#"{"wibble":{"test":12345},"ab":12,"cd":34,"ef":56}"#[..]
        );
        assert_eq!(value.nodes[0].children, 10);
        assert_eq!(value.nodes[0].length_in_bytes as usize, value.compacted.len());
        assert_eq!(value.nodes[1].children, 0);
        assert_eq!(value.nodes[1].length_in_bytes as usize, 8);
        assert_eq!(sub_value.nodes.len(), 3);
        assert_eq!(value.nodes.len(), 11);
        assert_eq!(
            value,
            Value::from_slice(&br#"{"wibble":{"test":12345},"ab":12,"cd":34,"ef":56}"#[..])
        );
    }

    #[test]
    fn prepend_key_empty() {
        let mut value = Value::from_slice(br#"{}"#);
        let sub_value = Value::from_slice(br#"{"test":12345}"#);

        value.prepend_key(b"wibble", &sub_value);

        assert_eq!(
            str::from_utf8(&value.compacted[..]).unwrap(),
            &r#"{"wibble":{"test":12345}}"#[..]
        );
        assert_eq!(
            value,
            Value::from_slice(&br#"{"wibble":{"test":12345}}"#[..])
        );
    }

    #[test]
    fn add_key() {
        let mut value = Value::from_slice(TEST_STRING);
        let sub_value = Value::from_slice(br#"{"test":12345}"#);

        assert_eq!(value.nodes.len(), 7);

        value.add_key(b"wibble", &sub_value);

        assert_eq!(
            str::from_utf8(&value.compacted[..]).unwrap(),
            &r#"{"ab":12,"cd":34,"ef":56,"wibble":{"test":12345}}"#[..]
        );
        assert_eq!(value.nodes[0].children, 10);
        assert_eq!(value.nodes[0].length_in_bytes as usize, value.compacted.len());
        assert_eq!(sub_value.nodes.len(), 3);
        assert_eq!(value.nodes.len(), 11);
        assert_eq!(
            value,
            Value::from_slice(&br#"{"ab":12,"cd":34,"ef":56,"wibble":{"test":12345}}"#[..])
        );
    }

    #[test]
    fn add_key_empty() {
        let mut value = Value::from_slice(br#"{}"#);
        let sub_value = Value::from_slice(br#"{"test":12345}"#);

        value.add_key(b"wibble", &sub_value);

        assert_eq!(
            str::from_utf8(&value.compacted[..]).unwrap(),
            &r#"{"wibble":{"test":12345}}"#[..]
        );
        assert_eq!(
            value,
            Value::from_slice(&br#"{"wibble":{"test":12345}}"#[..])
        );
    }

    #[test]
    fn get_key() {
        let value = Value::from_slice(TEST_STRING);
        let (nodes, bytes) = value.get_key(b"cd").unwrap();
        assert_eq!(bytes, b"34");
        assert_eq!(nodes, &Value::from_slice(b"34").nodes[..]);

        let value = Value::from_slice(br#"{"ab":12,"cd":34,"ef":56,"wibble":{"test":12345}}"#);
        let (nodes, bytes) = value.get_key(b"wibble").unwrap();
        assert_eq!(bytes, br#"{"test":12345}"#);
        assert_eq!(nodes, &Value::from_slice(br#"{"test":12345}"#).nodes[..]);
    }
}
