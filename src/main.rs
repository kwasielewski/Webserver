// Krzysztof Wasielewski 322091
use std::{
    env, fs,
    path,
    fmt,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream}, time::Duration,
};

struct Request{
    addr: String,
    host: String,
    port: String,
    conn: bool
}

impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result{
        f.debug_struct("Request")
            .field("address", &self.addr)
            .field("host", &self.host)
            .field("connection", &self.conn)
            .finish()
    }
}

fn main() {
    let arguments: Vec<String> = env::args().collect();
    let listener = match TcpListener::bind(
        format!("{}:{}", arguments[1], arguments[2]))
        {
            Ok(v) => v,
            Err(_) => {println!("Incorrect arguments"); return;}
        };
    for stream in listener.incoming(){
        let stream = stream.unwrap();
        
        handle_connection(stream);
    }
}

fn parse_request(req: Vec<String>) -> Option<Request>{
    
    let important: Vec<&String> = req
        .iter()
        .filter(|s| s.starts_with("GET") 
            || s.starts_with("Host")
            || s.starts_with("Connection"))
        .collect();

    println!("{:?} {}", important, important.len());

    if important.len() != 3 {
        return None;
    }
    println!("Host {}", important[1]);
    let begin_host =
        match important[1].find("Host: ") {
            None => return None,
            Some(v) => if v == 0 {6} else {return None}
        };
   let end_host = 
        match important[1][begin_host..].find(':'){
            None => return None,
            Some(v) => v,
        }; 
    let end_host = begin_host+end_host; 
    println!("Host name: {}", important[1][begin_host..end_host].to_string());
    let begin =
        match important[0].find('/') {
            None => return None,
            Some(v) => v 
        };
    let end = 
        match important[0][begin..].find(' '){
            None => return None,
            Some(v) => v,
        };
    let end = end+begin;
    println!("Results {} {}", begin, end);
    
    let connection_status = 
        match important[2].as_str(){
            "Connection: keep-alive" => true,
            "Connection: close" => false,
            _ => return None,
        };
    return Some(Request { addr: important[0][(begin+1)..end].to_string(),
                          host: important[1][begin_host..end_host].to_string(),
                          port: important[1][(end_host+1)..].to_string(),
                          conn: connection_status});
}

fn build_response(req: &Request) -> (i32, String, path::PathBuf){
    println!("{:?}", env::current_dir());
    let path = match path::Path::new("./resources/webpages/")
        .join(req.host.clone())
        .join(req.addr.clone())
        .canonicalize() {
            Ok(v) => v.to_path_buf(),
            Err(_) =>  return (404, String::from("Not Found"),
                path::Path::new("./resources/responses/404.html").to_path_buf())
        };

    println!("Path that should be relative {:?}", path);
    let origin = path::Path::new("./resources/webpages/").canonicalize().unwrap();
    if !path.starts_with(origin) {
        //jailbreak
        return (403, String::from("Forbidden"),
            path::Path::new("./resources/responses/forbidden.html").to_path_buf());
    }
    //is a directory -> moved permanently
    if path.is_dir() {
        return (301, String::from("Moved Permanently"),
            path::Path::new("./resources/responses/301.html").to_path_buf()); //to be changed
    }
    if path.is_file() {
        return (200, String::from("OK"), path);
    }
    return (404, String::from("Not Found"), path::Path::new("./resources/responses/404.html").to_path_buf());
}

fn content_type(filename: &String) -> String {
    let s = filename.as_str();
    let ret_val = match s {
        s if s.ends_with("txt") => String::from("Content-Type: text/plain; charset=UTF-8\r\n"),
        s if s.ends_with("html") => String::from("Content-Type: text/html; charset=UTF-8\r\n"),
        s if s.ends_with("css") => String::from("Content-Type: text/css; charset=UTF-8\r\n"),
        s if s.ends_with("jpg") || s.ends_with("jpeg") => 
            String::from("Content-Type: image/jpeg\r\n"),
        s if s.ends_with("png") => String::from("Content-Type: image/png\r\n"),
        s if s.ends_with("pdf") => String::from("Content-Type: application/pdf\r\n"),
        _ => String::from("Content-Type: application/octet-stream\r\n")
    };
    return ret_val;
}

fn not_implemented() -> String{
    let status_line = "HTTP/1.1 501 Not Implemented";
    let contents = "";
    let length = contents.len();

    let response =
        format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

    return response;
}

fn handle_connection(mut stream: TcpStream){
    stream.set_read_timeout(Some(Duration::new(1, 0))).expect("Timeout");
    let mut stream_clone = stream.try_clone().unwrap();
    let mut buf_reader = BufReader::new(&mut stream);
    loop{
        let read_lines = buf_reader.by_ref().lines();
        
        let http_request: Vec<String> = read_lines
                    .take_while(|result| match result {Err(_) => false, Ok(_)=> true})
                    .map(|result| result.unwrap())
                    .take_while(|line| !line.is_empty()) //tutaj rozdziela się requesty
                    .collect();
        if http_request.len() == 0 {
            return;
        }  
        
        let parsed = parse_request(http_request);

        match parsed {
            None => std::io::Write::by_ref(&mut stream_clone)
                .write_all(not_implemented().as_bytes()).unwrap(),
            Some(v) => {
                let (code, msg, path) = build_response(&v);
                let status_line = format!("HTTP/1.1 {code} {msg}");
                let content_line = if code == 200 {content_type(&v.addr)} else {String::from("Content-Type: text/html; charset=UTF-8\r\n")}; 
                println!("{}", code);
                println!("{:?}", path);
                if code == 301 {
                    println!("{0} {1} {2}", v.host,v.port, v.addr);
                    let response =
                        format!("{status_line}\r\nLocation: http://{0}:{1}/{2}index.html\r\n\r\n", v.host, v.port, v.addr);
                    println!("{}", response); 
                    stream_clone.write_all(response.as_bytes()).unwrap();

                } else {
                    let contents = fs::read(path).unwrap();//fs::read_to_string(path).unwrap();
                    let length = contents.len();

                    let response =
                        format!("{status_line}\r\n{content_line}Content-Length: {length}\r\n\r\n");
                    
                    println!("{}", response); 
                    stream_clone.write_all(response.as_bytes()).unwrap();
                    stream_clone.write_all(contents.as_slice()).unwrap();
                }
                if v.conn == false {
                    return;
                }
            }
        }
    }
    
}
