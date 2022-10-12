use std::{
    collections::{btree_map, BTreeMap},
    ops::Index,
};

use tree::Tree;

mod tree;

pub struct Router<T> {
    tree: Tree<T>,
}

impl<T> Router<T> {
    pub fn new() -> Self {
        Router { tree: Tree::new() }
    }

    pub fn add(&mut self, pattern: &str, endpoint: T) {
        self.tree.insert(pattern, endpoint);
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

impl<T: Default> Router<T> {
    pub fn at_or_default(&mut self, pattern: &str) -> &mut T {
        let endpoint = self.tree.at(pattern);

        match endpoint {
            Some(ep) => ep,
            None => {
                *endpoint = Some(T::default());
                endpoint.as_mut().unwrap()
            }
        }
    }
}

impl<T: Default> Default for Router<T> {
    fn default() -> Self {
        Router::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
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

    pub fn iter(&self) -> ParamIter {
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
    fn basic_router() {
        let mut router = Router::new();

        router.add("/hello", "Hello");
        router.add("/hell", "Hell");
        router.add("/world", "World");

        let (endpoint, params) = router.route("/hello").unwrap();

        assert_eq!(*endpoint, "Hello");
        assert_eq!(params, empty_params());
    }

    #[test]
    fn ambiguous_router() {
        let mut router = Router::new();

        router.add("/posts/new", "new");
        router.add("/posts/:id", "id");

        let (endpoint, params) = router.route("/posts/1").unwrap();

        assert_eq!(*endpoint, "id");
        assert_eq!(params, one_params("id", "1"));

        let (endpoint, params) = router.route("/posts/new").unwrap();

        assert_eq!(*endpoint, "new");
        assert_eq!(params, empty_params());
    }

    #[test]
    fn ambiguous_router_b() {
        let mut router = Router::new();

        router.add("/posts/:id", "id");
        router.add("/posts/new", "new");

        let (endpoint, params) = router.route("/posts/1").unwrap();

        assert_eq!(*endpoint, "id");
        assert_eq!(params, one_params("id", "1"));

        let (endpoint, params) = router.route("/posts/new").unwrap();

        assert_eq!(*endpoint, "new");
        assert_eq!(params, empty_params());
    }

    #[test]
    fn multiple_params() {
        let mut router = Router::new();

        router.add("/posts/:post_id/comments/:comment_id", "comment");
        router.add("/posts/:post_id/comments", "comments");

        let (endpoint, params) = router.route("/posts/12/comments/100").unwrap();
        assert_eq!(*endpoint, "comment");
        assert_eq!(params, two_params("post_id", "12", "comment_id", "100"));

        let (endpoint, params) = router.route("/posts/12/comments").unwrap();
        assert_eq!(*endpoint, "comments");
        assert_eq!(params, one_params("post_id", "12"));
        assert_eq!(params["post_id"], "12".to_string());
    }

    #[test]
    fn wildcard_colon() {
        let mut router = Router::new();

        router.add("/a/*b", "ab");
        router.add("/a/:b/c", "abc");
        router.add("/a/:b/c/:d", "abcd");

        let (endpoint, params) = router.route("/a/foo").unwrap();
        assert_eq!(*endpoint, "ab");
        assert_eq!(params, one_params("b", "foo"));

        let (endpoint, params) = router.route("/a/foo/bar").unwrap();
        assert_eq!(*endpoint, "ab");
        assert_eq!(params, one_params("b", "foo/bar"));

        let (endpoint, params) = router.route("/a/foo/c").unwrap();
        assert_eq!(*endpoint, "abc");
        assert_eq!(params, one_params("b", "foo"));
    }

    // TODO: add support different params
    // #[test]
    // fn unnamed_parameters() {
    //     let mut router = Router::new();

    //     router.add("/foo/:/bar", "test");
    //     router.add("/foo/:bar/*", "test2");

    //     let (endpoint, params) = router.route("/foo/test/bar").unwrap();
    //     assert_eq!(*endpoint, "test");
    //     assert_eq!(params, empty_params());

    //     let (endpoint, params) = router.route("/foo/test/blah").unwrap();
    //     assert_eq!(*endpoint, "test2");
    //     assert_eq!(params, one_params("bar", "test"));
    // }

    fn empty_params() -> Params {
        Params::new()
    }

    fn one_params(key: &str, value: &str) -> Params {
        let mut map = Params::new();
        map.insert(key, value);
        map
    }

    fn two_params(k1: &str, v1: &str, k2: &str, v2: &str) -> Params {
        let mut map = Params::new();
        map.insert(k1, v1);
        map.insert(k2, v2);
        map
    }
}
