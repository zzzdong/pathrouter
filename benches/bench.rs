#![feature(test)]

extern crate pathrouter;
extern crate test;

use pathrouter::{Router, TreeRouter};

#[bench]
fn benchmark_nfa(b: &mut test::Bencher) {
    let mut router = Router::new();

    // 静态路由
    router.add("/posts", "posts");
    router.add("/comments", "comments2");
    router.add("/api/v1/self/profile", "profile");
    router.add("/users", "users");
    router.add("/products", "products");
    router.add("/categories", "categories");
    
    // 带参数的路由
    router.add("/posts/:post_id", "post");
    router.add("/posts/:post_id/comments", "comments");
    router.add("/posts/:post_id/comments/:id", "comment");
    router.add("/comments/:id", "comment2");
    router.add("/users/:user_id", "user");
    router.add("/users/:user_id/profile", "user_profile");
    router.add("/products/:product_id", "product");
    router.add("/categories/:category_id/products", "category_products");
    
    // 通配符路由
    router.add("/api/v1/*v1", "v1");
    router.add("/api/v2/*v2", "v2");
    router.add("/files/*filepath", "static_files");
    router.add("/docs/*path", "documentation");
    
    // 复杂嵌套路由
    router.add("/users/:user_id/posts/:post_id/comments/:comment_id", "user_post_comment");
    router.add("/api/v1/organizations/:org_id/teams/:team_id/members/:member_id", "org_team_member");

    b.iter(|| {
        // 静态路由测试
        router.route("/posts");
        router.route("/comments");
        router.route("/api/v1/self/profile");
        router.route("/users");
        router.route("/products");
        router.route("/categories");
        
        // 带参数的路由测试
        router.route("/posts/100");
        router.route("/posts/100/comments");
        router.route("/posts/100/comments/200");
        router.route("/comments/300");
        router.route("/users/123");
        router.route("/users/123/profile");
        router.route("/products/456");
        router.route("/categories/789/products");
        
        // 通配符路由测试
        router.route("/api/v1/user/110/profile");
        router.route("/api/v2/settings/notifications");
        router.route("/files/images/logo.png");
        router.route("/docs/api/reference");
        
        // 复杂嵌套路由测试
        router.route("/users/123/posts/456/comments/789");
        router.route("/api/v1/organizations/10/teams/20/members/30");
        
        // 不匹配的路由测试
        router.route("/nonexistent/path");
        router.route("/api/v3/unknown");
    });
}

#[bench]
fn benchmark_tree(b: &mut test::Bencher) {
    let mut router = TreeRouter::new();

    // 静态路由
    router.add("/posts", "posts");
    router.add("/comments", "comments2");
    router.add("/api/v1/self/profile", "profile");
    router.add("/users", "users");
    router.add("/products", "products");
    router.add("/categories", "categories");
    
    // 带参数的路由
    router.add("/posts/:post_id", "post");
    router.add("/posts/:post_id/comments", "comments");
    router.add("/posts/:post_id/comments/:id", "comment");
    router.add("/comments/:id", "comment2");
    router.add("/users/:user_id", "user");
    router.add("/users/:user_id/profile", "user_profile");
    router.add("/products/:product_id", "product");
    router.add("/categories/:category_id/products", "category_products");
    
    // 通配符路由
    router.add("/api/v1/*v1", "v1");
    router.add("/api/v2/*v2", "v2");
    router.add("/files/*filepath", "static_files");
    router.add("/docs/*path", "documentation");
    
    // 复杂嵌套路由
    router.add("/users/:user_id/posts/:post_id/comments/:comment_id", "user_post_comment");
    router.add("/api/v1/organizations/:org_id/teams/:team_id/members/:member_id", "org_team_member");

    b.iter(|| {
        // 静态路由测试
        router.route("/posts");
        router.route("/comments");
        router.route("/api/v1/self/profile");
        router.route("/users");
        router.route("/products");
        router.route("/categories");
        
        // 带参数的路由测试
        router.route("/posts/100");
        router.route("/posts/100/comments");
        router.route("/posts/100/comments/200");
        router.route("/comments/300");
        router.route("/users/123");
        router.route("/users/123/profile");
        router.route("/products/456");
        router.route("/categories/789/products");
        
        // 通配符路由测试
        router.route("/api/v1/user/110/profile");
        router.route("/api/v2/settings/notifications");
        router.route("/files/images/logo.png");
        router.route("/docs/api/reference");
        
        // 复杂嵌套路由测试
        router.route("/users/123/posts/456/comments/789");
        router.route("/api/v1/organizations/10/teams/20/members/30");
        
        // 不匹配的路由测试
        router.route("/nonexistent/path");
        router.route("/api/v3/unknown");
    });
}

// 测试 NFA 路由器处理大量路由的性能
#[bench]
fn benchmark_nfa_large_route_set(b: &mut test::Bencher) {
    // 创建 NFA 路由器实例
    let mut nfa_router = Router::new();
    
    // 添加大量路由（100个不同的路由）
    for i in 0..100 {
        // 静态路由
        let static_path = format!("/static/path/{}", i);
        nfa_router.add(&static_path, format!("static_{}", i));
        
        // 带参数的路由
        let param_path = format!("/users/{}/items/{}", i, i);
        nfa_router.add(&param_path, format!("param_{}", i));
        
        // 通配符路由
        let wildcard_path = format!("/api/v{}/resources/*path", i % 10 + 1);
        nfa_router.add(&wildcard_path, format!("wildcard_{}", i));
    }
    
    b.iter(|| {
        // 测试静态路由
        for i in 0..20 {
            nfa_router.route(&format!("/static/path/{}", i * 5));
        }
        
        // 测试带参数的路由
        for i in 0..20 {
            nfa_router.route(&format!("/users/{}/items/{}", i * 5, i * 5));
        }
        
        // 测试通配符路由
        for i in 0..10 {
            nfa_router.route(&format!("/api/v{}/resources/some/deep/path", i % 10 + 1));
        }
        
        // 测试不匹配的路由
        nfa_router.route("/this/path/does/not/exist");
    });
}

// 测试 Tree 路由器处理大量路由的性能
#[bench]
fn benchmark_tree_large_route_set(b: &mut test::Bencher) {
    // 创建 Tree 路由器实例
    let mut tree_router = TreeRouter::new();
    
    // 添加大量路由（100个不同的路由）
    for i in 0..100 {
        // 静态路由
        let static_path = format!("/static/path/{}", i);
        tree_router.add(&static_path, format!("static_{}", i));
        
        // 带参数的路由
        let param_path = format!("/users/{}/items/{}", i, i);
        tree_router.add(&param_path, format!("param_{}", i));
        
        // 通配符路由
        let wildcard_path = format!("/api/v{}/resources/*path", i % 10 + 1);
        tree_router.add(&wildcard_path, format!("wildcard_{}", i));
    }
    
    b.iter(|| {
        // 测试静态路由
        for i in 0..20 {
            tree_router.route(&format!("/static/path/{}", i * 5));
        }
        
        // 测试带参数的路由
        for i in 0..20 {
            tree_router.route(&format!("/users/{}/items/{}", i * 5, i * 5));
        }
        
        // 测试通配符路由
        for i in 0..10 {
            tree_router.route(&format!("/api/v{}/resources/some/deep/path", i % 10 + 1));
        }
        
        // 测试不匹配的路由
        tree_router.route("/this/path/does/not/exist");
    });
}

// 测试 NFA 路由器参数提取的性能
#[bench]
fn benchmark_nfa_param_extraction(b: &mut test::Bencher) {
    // 创建 NFA 路由器实例
    let mut nfa_router = Router::new();
    
    // 添加各种复杂度的参数路由
    nfa_router.add("/api/users/:user_id", "user");
    nfa_router.add("/api/users/:user_id/posts/:post_id", "user_post");
    nfa_router.add("/api/users/:user_id/posts/:post_id/comments/:comment_id", "user_post_comment");
    nfa_router.add("/api/organizations/:org_id/teams/:team_id/members/:member_id/roles/:role_id", "org_team_member_role");
    nfa_router.add("/api/products/:category/:subcategory/:product_id/:variant_id/:sku", "product_detail");
    
    // 测试 NFA 路由器的参数提取
    b.iter(|| {
        // 测试不同复杂度的参数提取
        nfa_router.route("/api/users/123");
        nfa_router.route("/api/users/123/posts/456");
        nfa_router.route("/api/users/123/posts/456/comments/789");
        nfa_router.route("/api/organizations/10/teams/20/members/30/roles/40");
        nfa_router.route("/api/products/electronics/computers/laptop-123/silver/SKU-98765");
    });
}

// 测试 Tree 路由器参数提取的性能
#[bench]
fn benchmark_tree_param_extraction(b: &mut test::Bencher) {
    // 创建 Tree 路由器实例
    let mut tree_router = TreeRouter::new();
    
    // 添加各种复杂度的参数路由
    tree_router.add("/api/users/:user_id", "user");
    tree_router.add("/api/users/:user_id/posts/:post_id", "user_post");
    tree_router.add("/api/users/:user_id/posts/:post_id/comments/:comment_id", "user_post_comment");
    tree_router.add("/api/organizations/:org_id/teams/:team_id/members/:member_id/roles/:role_id", "org_team_member_role");
    tree_router.add("/api/products/:category/:subcategory/:product_id/:variant_id/:sku", "product_detail");
    
    // 测试 Tree 路由器的参数提取
    b.iter(|| {
        // 测试不同复杂度的参数提取
        tree_router.route("/api/users/123");
        tree_router.route("/api/users/123/posts/456");
        tree_router.route("/api/users/123/posts/456/comments/789");
        tree_router.route("/api/organizations/10/teams/20/members/30/roles/40");
        tree_router.route("/api/products/electronics/computers/laptop-123/silver/SKU-98765");
    });
}

// NFA 路由器性能测试
#[bench]
fn benchmark_nfa_comparison(b: &mut test::Bencher) {
    // 创建 NFA 路由器实例
    let mut nfa_router = Router::new();
    
    // 静态路由
    let static_routes = vec![
        "/", 
        "/about", 
        "/contact", 
        "/api/status", 
        "/api/v1/health", 
        "/products/featured",
        "/blog/recent",
        "/services/pricing",
        "/auth/login",
        "/auth/register"
    ];
    
    // 参数路由
    let param_routes = vec![
        "/users/:id",
        "/products/:product_id",
        "/categories/:category_id",
        "/posts/:year/:month/:day/:slug",
        "/api/v1/users/:user_id/profile",
        "/orders/:order_id/items/:item_id",
        "/regions/:country/:state/:city",
        "/events/:event_id/tickets/:ticket_type"
    ];
    
    // 通配符路由
    let wildcard_routes = vec![
        "/assets/*filepath",
        "/images/*path",
        "/api/v1/*rest",
        "/docs/*path",
        "/downloads/*filename"
    ];
    
    // 添加所有路由到 NFA 路由器
    for route in static_routes.iter() {
        nfa_router.add(route, route);
    }
    
    for route in param_routes.iter() {
        nfa_router.add(route, route);
    }
    
    for route in wildcard_routes.iter() {
        nfa_router.add(route, route);
    }
    
    // 准备测试路径
    let test_paths = vec![
        // 静态路由测试
        "/",
        "/about",
        "/contact",
        "/api/status",
        "/api/v1/health",
        
        // 参数路由测试
        "/users/123",
        "/products/456",
        "/categories/electronics",
        "/posts/2023/08/15/rust-performance",
        "/api/v1/users/789/profile",
        
        // 通配符路由测试
        "/assets/css/main.css",
        "/images/logo.png",
        "/api/v1/custom/endpoint",
        "/docs/getting-started/installation",
        
        // 不匹配的路由
        "/this/does/not/exist",
        "/api/v3/unknown"
    ];
    
    b.iter(|| {
        // 测试 NFA 路由器
        for path in test_paths.iter() {
            let _ = nfa_router.route(path);
        }
    });
}

// Tree 路由器性能测试
#[bench]
fn benchmark_tree_comparison(b: &mut test::Bencher) {
    // 创建 Tree 路由器实例
    let mut tree_router = TreeRouter::new();
    
    // 静态路由
    let static_routes = vec![
        "/", 
        "/about", 
        "/contact", 
        "/api/status", 
        "/api/v1/health", 
        "/products/featured",
        "/blog/recent",
        "/services/pricing",
        "/auth/login",
        "/auth/register"
    ];
    
    // 参数路由
    let param_routes = vec![
        "/users/:id",
        "/products/:product_id",
        "/categories/:category_id",
        "/posts/:year/:month/:day/:slug",
        "/api/v1/users/:user_id/profile",
        "/orders/:order_id/items/:item_id",
        "/regions/:country/:state/:city",
        "/events/:event_id/tickets/:ticket_type"
    ];
    
    // 通配符路由
    let wildcard_routes = vec![
        "/assets/*filepath",
        "/images/*path",
        "/api/v1/*rest",
        "/docs/*path",
        "/downloads/*filename"
    ];
    
    // 添加所有路由到 Tree 路由器
    for route in static_routes.iter() {
        tree_router.add(route, route);
    }
    
    for route in param_routes.iter() {
        tree_router.add(route, route);
    }
    
    for route in wildcard_routes.iter() {
        tree_router.add(route, route);
    }
    
    // 准备测试路径
    let test_paths = vec![
        // 静态路由测试
        "/",
        "/about",
        "/contact",
        "/api/status",
        "/api/v1/health",
        
        // 参数路由测试
        "/users/123",
        "/products/456",
        "/categories/electronics",
        "/posts/2023/08/15/rust-performance",
        "/api/v1/users/789/profile",
        
        // 通配符路由测试
        "/assets/css/main.css",
        "/images/logo.png",
        "/api/v1/custom/endpoint",
        "/docs/getting-started/installation",
        
        // 不匹配的路由
        "/this/does/not/exist",
        "/api/v3/unknown"
    ];
    
    b.iter(|| {
        // 测试 Tree 路由器
        for path in test_paths.iter() {
            let _ = tree_router.route(path);
        }
    });
}

// 测试 NFA 路由器的路由冲突处理性能
#[bench]
fn benchmark_nfa_route_conflicts(b: &mut test::Bencher) {
    // 创建 NFA 路由器实例
    let mut nfa_router = Router::new();
    
    // 添加可能产生冲突的路由
    // 1. 静态路由与参数路由冲突
    nfa_router.add("/users/admin", "admin_user");
    nfa_router.add("/users/:user_id", "normal_user");
    
    // 2. 参数路由与通配符路由冲突
    nfa_router.add("/files/:file_id", "specific_file");
    nfa_router.add("/files/*path", "any_file");
    
    // 3. 多参数路由冲突
    nfa_router.add("/api/:version/users/:user_id", "api_user");
    nfa_router.add("/api/v1/users/:user_id", "v1_api_user");
    
    b.iter(|| {
        // 测试静态路由与参数路由冲突
        nfa_router.route("/users/admin");
        nfa_router.route("/users/123");
        
        // 测试参数路由与通配符路由冲突
        nfa_router.route("/files/document.pdf");
        nfa_router.route("/files/documents/report.pdf");
        
        // 测试多参数路由冲突
        nfa_router.route("/api/v1/users/123");
        nfa_router.route("/api/v2/users/456");
    });
}

// 测试 Tree 路由器的路由冲突处理性能
#[bench]
fn benchmark_tree_route_conflicts(b: &mut test::Bencher) {
    // 创建 Tree 路由器实例
    let mut tree_router = TreeRouter::new();
    
    // 添加可能产生冲突的路由
    // 1. 静态路由与参数路由冲突
    tree_router.add("/users/admin", "admin_user");
    tree_router.add("/users/:user_id", "normal_user");
    
    // 2. 参数路由与通配符路由冲突
    tree_router.add("/files/:file_id", "specific_file");
    tree_router.add("/files/*path", "any_file");
    
    // 3. 多参数路由冲突
    tree_router.add("/api/:version/users/:user_id", "api_user");
    tree_router.add("/api/v1/users/:user_id", "v1_api_user");
    
    b.iter(|| {
        // 测试静态路由与参数路由冲突
        tree_router.route("/users/admin");
        tree_router.route("/users/123");
        
        // 测试参数路由与通配符路由冲突
        tree_router.route("/files/document.pdf");
        tree_router.route("/files/documents/report.pdf");
        
        // 测试多参数路由冲突
        tree_router.route("/api/v1/users/123");
        tree_router.route("/api/v2/users/456");
    });
}