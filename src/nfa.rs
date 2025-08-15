use std::collections::BTreeMap;

const CHAR_PATH_SEP: char = '/';
const CHAR_PARAM: char = ':';
const CHAR_WILDCARD: char = '*';

#[derive(Debug, Clone)]
struct Entry {
    pat: Pattern,
    index: usize,
}

impl Entry {
    fn new(pat: Pattern, index: usize) -> Self {
        Entry { pat, index }
    }
}

#[derive(Debug, Clone)]
struct Transitions {
    static_entries: BTreeMap<String, usize>,
    dynamic_entries: Vec<Entry>,
}

impl Transitions {
    fn new() -> Self {
        Transitions {
            static_entries: BTreeMap::new(),
            dynamic_entries: Vec::new(),
        }
    }

    fn get(&self, pat: &Pattern) -> Option<usize> {
        match pat {
            Pattern::Static(p) => self.static_entries.get(p).cloned(),
            _ => {
                for entry in &self.dynamic_entries {
                    if &entry.pat == pat {
                        return Some(entry.index);
                    }
                }
                None
            }
        }
    }

    fn push(&mut self, pat: Pattern, index: usize) {
        match pat {
            Pattern::Static(p) => {
                self.static_entries.insert(p, index);
            }
            p => {
                self.dynamic_entries.push(Entry::new(p, index));
            }
        }
    }

    fn entries(&self) -> Vec<Entry> {
        let mut ret = Vec::new();

        for (k, v) in self.static_entries.iter() {
            ret.push(Entry::new(Pattern::Static(k.to_owned()), *v))
        }

        ret.extend_from_slice(&self.dynamic_entries);

        ret
    }

    fn capture<'a: 'b, 'b>(&'b self, seg: &'a str, path: &'a str) -> Vec<(Capture<'b>, usize)> {
        let mut captures = Vec::new();

        if let Some(index) = self.static_entries.get(seg) {
            captures.push((Capture::Static, *index));
        }

        for entry in &self.dynamic_entries {
            match &entry.pat {
                Pattern::Param(name) => {
                    if seg.is_empty() {
                        continue;
                    }
                    captures.push((Capture::Param(name, seg), entry.index));
                }
                Pattern::Wildcard(name) => {
                    captures.push((Capture::Wildcard(name, path), entry.index));
                }
                _ => unreachable!(),
            }
        }

        captures
    }

    fn capture_static(&self, seg: &str) -> Option<usize> {
        self.static_entries.get(seg).copied()
    }
}

#[derive(Debug, Clone)]
struct State {
    index: usize,
    transitions: Transitions,
}

impl State {
    fn new(index: usize) -> Self {
        State {
            index,
            transitions: Transitions::new(),
        }
    }
}

#[derive(Debug, Clone)]
enum Pattern {
    Static(String),
    Param(String),
    Wildcard(String),
}

impl PartialEq for Pattern {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Static(l0), Self::Static(r0)) => l0 == r0,
            (Self::Param(_l0), Self::Param(_r0)) => true,
            (Self::Wildcard(_l0), Self::Wildcard(_r0)) => true,
            _ => false,
        }
    }
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
}

#[derive(Debug, Clone)]
pub struct Nfa {
    states: Vec<State>,
    acceptances: Vec<bool>,
}

impl Nfa {
    pub fn new() -> Self {
        let mut this = Nfa {
            states: Vec::new(),
            acceptances: Vec::new(),
        };

        this.new_state();

        this
    }

    fn new_state(&mut self) -> usize {
        let new_index = self.states.len();

        let new_state = State::new(new_index);

        self.states.push(new_state);
        self.acceptances.push(false);

        new_index
    }

    pub(crate) fn start_state(&self) -> usize {
        self.states.first().expect("first state not exist").index
    }

    fn get_state(&self, index: usize) -> &State {
        &self.states[index]
    }

    fn get_state_mut(&mut self, index: usize) -> &mut State {
        self.states.get_mut(index).expect("state not exist")
    }

    fn get_acceptance(&self, state: usize) -> bool {
        self.acceptances[state]
    }

    pub fn locate(&mut self, path: &str) -> usize {
        let path = path.trim_start_matches(CHAR_PATH_SEP);
        let segs = path.split(CHAR_PATH_SEP);

        let mut index = self.start_state();

        for seg in segs {
            let pat = Pattern::from_str(seg);

            let next = self.get_state(index).transitions.get(&pat);

            match next {
                Some(s) => {
                    index = s;
                }
                None => {
                    let new_state = self.new_state();
                    self.get_state_mut(index).transitions.push(pat, new_state);

                    index = new_state;
                }
            }
        }

        index
    }

    pub fn accept(&mut self, state: usize) {
        if state != self.start_state() {
            self.acceptances[state] = true;
        }
    }

    pub fn insert(&mut self, path: &str) -> usize {
        let state = self.locate(path);
        self.accept(state);
        state
    }

    pub fn search<'a: 'b, 'b>(&'a self, path: &'b str) -> Option<Match<'b>> {
        if path.is_empty() {
            return None;
        }
        let mut path = path.trim_start_matches(CHAR_PATH_SEP);

        // try fast path, only match static transition
        if let Some(ret) = self.fast_path_search(path) {
            return Some(ret);
        }

        let mut roads = vec![Road::new(self.start_state(), Vec::new())];
        while let Some((seg, reminder)) = path.split_once(CHAR_PATH_SEP) {
            roads = self.process_seg(roads, seg, path);
            path = reminder;
        }

        roads = self.process_seg(roads, path, path);

        let roads = roads
            .into_iter()
            .filter(|road| self.get_acceptance(road.state));

        // detect longest path
        let found = roads.fold(None, |prev, curr| match prev {
            Some(item) => {
                if item < curr {
                    Some(curr)
                } else {
                    Some(item)
                }
            }
            None => Some(curr),
        });

        found.map(|found| {
            let mut params = Vec::new();
            for capture in found.captures {
                match capture {
                    Capture::Param(n, v) => {
                        params.push((n, v));
                    }
                    Capture::Wildcard(n, v) => {
                        params.push((n, v));
                    }
                    Capture::Static => {}
                }
            }

            Match::new(found.state, params)
        })
    }

    fn fast_path_search(&self, path: &str) -> Option<Match<'_>> {
        let mut road = Road::new(self.start_state(), Vec::new());
        for seg in path.split(CHAR_PATH_SEP) {
            match self.process_static_seg(seg, road) {
                Some(r) => {
                    road = r;
                }
                None => {
                    return None;
                }
            }
        }

        if self.get_acceptance(road.state) {
            return Some(Match::new(road.state, Vec::new()));
        }

        None
    }

    fn process_static_seg<'a: 'b, 'b>(&'a self, seg: &str, mut road: Road<'b>) -> Option<Road<'b>> {
        self.get_state(road.state)
            .transitions
            .capture_static(seg)
            .map(|next| {
                road.state = next;
                road
            })
    }

    fn process_seg<'a: 'b, 'b>(
        &'a self,
        roads: Vec<Road<'a>>,
        seg: &'b str,
        path: &'b str,
    ) -> Vec<Road<'b>> {
        let mut returned = Vec::with_capacity(roads.len());

        for r in roads {
            // while into wildcard, skip it
            if r.wildcard {
                returned.push(r);
                continue;
            }

            let Road {
                state, captures, ..
            } = r;

            for (capture, next) in self.get_state(state).transitions.capture(seg, path) {
                let mut new_captures = captures.clone();
                match capture {
                    Capture::Wildcard(_name, _param) => {
                        new_captures.push(capture);
                        let mut road = Road::new(next, new_captures);
                        road.set_wildcard(true);
                        returned.push(road);
                    }
                    _ => {
                        new_captures.push(capture);
                        returned.push(Road::new(next, new_captures));
                    }
                }
            }
        }

        returned
    }

    pub(crate) fn merge(&mut self, left: usize, other: &Self, right: usize) -> Vec<(usize, usize)> {
        let mut returned = Vec::new();

        for Entry { pat, index: old } in other.get_state(right).transitions.entries() {
            let new_state = self.new_state();
            if other.get_acceptance(old) {
                self.accept(new_state);
            }
            self.get_state_mut(left)
                .transitions
                .push(pat.clone(), new_state);

            returned.push((new_state, old));

            returned.extend(self.merge(new_state, other, old));
        }

        returned
    }
}

#[derive(Debug)]
pub struct Match<'a> {
    pub state: usize,
    pub params: Vec<(&'a str, &'a str)>,
}

impl<'a> Match<'a> {
    fn new(state: usize, params: Vec<(&'a str, &'a str)>) -> Self {
        Match { state, params }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Capture<'a> {
    Static,
    Param(&'a str, &'a str),
    Wildcard(&'a str, &'a str),
}

#[derive(Debug, PartialEq)]
struct Road<'a> {
    state: usize,
    captures: Vec<Capture<'a>>,
    wildcard: bool,
}

impl<'a> Road<'a> {
    fn new(state: usize, captures: Vec<Capture<'a>>) -> Self {
        Road {
            state,
            captures,
            wildcard: false,
        }
    }

    fn set_wildcard(&mut self, wildcard: bool) {
        self.wildcard = wildcard;
    }
}

impl<'a> PartialOrd for Road<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.captures.len() == other.captures.len() {
            for (a, b) in self.captures.iter().zip(other.captures.iter()) {
                match (a, b) {
                    (Capture::Static, Capture::Param(_, _))
                    | (Capture::Static, Capture::Wildcard(_, _)) => {
                        return Some(std::cmp::Ordering::Greater);
                    }
                    (Capture::Param(_, _), Capture::Static)
                    | (Capture::Wildcard(_, _), Capture::Static) => {
                        return Some(std::cmp::Ordering::Less);
                    }
                    (Capture::Param(_, _), Capture::Wildcard(_, _)) => {
                        return Some(std::cmp::Ordering::Greater);
                    }
                    (Capture::Wildcard(_, _), Capture::Param(_, _)) => {
                        return Some(std::cmp::Ordering::Less);
                    }
                    _ => continue,
                }
            }
            None
        } else {
            self.captures.len().partial_cmp(&other.captures.len())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_static_paths() {
        let mut nfa = Nfa::new();

        // 插入静态路径
        let state1 = nfa.insert("/api/v1/users");
        let state2 = nfa.insert("/api/v1/posts");
        let state3 = nfa.insert("/");

        // 验证路径匹配
        let result1 = nfa.search("/api/v1/users");
        assert!(result1.is_some());
        assert_eq!(result1.unwrap().state, state1);

        let result2 = nfa.search("/api/v1/posts");
        assert!(result2.is_some());
        assert_eq!(result2.unwrap().state, state2);

        // 验证根路径
        let result3 = nfa.search("/");
        assert!(result3.is_some());
        assert_eq!(result3.unwrap().state, state3);

        // 验证不存在路径
        assert!(nfa.search("/api/v1/comments").is_none());
        assert!(nfa.search("").is_none());
    }

    #[test]
    fn test_param_paths() {
        let mut nfa = Nfa::new();

        // 插入参数路径
        let state1 = nfa.insert("/api/v1/users/:id");
        let state2 = nfa.insert("/posts/:post_id/comments/:comment_id");

        // 验证参数匹配
        let result = nfa.search("/api/v1/users/123");
        assert!(result.is_some());
        let matched = result.unwrap();
        assert_eq!(matched.state, state1);
        assert_eq!(matched.params, vec![("id", "123")]);

        // 验证不同参数值
        let result2 = nfa.search("/api/v1/users/abc");
        assert!(result2.is_some());
        let matched2 = result2.unwrap();
        assert_eq!(matched2.state, state1);
        assert_eq!(matched2.params, vec![("id", "abc")]);

        // 验证多参数路径
        let result3 = nfa.search("/posts/100/comments/200");
        assert!(result3.is_some());
        let matched3 = result3.unwrap();
        assert_eq!(matched3.state, state2);
        assert_eq!(
            matched3.params,
            vec![("post_id", "100"), ("comment_id", "200")]
        );

        // 验证空参数段
        assert!(nfa.search("/api/v1/users/").is_none());
        assert!(nfa.search("/posts//comments/200").is_none());
    }

    #[test]
    fn test_wildcard_paths() {
        let mut nfa = Nfa::new();

        // 插入通配符路径
        let state1 = nfa.insert("/api/v1/assets/*path");
        let state2 = nfa.insert("/*any");

        // 验证通配符匹配
        let result = nfa.search("/api/v1/assets/css/style.css");
        assert!(result.is_some());
        let matched = result.unwrap();
        assert_eq!(matched.state, state1);
        assert_eq!(matched.params, vec![("path", "css/style.css")]);

        // 验证尾部斜杠处理
        let result2 = nfa.search("/api/v1/assets/");
        assert!(result2.is_some());
        let matched2 = result2.unwrap();
        assert_eq!(matched2.state, state1);
        assert_eq!(matched2.params, vec![("path", "")]);

        // 验证全局通配符
        let result3 = nfa.search("/any/path/here");
        assert!(result3.is_some());
        let matched3 = result3.unwrap();
        assert_eq!(matched3.state, state2);
        assert_eq!(matched3.params, vec![("any", "any/path/here")]);

        // 验证根路径匹配
        let result4 = nfa.search("/");
        assert!(result4.is_some());
        let matched4 = result4.unwrap();
        assert_eq!(matched4.state, state2);
        assert_eq!(matched4.params, vec![("any", "")]);
    }

    #[test]
    fn test_mixed_patterns() {
        let mut nfa = Nfa::new();

        // 插入混合模式路径
        let state1 = nfa.insert("/api/v1/users/:id/posts");
        let state2 = nfa.insert("/api/v1/users/:id/posts/*slug");
        let state3 = nfa.insert("/static/:category/*filepath");

        // 验证精确匹配
        let result1 = nfa.search("/api/v1/users/123/posts");
        assert!(result1.is_some());
        let matched1 = result1.unwrap();
        assert_eq!(matched1.state, state1);
        assert_eq!(matched1.params, vec![("id", "123")]);

        // 验证通配符匹配
        let result2 = nfa.search("/api/v1/users/123/posts/images/banner.png");
        assert!(result2.is_some());
        let matched2 = result2.unwrap();
        assert_eq!(matched2.state, state2);
        assert_eq!(
            matched2.params,
            vec![("id", "123"), ("slug", "images/banner.png")]
        );

        // 验证混合参数和通配符
        let result3 = nfa.search("/static/css/styles/main.css");
        assert!(result3.is_some());
        let matched3 = result3.unwrap();
        assert_eq!(matched3.state, state3);
        assert_eq!(
            matched3.params,
            vec![("category", "css"), ("filepath", "styles/main.css")]
        );

        // 验证部分匹配
        assert!(nfa.search("/api/v1/users/123").is_none());
    }

    #[test]
    fn test_priority_matching() {
        let mut nfa = Nfa::new();

        // 插入具有相同前缀但不同优先级的路径
        let static_state = nfa.insert("/api/v1/special");
        let param_state = nfa.insert("/api/v1/:param");
        let wildcard_state = nfa.insert("/api/v1/*any");

        // 静态路径应优先匹配
        let result = nfa.search("/api/v1/special");
        assert!(result.is_some());
        assert_eq!(result.unwrap().state, static_state);

        // 参数路径匹配其他值
        let result2 = nfa.search("/api/v1/anything");
        assert!(result2.is_some());
        let matched2 = result2.unwrap();
        assert_eq!(matched2.state, param_state);
        assert_eq!(matched2.params, vec![("param", "anything")]);

        // 通配符路径应最后匹配
        let result3 = nfa.search("/api/v1/other/path");
        assert!(result3.is_some());
        let matched3 = result3.unwrap();
        assert_eq!(matched3.state, wildcard_state);
        assert_eq!(matched3.params, vec![("any", "other/path")]);

        // 验证更具体的路径优先
        let specific_state = nfa.insert("/api/v1/special/detail");
        let result4 = nfa.search("/api/v1/special/detail");
        assert!(result4.is_some());
        assert_eq!(result4.unwrap().state, specific_state);
    }

    #[test]
    fn test_merge_functionality() {
        let mut nfa = Nfa::new();
        nfa.insert("/a/b/c");
        nfa.insert("/a/b/d");

        let mut other = Nfa::new();
        other.insert("/h/i/j");
        other.insert("/h/i/k");
        other.insert("/h/:param");

        let sub = nfa.locate("/a/b");
        nfa.merge(sub, &other, other.start_state());

        // 验证合并后的路径匹配
        let result1 = nfa.search("/a/b/h/i/j");
        assert!(result1.is_some());
        let matched1 = result1.unwrap();
        assert_eq!(matched1.params.len(), 0);

        let result2 = nfa.search("/a/b/h/i/k");
        assert!(result2.is_some());
        let matched2 = result2.unwrap();
        assert_eq!(matched2.params.len(), 0);

        // 验证参数路径
        let result3 = nfa.search("/a/b/h/123");
        assert!(result3.is_some());
        let matched3 = result3.unwrap();
        assert_eq!(matched3.params, vec![("param", "123")]);

        // 验证原始路径仍然存在
        let result4 = nfa.search("/a/b/c");
        assert!(result4.is_some());
        let result5 = nfa.search("/a/b/d");
        assert!(result5.is_some());
    }

    #[test]
    fn test_edge_cases() {
        let mut nfa = Nfa::new();

        // 测试空路径
        assert!(nfa.search("").is_none());

        // 测试只有斜杠的路径
        let root_state = nfa.insert("/");
        let result = nfa.search("/");
        assert!(result.is_some());
        assert_eq!(result.unwrap().state, root_state);

        // 测试多个连续斜杠
        nfa.insert("/api//v1/users");
        let result2 = nfa.search("/api//v1/users");
        assert!(result2.is_some());

        // 测试空参数
        let state = nfa.insert("/api/:param");
        let result3 = nfa.search("/api/");
        assert!(result3.is_none());

        // 测试带特殊字符的路径
        nfa.insert("/files/:name");
        let result4 = nfa.search("/files/image%20with%20space.jpg");
        assert!(result4.is_some());
        let matched4 = result4.unwrap();
        assert_eq!(matched4.params, vec![("name", "image%20with%20space.jpg")]);
    }

    #[test]
    fn test_acceptance_logic() {
        let mut nfa = Nfa::new();

        // 插入但不标记为接受状态
        let state1 = nfa.locate("/api/v1/users");

        // 验证未接受状态
        assert!(!nfa.get_acceptance(state1));
        println!("-> {:?}", nfa.search("/api/v1/users"));
        assert!(nfa.search("/api/v1/users").is_none());

        // 标记为接受状态
        nfa.accept(state1);
        assert!(nfa.get_acceptance(state1));

        // 现在应该能匹配
        let result = nfa.search("/api/v1/users");
        assert!(result.is_some());
        assert_eq!(result.unwrap().state, state1);
    }

    #[test]
    fn test_fast_path_search() {
        let mut nfa = Nfa::new();
        nfa.insert("/static/css/style.css");
        nfa.insert("/static/js/app.js");

        // 验证快速路径匹配
        let result1 = nfa.search("/static/css/style.css");
        assert!(result1.is_some());

        let result2 = nfa.search("/static/js/app.js");
        assert!(result2.is_some());

        // 验证非静态路径不会使用快速路径
        nfa.insert("/dynamic/:id");
        let result3 = nfa.search("/dynamic/123");
        assert!(result3.is_some());
    }
}
