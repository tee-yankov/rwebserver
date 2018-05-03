extern crate regex;
pub mod threadpool;
use std::net::{TcpStream};
use std::io::{Read, Write};
use std::net::{TcpListener};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::collections::HashMap;
use regex::Regex;

type RouteHandler = Box<Fn(Option<&Request>) -> Response + Send + Sync + 'static>;
#[derive(Clone)]
pub enum Path {
    Str(String),
    Rex(Regex)
}
pub struct Route (pub Path, pub RouteHandler);

#[derive(Clone)]
pub struct Response {
    body: String,
    status: u32
}

impl Response {
    pub fn new(body: String, status: u32) -> Response {
        Response {
            body,
            status
        }
    }
}

pub struct Request {
    pub body: Option<String>,
    pub method: String,
    pub path: String,
    matched_path: Option<Path>
}

impl Request {
    fn new(request_string: String) -> Request {
        let body = match request_string.split("\r\n\r\n").nth(1) {
            Some(x) => Some(String::from(x)),
            None => None
        };
        Request {
            method: String::from(request_string.split_whitespace().next().unwrap()),
            body,
            path: String::from(request_string.split_whitespace().nth(1).unwrap()),
            matched_path: None
        }
    }

    pub fn get_param(&self, name: &str) -> Option<&str> {
        if let None = self.matched_path {
            None
        } else {
            let mut params = None;
            if let Some(Path::Rex(ref matched_path)) = self.matched_path {
                for cap in matched_path.captures_iter(self.path.as_ref()) {
                    params = match cap.name(name) {
                        Some(x) => Some(x.as_str()),
                        None => None
                    };
                }
                params
            } else {
                None
            }
        }
    }
}

struct Routes {
    pub get: Vec<Route>,
    pub post: Vec<Route>
}

pub struct Server {
    routes: Arc<Mutex<Routes>>,
    request_cache: HashMap<String, Response>
}

impl Server {
    fn handle_client(&mut self, mut stream: TcpStream) -> () {
        let mut buf: [u8; 512] = [0; 512];
        stream.read(&mut buf).unwrap();
        let path_str = String::from_utf8_lossy(&buf);
        let request_path: Vec<&str> = path_str.split_whitespace().collect();
        let path = request_path[1];
        let mut req = Request::new(path_str.to_string());

        let instant = Instant::now();

        let mut res: Option<Response> = None;
        if self.request_cache.contains_key(path) {
            res = Some(self.request_cache.get(path).unwrap().clone());
        } else {
            let routes = &self.routes.clone();
            let routes = routes.lock().unwrap();
            let routes = match req.method.as_ref() {
                "GET" => &routes.get,
                "POST" => &routes.post,
                _ => &routes.get
            };

            for route in routes.iter() {
                let is_match = match route.0 {
                    Path::Str(ref x) => path == x,
                    Path::Rex(ref x) => x.is_match(path)
                };
                if is_match {
                    req.matched_path = Some(route.0.clone());
                    res = Some((route.1)(Some(&req)));
                    break
                }
            }

            if let Some(ref res) = res {
                self.request_cache.insert(String::from(path), res.clone());
            }
        }

        println!("{} {} - {}ns", &req.method, &req.path, instant.elapsed().subsec_nanos());

        if let Some(r) = res {
            stream.write(format!("HTTP/1.1 {} OK\r\n\r\n{}", r.status, r.body).as_bytes()).unwrap();
        } else {
            stream.write(format!("HTTP/1.1 404 Not Found\r\n\r\nNot Found").as_bytes()).unwrap();
        }
    }

    pub fn new() -> Server {
        Server {
            routes: Arc::new(Mutex::new(Routes {
                get: Vec::new(),
                post: Vec::new()
            })),
            request_cache: HashMap::new()
        }
    }

    pub fn listen(self, port: u32) {
        let pool = threadpool::ThreadPool::new(4);
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
            .unwrap();

        let s = Arc::new(Mutex::new(self));

        for stream in listener.incoming() {
            let stream = stream.unwrap();

            let s = s.clone();

            pool.execute(move || {
                let mut s = s.lock().unwrap();
                s.handle_client(stream);
            })
        }
    }

    pub fn get(&self, path: Path, handler: RouteHandler) {
        let routes = &self.routes.clone();
        routes.lock().unwrap().get.push(Route(path, handler));
    }

    pub fn post(&self, path: Path, handler: RouteHandler) {
        let routes = &self.routes.clone();
        routes.lock().unwrap().post.push(Route(path, handler));
    }
}
