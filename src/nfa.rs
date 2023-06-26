use std::collections::BTreeMap;

const CHAR_PATH_SEP: char = '/';
const CHAR_PARAM: char = ':';
const CHAR_WILDCARD: char = '*';

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
struct Transitions {
    fast_path: BTreeMap<String, usize>,
    slow_path: Vec<(Pattern, usize)>,
}

impl Transitions {
    fn new() -> Self {
        Transitions {
            fast_path: BTreeMap::new(),
            slow_path: Vec::new(),
        }
    }

    fn insert(&mut self, pat: Pattern, new_state: usize) {
        match pat {
            Pattern::Static(p) => {
                self.fast_path.insert(p, new_state);
            }
            _ => {
                self.slow_path.push((pat, new_state));
            }
        }
    }

    fn find(&self, pat: &Pattern) -> Option<usize> {
        match pat {
            Pattern::Static(p) => self.fast_path.get(p).cloned(),
            _ => {
                for (ref p, state) in &self.slow_path {
                    if p == pat {
                        return Some(*state);
                    }
                }
                None
            }
        }
    }

    fn route<'a: 'b, 'b>(&'a self, seg: &'b str) -> Vec<(usize, Capture<'b>)> {
        match self.fast_path.get(seg) {
            Some(state) => {
                vec![(*state, Capture::Static)]
            }
            None => {
                let mut ret = Vec::with_capacity(self.slow_path.len());
                for (ref p, state) in &self.slow_path {
                    match p {
                        Pattern::Param(name) => {
                            ret.push((*state, Capture::Param(name, seg)));
                        }
                        Pattern::Wildcard(name) => {
                            ret.push((*state, Capture::Wildcard(name, seg)));
                        }
                        Pattern::Static(_) => {
                            unreachable!()
                        }
                    }
                }
                ret
            }
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

    fn start_state(&self) -> usize {
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

            let next = self.get_state(index).transitions.find(&pat);

            match next {
                Some(s) => {
                    index = s;
                }
                None => {
                    let new_state = self.new_state();
                    self.get_state_mut(index).transitions.insert(pat, new_state);

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
        let mut roads = vec![Road::new(self.start_state(), Vec::new())];

        let mut path = path.trim_start_matches(CHAR_PATH_SEP);

        while let Some((seg, reminder)) = path.split_once(CHAR_PATH_SEP) {
            roads = self.process_seg(roads, seg, path);
            path = reminder;
        }

        roads = self.process_seg(roads, path, path);

        if roads.is_empty() {
            return None;
        }

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

            for (next, capture) in self.get_state(state).transitions.route(seg) {
                let mut captures = captures.clone();

                match capture {
                    Capture::Static => {
                        captures.push(capture);
                        returned.push(Road::new(next, captures));
                    }
                    Capture::Param(_name, _value) => {
                        captures.push(capture);
                        returned.push(Road::new(next, captures));
                    }
                    Capture::Wildcard(name, _) => {
                        captures.push(Capture::Wildcard(name, path));
                        let mut road = Road::new(next, captures);
                        road.set_wildcard(true);
                        returned.push(road);
                    }
                }
            }
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
}
