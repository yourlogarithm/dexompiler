use axmldecoder::{Node, XmlDocument};

pub(crate) fn parse_permissions(contents: Vec<u8>) -> Option<Vec<String>> {
    let xml = match axmldecoder::parse(&contents) {
        Ok(xml) => xml,
        _ => return None
    };
    let XmlDocument { root } = xml;
    if let Some(Node::Element(root)) = root {
        return Some(root.children.into_iter()
        .filter_map(|node| match node {
            Node::Element(mut element) if element.get_tag() == "uses-permission" => {
                element.attributes.remove("android:name")
            },
            _ => None
        })
        .filter_map(|s| match s.strip_prefix("android.permission.") {
            Some(s) => Some(s.to_string()),
            None => None
        })
        .collect())
    }
    None
}
