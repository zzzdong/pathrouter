use std::collections::BTreeMap;

pub(crate) type ParamMap = BTreeMap<usize, (String, String)>;

const CHAR_PATH_SEP: char = '/';
const CHAR_PARAM: char = ':';
const CHAR_WILDCARD: char = '*';

const PAT_PATH_SEP: &str = "/";
const PAT_PARAM: &str = ":";
const PAT_WILDCARD: &str = "*";

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone)]
pub(crate) struct Node<T> {
    index: usize,
    parent: usize,
    pattern: Pattern,
    children: BTreeMap<String, usize>,
    has_param_child: bool,
    has_wildcard_child: bool,
    pub(crate) data: Option<T>,
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

#[derive(Debug, Clone)]
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

        got.data = Some(data);
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

    pub fn merge(&mut self, path: &str, other: Self) {
        let offset = self.nodes.len() - 1;

        let path = path.trim_end_matches('/');

        let root = self.at(path).index;

        for mut n in other.nodes {
            // skip root
            if n.index == 0 {
                continue;
            }

            if n.parent == 0 {
                n.parent = root;
            } else {
                n.parent += offset;
            }

            let child = self.add_child(n.parent, n.pattern);

            self.get_mut(child).data = n.data;
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
                None => match self.search_closest_wildcard_node(node) {
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

        if self.get(node).data.is_none()
            && let Some(n) = self.search_closest_wildcard_node(node)
        {
            node = n;
        }

        self.get(node).data.as_ref().map(|_| node)
    }

    pub(crate) fn at(&mut self, path: &str) -> &mut Node<T> {
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

        self.get_mut(node)
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
                    if n.has_param_child
                        && let Some(child) = n.children.get(PAT_PARAM)
                    {
                        return Some(child);
                    }
                    if n.has_wildcard_child
                        && let Some(child) = n.children.get(PAT_WILDCARD)
                    {
                        return Some(child);
                    }
                }
            };

            None
        });

        perfect.cloned()
    }

    fn search_closest_wildcard_node(&self, node: usize) -> Option<usize> {
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

    // 测试辅助函数，用于创建测试用的树结构
    fn create_basic_tree() -> Tree<&'static str> {
        let mut tree: Tree<&'static str> = Tree::new();
        tree.insert("/a/b/c", "static_abc");
        tree.insert("/a/b/d", "static_abd");
        tree.insert("/a/c", "static_ac");
        tree.insert("/a/c/:f", "param_acf");
        tree.insert("/h/i/j", "static_hij");
        tree
    }

    // 测试辅助函数，用于创建通配符测试用的树结构
    fn create_wildcard_tree() -> Tree<&'static str> {
        let mut tree: Tree<&'static str> = Tree::new();
        tree.insert("/o/:p/*q", "wildcard_o");
        tree.insert("/r/:s/t", "param_rst");
        tree.insert("/r/*u", "wildcard_ru");
        tree.insert("/*any", "global_wildcard");
        tree
    }

    // 测试辅助函数，用于创建优先级测试用的树结构
    fn create_priority_tree() -> Tree<&'static str> {
        let mut tree: Tree<&'static str> = Tree::new();
        tree.insert("/posts/:post_id/comments/:comment_id", "comment_detail");
        tree.insert("/posts/:post_id/comments", "comments_list");
        tree.insert("/posts/*any", "posts_wildcard");
        tree
    }

    #[test]
    fn test_tree_insert_and_search() {
        let tree = create_basic_tree();

        // 验证静态路径匹配
        let (data, params) = tree.search("/a/b/c").unwrap();
        assert_eq!(*data, "static_abc");
        assert!(params.is_empty());

        let (data, params) = tree.search("/a/b/d").unwrap();
        assert_eq!(*data, "static_abd");
        assert!(params.is_empty());

        let (data, params) = tree.search("/a/c").unwrap();
        assert_eq!(*data, "static_ac");
        assert!(params.is_empty());

        let (data, params) = tree.search("/h/i/j").unwrap();
        assert_eq!(*data, "static_hij");
        assert!(params.is_empty());

        // 验证参数路径匹配
        let (data, params) = tree.search("/a/c/test").unwrap();
        assert_eq!(*data, "param_acf");
        let params = params.into_values().collect::<BTreeMap<String, String>>();
        assert_eq!(params.get("f"), Some(&"test".to_string()));

        // 验证不存在路径
        assert!(tree.search("/unknown").is_none());
    }

    #[test]
    fn test_tree_wildcards() {
        let tree = create_wildcard_tree();

        // 验证通配符匹配
        let (data, params) = tree.search("/o/123/456").unwrap();
        assert_eq!(*data, "wildcard_o");
        let params = params.into_values().collect::<BTreeMap<String, String>>();
        assert_eq!(params.get("p"), Some(&"123".to_string()));
        assert_eq!(params.get("q"), Some(&"456".to_string()));

        // 验证参数路径匹配
        let (data, params) = tree.search("/r/abc/t").unwrap();
        assert_eq!(*data, "param_rst");
        let params = params.into_values().collect::<BTreeMap<String, String>>();
        assert_eq!(params.get("s"), Some(&"abc".to_string()));

        // 验证通配符匹配剩余路径
        let (data, params) = tree.search("/r/xyz/uvw").unwrap();
        assert_eq!(*data, "wildcard_ru");
        let params = params.into_values().collect::<BTreeMap<String, String>>();
        assert_eq!(params.get("u"), Some(&"xyz/uvw".to_string()));

        // 验证全局通配符
        let (data, params) = tree.search("/any/path").unwrap();
        assert_eq!(*data, "global_wildcard");
        let params = params.into_values().collect::<BTreeMap<String, String>>();
        assert_eq!(params.get("any"), Some(&"any/path".to_string()));
    }

    #[test]
    fn test_tree_priority() {
        let tree = create_priority_tree();

        // 精确匹配优先
        let (data, params) = tree.search("/posts/123/comments/456").unwrap();
        assert_eq!(*data, "comment_detail");
        let params = params.into_values().collect::<BTreeMap<String, String>>();
        assert_eq!(params.get("post_id"), Some(&"123".to_string()));
        assert_eq!(params.get("comment_id"), Some(&"456".to_string()));

        // 参数匹配优先于通配符
        let (data, params) = tree.search("/posts/123/comments").unwrap();
        assert_eq!(*data, "comments_list");
        let params = params.into_values().collect::<BTreeMap<String, String>>();
        assert_eq!(params.get("post_id"), Some(&"123".to_string()));

        // 通配符匹配剩余路径
        let (data, params) = tree.search("/posts/123/other").unwrap();
        assert_eq!(*data, "posts_wildcard");
        let params = params.into_values().collect::<BTreeMap<String, String>>();
        assert_eq!(params.get("any"), Some(&"123/other".to_string()));
    }

    #[test]
    fn test_tree_merge() {
        let mut tree1: Tree<&'static str> = Tree::new();
        tree1.insert("/api/v1/users", "users_v1");
        tree1.insert("/api/v1/posts", "posts_v1");

        let mut tree2: Tree<&'static str> = Tree::new();
        tree2.insert("/comments", "comments");
        tree2.insert("/comments/:id", "comment_detail");

        tree1.merge("/api/v1", tree2);

        // 验证原始路径
        let (data, _) = tree1.search("/api/v1/users").unwrap();
        assert_eq!(*data, "users_v1");

        let (data, _) = tree1.search("/api/v1/posts").unwrap();
        assert_eq!(*data, "posts_v1");

        // 验证合并后的路径
        let (data, _) = tree1.search("/api/v1/comments").unwrap();
        assert_eq!(*data, "comments");

        let (data, params) = tree1.search("/api/v1/comments/123").unwrap();
        assert_eq!(*data, "comment_detail");
        let params = params.into_values().collect::<BTreeMap<String, String>>();
        assert_eq!(params.get("id"), Some(&"123".to_string()));
    }

    #[test]
    fn test_tree_edge_cases() {
        let mut tree: Tree<&'static str> = Tree::new();

        // 根路径
        tree.insert("/", "root");
        let (data, _) = tree.search("/").unwrap();
        assert_eq!(*data, "root");

        // 空路径
        tree.insert("", "empty");
        let (data, _) = tree.search("").unwrap();
        assert_eq!(*data, "empty");

        // 尾部斜杠
        tree.insert("/trailing/", "trailing_slash");
        let (data, _) = tree.search("/trailing/").unwrap();
        assert_eq!(*data, "trailing_slash");

        // 多个斜杠
        tree.insert("/multiple//slashes", "multiple_slashes");
        let (data, _) = tree.search("/multiple//slashes").unwrap();
        assert_eq!(*data, "multiple_slashes");
    }

    #[test]
    fn test_tree_overwrite_data() {
        let mut tree: Tree<&'static str> = Tree::new();

        // 插入数据
        tree.insert("/test", "first");
        let (data, _) = tree.search("/test").unwrap();
        assert_eq!(*data, "first");

        // 覆盖数据
        tree.insert("/test", "second");
        let (data, _) = tree.search("/test").unwrap();
        assert_eq!(*data, "second");
    }

    #[test]
    fn test_complex_wildcard_scenarios() {
        let mut tree: Tree<&'static str> = Tree::new();

        // 复杂的通配符场景
        tree.insert("/api/:version/*path", "api_handler");
        tree.insert("/api/v2/specific", "specific_handler");

        // 通配符应该匹配除了特定路径外的所有内容
        let (data, params) = tree.search("/api/v1/anything/goes").unwrap();
        assert_eq!(*data, "api_handler");
        assert_eq!(params.len(), 2);

        // 特定路径应该优先匹配
        let (data, _) = tree.search("/api/v2/specific").unwrap();
        assert_eq!(*data, "specific_handler");
    }

    #[test]
    fn test_nested_parameters() {
        let mut tree: Tree<&'static str> = Tree::new();

        tree.insert("/user/:id/profile/:field", "user_field");
        tree.insert("/user/:id", "user_detail");

        let (data, params) = tree.search("/user/123/profile/email").unwrap();
        assert_eq!(*data, "user_field");
        let params = params.into_values().collect::<BTreeMap<String, String>>();
        assert_eq!(params.get("id"), Some(&"123".to_string()));
        assert_eq!(params.get("field"), Some(&"email".to_string()));

        let (data, params) = tree.search("/user/456").unwrap();
        assert_eq!(*data, "user_detail");
        let params = params.into_values().collect::<BTreeMap<String, String>>();
        assert_eq!(params.get("id"), Some(&"456".to_string()));
    }

    #[test]
    fn test_empty_parameter_names() {
        let mut tree: Tree<&'static str> = Tree::new();

        // 测试空参数名
        tree.insert("/test/:", "empty_param_name");
        let (data, params) = tree.search("/test/value").unwrap();
        assert_eq!(*data, "empty_param_name");
        assert!(params.is_empty()); // 空参数名不应该被捕获

        // 测试空通配符名
        tree.insert("/wildcard/*", "empty_wildcard_name");
        let (data, params) = tree.search("/wildcard/anything/else").unwrap();
        assert_eq!(*data, "empty_wildcard_name");
        assert!(params.is_empty()); // 空通配符名不应该被捕获
    }

    // 新增测试：验证特殊字符处理
    #[test]
    fn test_special_characters_in_paths() {
        let mut tree: Tree<&'static str> = Tree::new();

        // 测试包含特殊字符的路径参数
        tree.insert("/files/:filename", "file_handler");
        let (data, params) = tree.search("/files/document.pdf").unwrap();
        assert_eq!(*data, "file_handler");
        let params = params.into_values().collect::<BTreeMap<String, String>>();
        assert_eq!(params.get("filename"), Some(&"document.pdf".to_string()));

        // 测试包含Unicode字符
        let (data, params) = tree.search("/files/文件.txt").unwrap();
        assert_eq!(*data, "file_handler");
        let params = params.into_values().collect::<BTreeMap<String, String>>();
        assert_eq!(params.get("filename"), Some(&"文件.txt".to_string()));
    }

    // 新增测试：验证复杂合并场景
    #[test]
    fn test_complex_merge_scenarios() {
        let mut tree1: Tree<&'static str> = Tree::new();
        tree1.insert("/api", "api_root");

        let mut tree2: Tree<&'static str> = Tree::new();
        tree2.insert("/v1/users", "users_handler");
        tree2.insert("/v1/posts", "posts_handler");
        tree2.insert("/:version/*path", "version_catch_all");

        tree1.merge("/api", tree2);

        // 验证合并后的静态路径
        let (data, _) = tree1.search("/api/v1/users").unwrap();
        assert_eq!(*data, "users_handler");

        // 验证合并后的参数路径
        let (data, params) = tree1.search("/api/v2/any/path").unwrap();
        assert_eq!(*data, "version_catch_all");
        let params = params.into_values().collect::<BTreeMap<String, String>>();
        assert_eq!(params.get("version"), Some(&"v2".to_string()));
        assert_eq!(params.get("path"), Some(&"any/path".to_string()));
    }

    // 新增测试：验证参数捕获边界情况
    #[test]
    fn test_parameter_capture_edge_cases() {
        let mut tree: Tree<&'static str> = Tree::new();

        // 测试连续参数
        tree.insert("/api/:version/:resource/:id", "api_handler");
        let (data, params) = tree.search("/api/v1/users/123").unwrap();
        assert_eq!(*data, "api_handler");
        let params = params.into_values().collect::<BTreeMap<String, String>>();
        assert_eq!(params.get("version"), Some(&"v1".to_string()));
        assert_eq!(params.get("resource"), Some(&"users".to_string()));
        assert_eq!(params.get("id"), Some(&"123".to_string()));
    }

    // 新增测试：验证通配符边界情况
    #[test]
    fn test_wildcard_edge_cases() {
        let mut tree: Tree<&'static str> = Tree::new();

        // 测试根通配符
        tree.insert("/*path", "root_wildcard");
        let (data, params) = tree.search("/").unwrap();
        assert_eq!(*data, "root_wildcard");
        let params = params.into_values().collect::<BTreeMap<String, String>>();
        assert_eq!(params.get("path"), Some(&"".to_string()));

        // 测试深层通配符
        tree.insert("/deep/*rest", "deep_wildcard");
        let (data, params) = tree.search("/deep/a/b/c/d").unwrap();
        assert_eq!(*data, "deep_wildcard");
        let params = params.into_values().collect::<BTreeMap<String, String>>();
        assert_eq!(params.get("rest"), Some(&"a/b/c/d".to_string()));
    }
}
