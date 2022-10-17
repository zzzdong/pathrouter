use std::collections::BTreeMap;

pub(crate) type ParamMap = BTreeMap<usize, (String, String)>;

const CHAR_PATH_SEP: char = '/';
const CHAR_PARAM: char = ':';
const CHAR_WILDCARD: char = '*';

const PAT_PATH_SEP: &str = "/";
const PAT_PARAM: &str = ":";
const PAT_WILDCARD: &str = "*";

#[derive(Debug, PartialEq)]
enum Pattern {
    Static(String),
    Param(String),
    Wildcard(String),
}

impl Pattern {
    fn from_str(pat: impl AsRef<str>) -> Self {
        let pat = pat.as_ref();
        match pat.chars().next() {
            Some(CHAR_PARAM) => Pattern::Param(pat[1..].to_owned()),
            Some(CHAR_WILDCARD) => Pattern::Wildcard(pat[1..].to_owned()),
            _ => Pattern::Static(pat.to_owned()),
        }
    }

    fn as_pat(&self) -> &str {
        match self {
            Pattern::Param(_) => PAT_PARAM,
            Pattern::Wildcard(_) => PAT_WILDCARD,
            Pattern::Static(p) => p,
        }
    }
}

impl From<&str> for Pattern {
    fn from(s: &str) -> Self {
        Pattern::from_str(s)
    }
}

#[derive(Debug)]
struct Node<T> {
    index: usize,
    parent: usize,
    pattern: Pattern,
    children: BTreeMap<String, usize>,
    has_param_child: bool,
    has_wildcard_child: bool,
    data: Option<T>,
}

impl<T> Node<T> {
    fn new(index: usize, parent: usize, pat: Pattern) -> Self {
        Node {
            index,
            parent,
            pattern: pat,
            children: BTreeMap::new(),
            has_param_child: false,
            has_wildcard_child: false,
            data: None,
        }
    }
}

#[derive(Debug)]
pub struct Tree<T> {
    nodes: Vec<Node<T>>,
}

impl<T> Tree<T> {
    pub fn new() -> Self {
        let root = Node::new(0, 0, Pattern::from_str(PAT_PATH_SEP));

        Tree { nodes: vec![root] }
    }

    pub fn insert(&mut self, path: &str, data: T) {
        let got = self.at(path);

        *got = Some(data);
    }

    pub fn search(&self, path: &str) -> Option<(&T, ParamMap)> {
        match self.search_node(path) {
            Some(node) => {
                let params = self.capture_params(path, node);

                self.get(node).data.as_ref().map(|data| (data, params))
            }

            None => None,
        }
    }

    fn search_node(&self, path: &str) -> Option<usize> {
        let mut node = self.nodes.first().unwrap().index;

        let mut segs = Segments::new(path);

        while let Some(seg) = segs.next() {
            match self.search_child(node, seg) {
                Some(n) => {
                    if let Pattern::Wildcard(_) = &self.get(n).pattern {
                        // when wildcard, return
                        return Some(n);
                    }

                    node = n;
                }
                None => match self.search_cloest_wildcard_node(node) {
                    Some(n) => {
                        node = n;

                        break;
                    }
                    None => {
                        return None;
                    }
                },
            }
        }

        if self.get(node).data.is_none() {
            if let Some(n) = self.search_cloest_wildcard_node(node) {
                node = n;
            }
        }

        self.get(node).data.as_ref().map(|_| node)
    }

    pub(crate) fn at(&mut self, path: &str) -> &mut Option<T> {
        let mut node = self.nodes.first().unwrap().index;

        let mut segs = Segments::new(path);

        while let Some(seg) = segs.next() {
            let pat = Pattern::from_str(seg);

            match self.get_child(node, &pat) {
                Some(n) => {
                    node = n;
                }
                None => {
                    node = self.add_child(node, pat);
                }
            }
        }

        let end = self.get_mut(node);

        &mut end.data
    }

    fn get(&self, index: usize) -> &Node<T> {
        &self.nodes[index]
    }

    fn get_mut(&mut self, index: usize) -> &mut Node<T> {
        &mut self.nodes[index]
    }

    fn search_child(&self, node: usize, pat: &str) -> Option<usize> {
        let perfect = self.nodes.get(node).and_then(|n| {
            match n.children.get(pat) {
                Some(child) => return Some(child),
                None => {
                    if n.has_param_child {
                        if let Some(child) = n.children.get(PAT_PARAM) {
                            return Some(child);
                        }
                    }
                    if n.has_wildcard_child {
                        if let Some(child) = n.children.get(PAT_WILDCARD) {
                            return Some(child);
                        }
                    }
                }
            };

            None
        });

        perfect.cloned()
    }

    fn search_cloest_wildcard_node(&self, node: usize) -> Option<usize> {
        let mut index = node;

        loop {
            let node = self.get(index);
            if node.index == 0 {
                break;
            }

            if self.get(node.parent).has_wildcard_child {
                let wildcard = self.get(node.parent).children.get(PAT_WILDCARD);
                return wildcard.cloned();
            } else {
                index = node.parent;
            }
        }

        None
    }

    /// Get route path from finished node, only return path when had least one param,
    /// otherwise return route path.
    fn get_route_path(&self, node: usize) -> Vec<usize> {
        let mut path = Vec::new();
        let mut index = node;
        let mut has_param = false;

        loop {
            let node = self.get(index);
            if node.index == 0 {
                break;
            }

            // ignore unamed params
            match &node.pattern {
                Pattern::Param(p) => {
                    if !p.is_empty() {
                        has_param = true;
                    }
                }
                Pattern::Wildcard(p) => {
                    if !p.is_empty() {
                        has_param = true;
                    }
                }
                Pattern::Static(_) => {}
            }

            path.push(index);

            index = node.parent;
        }

        if !has_param {
            return Vec::new();
        }

        path.reverse();

        path
    }

    fn capture_params(&self, path: &str, node: usize) -> ParamMap {
        let mut params: ParamMap = BTreeMap::new();
        let mut segs = Segments::new(path);

        let path = self.get_route_path(node);

        // recapture named params
        for index in &path {
            if let Some(seg) = segs.next() {
                match &self.get(*index).pattern {
                    Pattern::Param(p) => {
                        if !p.is_empty() {
                            params.insert(*index, (p.to_owned(), seg.to_owned()));
                        }
                    }
                    Pattern::Wildcard(p) => {
                        if !p.is_empty() {
                            params.insert(*index, (p.to_owned(), segs.reminder().to_owned()));
                        }
                    }
                    Pattern::Static(_) => {}
                }
            }
        }

        params
    }

    fn get_child(&self, node: usize, pat: &Pattern) -> Option<usize> {
        self.nodes
            .get(node)
            .and_then(|n| n.children.get(pat.as_pat()).cloned())
    }

    fn add_child(&mut self, node: usize, pat: Pattern) -> usize {
        {
            let node = self.get(node);
            if let Some(child) = node.children.get(pat.as_pat()) {
                return *child;
            }
        }

        let mut is_param_child = false;
        let mut is_wildcard_child = true;

        match &pat {
            Pattern::Param(_) => is_param_child = true,
            Pattern::Wildcard(_) => is_wildcard_child = true,
            _ => {}
        }

        let pattern = pat.as_pat().to_owned();
        let child = self.next_node(node, pat);

        let node = self.get_mut(node);

        node.children.insert(pattern, child);
        if is_param_child {
            node.has_param_child = is_param_child;
        }
        if is_wildcard_child {
            node.has_wildcard_child = is_wildcard_child;
        }

        child
    }

    fn next_node(&mut self, parent: usize, pat: Pattern) -> usize {
        let next = self.nodes.len();
        let child = Node::new(next, parent, pat);
        self.nodes.push(child);

        next
    }
}

struct Segments<'a> {
    s: &'a str,
    pos: &'a str,
    is_last: bool,
}

impl<'a> Segments<'a> {
    fn new(s: &'a str) -> Self {
        // skip first `/`
        let s = match s.strip_prefix(CHAR_PATH_SEP) {
            Some(s) => s,
            None => s,
        };

        Segments {
            s,
            pos: s,
            is_last: false,
        }
    }

    fn next(&mut self) -> Option<&str> {
        match self.s.split_once(CHAR_PATH_SEP) {
            Some((seg, s)) => {
                self.pos = self.s;
                self.s = s;

                Some(seg)
            }
            None => {
                if self.is_last {
                    None
                } else {
                    self.is_last = true;
                    self.pos = self.s;
                    Some(self.s)
                }
            }
        }
    }

    fn reminder(&self) -> &str {
        self.pos
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_segments() {
        let input = "/a/bc/d/efg";

        let mut segs = Segments::new(input);

        // while let Some(seg) = segs.next() {
        //     println!("-> {seg}");
        //     // break;
        // }

        for _ in 0..10 {
            let seg = segs.next();

            print!("{seg:?} - ");

            println!("{:?}", segs.reminder());
        }
    }

    #[test]
    fn test_tree() {
        let mut tree: Tree<&'static str> = Tree::new();

        tree.insert("/a/b/c", "/a/b/c");
        tree.insert("/a/b/d", "/a/b/d");
        tree.insert("/a/c", "/a/c");
        tree.insert("/a/c/:f", "/a/c/:f");
        tree.insert("/h/i/j", "/h/i/j");

        tree.insert("/o/:p/*q", "/o/:p/*q");

        tree.insert("/r/:s/t", "/r/:s/t");
        tree.insert("/r/*u", "/r/*u");

        tree.insert("/*", "/*");

        println!("{tree:?}");

        assert_eq!(simple_search(&tree, "/a/b/c"), Some(&"/a/b/c"));
        assert_eq!(simple_search(&tree, "/a/c"), Some(&"/a/c"));
        assert_eq!(simple_search(&tree, "/a/c/f"), Some(&"/a/c/:f"));

        assert_eq!(simple_search(&tree, "/h/i/j"), Some(&"/h/i/j"));

        assert_eq!(simple_search(&tree, "/o/p/q"), Some(&"/o/:p/*q"));

        assert_eq!(simple_search(&tree, "/r/s/t"), Some(&"/r/:s/t"));
        assert_eq!(simple_search(&tree, "/r/uuuuu/vvvv/wwww"), Some(&"/r/*u"));

        assert_eq!(simple_search(&tree, "/e/f/g"), Some(&"/*"));
    }

    #[test]
    fn test_tree_b() {
        let mut tree: Tree<&'static str> = Tree::new();

        tree.insert("/posts/:post_id/comments/:comment_id", "comment");

        println!("{tree:?}");

        tree.insert("/posts/:post_id/comments", "comments");

        println!("{tree:?}");

        assert_eq!(
            simple_search(&tree, "/posts/12/comments/100"),
            Some(&"comment")
        );
    }

    fn simple_search<'a, T>(tree: &'a Tree<T>, path: &str) -> Option<&'a T> {
        tree.search(path).map(|(v, _p)| v)
    }
}
