use std::fs;
use std::sync::Arc;
use tiny_http::{ Server, Response, Request, Header };

fn handle_request(req: Request, html: Arc<String>, css: Arc<String>, js: Arc<String>) {
    let url = req.url();
    let (content, content_type) = match url {
        "/" | "/index.html" => (html.as_str(), "text/html"),
        "/api/positions" => {
            // Read fresh data from positions.json on each API call
            let positions_json = fs::read_to_string("positions.json").unwrap_or("[]".to_string());
            let response = Response::from_string(positions_json)
                .with_header(Header::from_bytes(&b"Content-Type"[..], "application/json").unwrap())
                .with_header(Header::from_bytes(&b"Access-Control-Allow-Origin"[..], "*").unwrap());
            let _ = req.respond(response);
            return;
        }
        "/webserver.css" => (css.as_str(), "text/css"),
        "/webserver.js" => (js.as_str(), "application/javascript"),
        _ => ("Not found", "text/plain"),
    };
    let response = Response::from_string(content).with_header(
        Header::from_bytes(&b"Content-Type"[..], content_type).unwrap()
    );
    let _ = req.respond(response);
}

pub fn start_webserver() {
    let html = Arc::new(
        fs::read_to_string("webserver.html").unwrap_or("<h1>No HTML</h1>".to_string())
    );
    let css = Arc::new(fs::read_to_string("webserver.css").unwrap_or("".to_string()));
    let js = Arc::new(fs::read_to_string("webserver.js").unwrap_or("".to_string()));
    let server = Server::http("127.0.0.1:8080").unwrap();
    println!("Webserver running at http://127.0.0.1:8080");

    for req in server.incoming_requests() {
        let html = html.clone();
        let css = css.clone();
        let js = js.clone();
        std::thread::spawn(move || {
            handle_request(req, html, css, js);
        });
    }
}
