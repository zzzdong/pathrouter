#![feature(test)]

extern crate pathrouter;
extern crate test;

use pathrouter::Router;

#[bench]
fn benchmark(b: &mut test::Bencher) {
    let mut router = Router::new();
    router.add("/posts/:post_id/comments/:id", "comment".to_string());
    router.add("/posts/:post_id/comments", "comments".to_string());
    router.add("/posts/:post_id", "post".to_string());
    router.add("/posts", "posts".to_string());
    router.add("/comments", "comments2".to_string());
    router.add("/comments/:id", "comment2".to_string());

    b.iter(|| router.route("/posts/100/comments/200"));
}