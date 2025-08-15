//! Recognizes URL path patterns with support for dynamic and wildcard segments
//!
//! # Examples
//!
//! ```
//! use pathrouter::{Router, Params};
//!
//! let mut router = Router::new();
//!
//! router.add("/posts", "posts");
//! router.add("/posts/:post_id", "post");
//!
//! let (endpoint, params) = router.route("/posts/1").unwrap();
//!
//! assert_eq!(*endpoint, "post");
//! let mut path_params = Params::new();
//! path_params.insert("post_id", "1");
//! assert_eq!(params, path_params);
//! ```
//!
//! # Routing params
//!
//! The router supports four kinds of route segments:
//! - __segments__: these are of the format `/a/b`.
//! - __params__: these are of the format `/a/:b`.
//! - __wildcards__: these are of the format `/a/*b`.

use std::{
    collections::{BTreeMap, btree_map},
    ops::Index,
};

mod nfa;
mod tree;

/// Recognizes URL path patterns with support for dynamic and wildcard segments.
#[derive(Debug, Clone)]
pub struct Router<T> {
    tree: nfa::Nfa,
    endpoints: BTreeMap<usize, T>,
}

impl<T> Router<T> {
    pub fn new() -> Self {
        Router {
            tree: nfa::Nfa::new(),
            endpoints: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, pattern: &str, endpoint: T) -> &mut Self {
        let state = self.tree.insert(pattern);
        self.endpoints.insert(state, endpoint);
        self
    }

    pub fn route(&self, path: &str) -> Option<(&T, Params)> {
        self.tree.search(path).map(|found| {
            let endpoint = self.endpoints.get(&found.state).unwrap();
            let mut params = Params::new();

            for (n, v) in found.params {
                if !n.is_empty() {
                    params.map.insert(n.to_string(), v.to_string());
                }
            }

            (endpoint, params)
        })
    }

    pub fn merge(&mut self, path: &str, mut other: Router<T>) {
        let path = path.trim_end_matches('/');
        let state = self.tree.locate(path);

        let right = other.tree.start_state();

        let states = self.tree.merge(state, &other.tree, right);

        for (new, old) in states {
            if let Some(ep) = other.endpoints.remove(&old) {
                self.endpoints.insert(new, ep);
            }
        }
    }
}

impl<T: Default> Router<T> {
    pub fn at_or_default(&mut self, path: &str) -> &mut T {
        let state = self.tree.locate(path);
        self.tree.accept(state);

        self.endpoints.entry(state).or_default()
    }
}

impl<T: Default> Default for Router<T> {
    fn default() -> Self {
        Router::new()
    }
}

#[derive(Debug, Clone)]
pub struct TreeRouter<T> {
    tree: crate::tree::Tree<T>,
}

impl<T> TreeRouter<T> {
    pub fn new() -> Self {
        TreeRouter {
            tree: crate::tree::Tree::new(),
        }
    }

    pub fn add(&mut self, pattern: &str, endpoint: T) {
        self.tree.insert(pattern, endpoint);
    }

    pub fn merge(&mut self, path: &str, other: TreeRouter<T>) {
        self.tree.merge(path, other.tree);
    }

    pub fn route(&self, path: &str) -> Option<(&T, Params)> {
        self.tree.search(path).map(|(endpoint, p)| {
            let mut params = Params::new();

            for (_k, (n, v)) in p {
                params.map.insert(n, v);
            }

            (endpoint, params)
        })
    }
}

impl<T: Default> TreeRouter<T> {
    pub fn at_or_default(&mut self, pattern: &str) -> &mut T {
        let endpoint = self.tree.at(pattern);

        let data = &mut endpoint.data;

        match data {
            Some(ep) => ep,
            None => {
                *data = Some(T::default());
                data.as_mut().unwrap()
            }
        }
    }
}

impl<T: Default> Default for TreeRouter<T> {
    fn default() -> Self {
        TreeRouter::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Params {
    map: BTreeMap<String, String>,
}

impl Params {
    pub fn new() -> Self {
        Params {
            map: BTreeMap::new(),
        }
    }

    pub fn find(&self, key: impl AsRef<str>) -> Option<&str> {
        self.map.get(key.as_ref()).map(|s| s.as_str())
    }

    pub fn insert(&mut self, key: impl ToString, value: impl ToString) -> Option<String> {
        self.map.insert(key.to_string(), value.to_string())
    }

    pub fn remove(&mut self, key: impl AsRef<str>) -> Option<String> {
        self.map.remove(key.as_ref())
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn iter(&self) -> ParamIter<'_> {
        ParamIter(self.map.iter())
    }
}

impl Default for Params {
    fn default() -> Self {
        Params::new()
    }
}

impl Index<&str> for Params {
    type Output = String;

    fn index(&self, index: &str) -> &Self::Output {
        match self.map.get(index) {
            Some(s) => s,
            None => {
                panic!("params[{}] did not exist", index)
            }
        }
    }
}

impl<'a> IntoIterator for &'a Params {
    type IntoIter = ParamIter<'a>;
    type Item = (&'a str, &'a str);

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct ParamIter<'a>(btree_map::Iter<'a, String, String>);

impl<'a> Iterator for ParamIter<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_basic_routing() {
        let mut router = Router::new();
        router.add("/hello", "Hello");
        router.add("/world", "World");

        let (endpoint, params) = router.route("/hello").unwrap();
        assert_eq!(*endpoint, "Hello");
        assert!(params.is_empty());
    }

    #[test]
    fn test_param_routing() {
        let mut router = Router::new();
        router.add("/posts/:id", "post_detail");

        let (endpoint, params) = router.route("/posts/123").unwrap();
        assert_eq!(*endpoint, "post_detail");
        assert_eq!(params.find("id"), Some("123"));
    }

    #[test]
    fn test_wildcard_routing() {
        let mut router = Router::new();
        router.add("/static/*path", "static_file");

        let (endpoint, params) = router.route("/static/css/style.css").unwrap();
        assert_eq!(*endpoint, "static_file");
        assert_eq!(params.find("path"), Some("css/style.css"));
    }

    #[test]
    fn test_priority_routing() {
        let mut router = Router::new();
        router.add("/posts/new", "new_post");
        router.add("/posts/:id", "post_detail");

        // 精确匹配优先
        let (endpoint, _) = router.route("/posts/new").unwrap();
        assert_eq!(*endpoint, "new_post");

        // 参数匹配
        let (endpoint, _) = router.route("/posts/123").unwrap();
        assert_eq!(*endpoint, "post_detail");
    }

    #[test]
    fn test_multiple_params() {
        let mut router = Router::new();
        router.add("/users/:user_id/posts/:post_id", "user_post");

        let (endpoint, params) = router.route("/users/123/posts/456").unwrap();
        assert_eq!(*endpoint, "user_post");
        assert_eq!(params.find("user_id"), Some("123"));
        assert_eq!(params.find("post_id"), Some("456"));
    }

    #[test]
    fn test_endpoint_modification() {
        let mut router = Router::new();
        router.add("/config", "default_config");

        // 修改端点值
        *router.at_or_default("/config") = "custom_config";

        let (endpoint, _) = router.route("/config").unwrap();
        assert_eq!(*endpoint, "custom_config");
    }

    #[test]
    fn test_subtree_merging() {
        let mut main_router = Router::new();
        main_router.add("/api/v1", "api_v1");

        let mut sub_router = Router::new();
        sub_router.add("/users", "users");
        sub_router.add("/posts", "posts");

        main_router.merge("/api/v1", sub_router);

        let (endpoint, _) = main_router.route("/api/v1/users").unwrap();
        assert_eq!(*endpoint, "users");

        let (endpoint, _) = main_router.route("/api/v1/posts").unwrap();
        assert_eq!(*endpoint, "posts");
    }

    #[test]
    fn test_edge_cases() {
        let mut router = Router::new();

        // 根路径
        router.add("/", "root");
        let (endpoint, _) = router.route("/").unwrap();
        assert_eq!(*endpoint, "root");

        // 尾部斜杠
        router.add("/trailing/", "trailing_slash");
        let (endpoint, _) = router.route("/trailing/").unwrap();
        assert_eq!(*endpoint, "trailing_slash");
    }

    #[test]
    fn test_collection_endpoint() {
        let mut router: Router<Vec<&str>> = Router::new();
        router.at_or_default("/items").push("item1");
        router.at_or_default("/items").push("item2");

        let (items, _) = router.route("/items").unwrap();
        assert_eq!(*items, vec!["item1", "item2"]);
    }
}
