#![feature(test)]

extern crate pathrouter;
extern crate test;

use pathrouter::{Router, Router2};

#[bench]
fn benchmark_nfa(b: &mut test::Bencher) {
    let mut router = Router::new();

    router.add("/posts", "posts");
    router.add("/posts/:post_id/comments/:id", "comment");
    router.add("/posts/:post_id/comments", "comments");
    router.add("/posts/:post_id", "post");
    router.add("/comments", "comments2");
    router.add("/comments/:id", "comment2");
    router.add("/api/v1/self/profile", "profile");
    router.add("/api/v1/*v1", "v1");

    b.iter(|| {
        router.route("/posts");
        router.route("/posts/100/comments/200");
        router.route("/api/v1/self/profile");
        router.route("/api/v1/user/110/profile");
    });
}

#[bench]
fn benchmark_tree(b: &mut test::Bencher) {
    let mut router = Router2::new();

    router.add("/posts", "posts");
    router.add("/posts/:post_id/comments/:id", "comment");
    router.add("/posts/:post_id/comments", "comments");
    router.add("/posts/:post_id", "post");
    router.add("/comments", "comments2");
    router.add("/comments/:id", "comment2");
    router.add("/api/v1/self/profile", "profile");
    router.add("/api/v1/*v1", "v1");

    b.iter(|| {
        router.route("/posts");
        router.route("/posts/100/comments/200");
        router.route("/api/v1/self/profile");
        router.route("/api/v1/user/110/profile");
    });
}
