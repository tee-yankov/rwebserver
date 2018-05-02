extern crate rwebserver;
use std::env::args;
use rwebserver::{Server, Response, Request, Path as RoutePath};
use std::fs::DirEntry;
use std::path::Path;

fn main() {
    let port: u32 = match args().nth(1) {
        Some(x) => x.parse().unwrap(),
        None => 1337
    };

    assert!(port > 1024);

    println!("Staring webserver on port {}", port);

    let server = Server::new();

    server.get(RoutePath::Str(String::from("/t1")), Box::new(|_| {
        Response::new(String::from("Hey there, alligator"), 200)
    }));

    server.get(RoutePath::Str(String::from("/dir")), Box::new(|_| {
        let dir = Path::new("./");
        let dirs: Vec<DirEntry> = dir.read_dir().unwrap().map(|x| x.unwrap()).collect();
        let mut files = String::new();
        files.push_str("<h1>Index of /</h1>");
        for entry in dirs {
            let p = entry.file_name();
            let file = p.to_str().unwrap();
            files.push_str(format!(
                "<p><a href=\"/dir/{}\">{}</a></p>", file, file
                ).as_ref());
        }
        let html = format!("
                           <html>
                            <head>
                                <title>Index of /</title>
                            </head>
                            <body>
                                {}
                            </body>
                           </html>
                           ", files);
        Response::new(html, 200)
    }));

    server.post(RoutePath::Str(String::from("/t1")), Box::new(|req: Option<&Request>| {
        let req = req.unwrap();
        Response::new(if let Some(ref x) = req.body {
            println!("{}", x);
            x.to_string()
        } else {
            String::from("")
        }, 200)
    }));

    server.listen(port);
}
