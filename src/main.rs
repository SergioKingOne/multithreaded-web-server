use std::{
    fs::File,
    io::{copy, prelude::*, BufReader, Error},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

use hello::ThreadPool;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:7878")?;
    let pool = ThreadPool::new(4);

    for stream in listener.incoming() {
        let stream = stream?;
        pool.execute(move || {
            if let Err(e) = handle_connection(stream) {
                eprintln!("Error handling connection: {}", e);
            }
        });
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> Result<(), Error> {
    let mut buf_reader = BufReader::new(&stream);
    let mut request_line = String::new();
    buf_reader.read_line(&mut request_line)?;

    let (status_line, filename) = match request_line.trim() {
        "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "hello.html"),
        "GET /sleep HTTP/1.1" => {
            thread::sleep(Duration::from_secs(5));
            ("HTTP/1.1 200 OK", "hello.html")
        }
        _ => ("HTTP/1.1 404 NOT FOUND", "404.html"),
    };

    let mut file = File::open(filename)?;
    let length = file.metadata()?.len();

    write_response(&mut stream, status_line, length, &mut file)?;

    Ok(())
}

fn write_response(
    stream: &mut TcpStream,
    status_line: &str,
    length: u64,
    file: &mut File,
) -> Result<(), Error> {
    let mut response = format!("{}\r\nContent-Length: {}\r\n\r\n", status_line, length);
    stream.write_all(response.as_bytes())?;
    copy(file, stream)?;

    Ok(())
}
