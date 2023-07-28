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
    static_segment: BTreeMap<String, usize>,
    param_segment: Option<Entry>,
    wildcard: Option<Entry>,
}

impl Transitions {
    fn new() -> Self {
        Transitions {
            static_segment: BTreeMap::new(),
            param_segment: None,
            wildcard: None,
        }
    }

    fn get(&self, pat: &Pattern) -> Option<usize> {
        match pat {
            Pattern::Static(p) => self.static_segment.get(p).cloned(),
            Pattern::Param(_p) => self.param_segment.as_ref().map(|entry| entry.index),
            Pattern::Wildcard(_p) => self.wildcard.as_ref().map(|entry| entry.index),
        }
    }

    fn push(&mut self, pat: Pattern, index: usize) {
        match pat {
            Pattern::Static(p) => {
                self.static_segment.insert(p, index);
            }
            Pattern::Param(p) => self.param_segment = Some(Entry::new(Pattern::Param(p), index)),
            Pattern::Wildcard(p) => self.wildcard = Some(Entry::new(Pattern::Wildcard(p), index)),
        }
    }

    fn entries(&self) -> Vec<Entry> {
        let mut ret = Vec::new();

        for (k, v) in self.static_segment.iter() {
            ret.push(Entry::new(Pattern::Static(k.to_owned()), *v))
        }

        if let Some(entry) = &self.param_segment {
            ret.push(entry.clone());
        }

        if let Some(entry) = &self.wildcard {
            ret.push(entry.clone());
        }

        ret
    }

    fn capture<'a: 'b, 'b>(&'b self, seg: &'a str, path: &'a str) -> Vec<(Capture, usize)> {
        let mut captures = Vec::new();

        if let Some(index) = self.static_segment.get(seg) {
            captures.push((Capture::Static, *index));
        }

        if let Some(Entry {
            pat: Pattern::Param(name),
            index,
        }) = &self.param_segment
        {
            captures.push((Capture::Param(name, seg), *index));
        }

        if let Some(Entry {
            pat: Pattern::Wildcard(name),
            index,
        }) = &self.wildcard
        {
            captures.push((Capture::Wildcard(name, path), *index));
        }

        captures
    }

    fn capture_static(&self, seg: &str) -> Option<(Capture, usize)> {
        self.static_segment
            .get(seg)
            .map(|next| (Capture::Static, *next))
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

        let mut roads: Vec<Road> = roads
            .into_iter()
            .filter(|road| self.get_acceptance(road.state))
            .collect();

        // detect longest path
        roads.sort_by(|a, b| b.partial_cmp(a).unwrap());

        roads.first().map(|found| {
            let mut params = Vec::new();

            for capture in &found.captures {
                match capture {
                    Capture::Param(n, v) => {
                        params.push((*n, *v));
                    }
                    Capture::Wildcard(n, v) => {
                        params.push((*n, *v));
                    }
                    Capture::Static => {}
                }
            }

            Match::new(found.state, params)
        })
    }

    fn fast_path_search(&self, path: &str) -> Option<Match> {
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

        Some(Match::new(road.state, Vec::new()))
    }

    fn process_static_seg<'a: 'b, 'b>(&'a self, seg: &str, mut road: Road<'b>) -> Option<Road<'b>> {
        self.get_state(road.state)
            .transitions
            .capture_static(seg)
            .map(|(capture, next)| {
                road.state = next;
                road.captures.push(capture);

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
                        return Some(std::cmp::Ordering::Greater)
                    }
                    (Capture::Param(_, _), Capture::Static)
                    | (Capture::Wildcard(_, _), Capture::Static) => {
                        return Some(std::cmp::Ordering::Less)
                    }
                    (Capture::Param(_, _), Capture::Wildcard(_, _)) => {
                        return Some(std::cmp::Ordering::Greater)
                    }
                    (Capture::Wildcard(_, _), Capture::Param(_, _)) => {
                        return Some(std::cmp::Ordering::Less)
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
    fn test_nfa() {
        let mut nfa = Nfa::new();

        nfa.insert("/api/v1/post/tom/daily");
        nfa.insert("/api/v2/post/tom/daily");
        nfa.insert("/api/v1/post/:user/daily");
        nfa.insert("/api/v1/post/*any");

        println!("-> {:?}", nfa);

        let ret = nfa.search("/api/v1/post/tom/daily");

        println!("ret => {:?}", ret);
    }

    #[test]
    fn test_nfa2() {
        let mut nfa = Nfa::new();

        nfa.insert("/posts/:post_id/comments/100");
        nfa.insert("/posts/100/comments/10");

        println!("-> {:?}", nfa);

        let ret = nfa.search("/posts/100/comments/100");

        println!("ret => {:?}", ret);
    }

    #[test]
    fn test_nfa_merge() {
        let mut nfa = Nfa::new();

        nfa.insert("/a/b/c");
        nfa.insert("/a/b/d");
        nfa.insert("/a/b/e");

        let mut other = Nfa::new();

        other.insert("/h/i/j");
        other.insert("/h/i/k");
        other.insert("/h/i/l");

        let sub = nfa.locate("/a");

        nfa.merge(sub, &other, other.start_state());

        println!("-> {:?}", nfa);

        let ret = nfa.search("/a/h/i/k");

        println!("ret => {:?}", ret);
    }
}
