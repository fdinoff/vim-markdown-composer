//! A simple client that listens for msgpack-serialized strings on a port and renders them as
//! markdown.
//!
//! The markdown is rendered on an arbitrary port on localhost, which is then automatically opened
//! in a browser. As new messages are received on the input port, the markdown is asynchonously
//! rendered in the browser (no refresh is required).

#[macro_use] extern crate log;
extern crate aurelius;
extern crate docopt;
extern crate env_logger;
extern crate rmp as msgpack;
extern crate rustc_serialize;

use std::io::BufReader;
use std::net::TcpStream;

use aurelius::Server;
use aurelius::browser;
use docopt::Docopt;
use msgpack::Decoder;
use msgpack::decode::ReadError::UnexpectedEOF;
use msgpack::decode::serialize::Error::InvalidMarkerRead;
use rustc_serialize::Decodable;

static USAGE: &'static str = "
Usage: markdown_composer [options] <nvim-port> [<initial-markdown>]
       markdown_composer --help

Options:
    -h, --help                  Show this message.
    --no-browser                Don't open the web browser automatically.
    --browser=<executable>      Specify a browser that the program should open. If not supplied,
                                the program will determine the user's default browser.
    --highlight-theme=<theme>   The theme to use for syntax highlighting. All highlight.js themes
                                are supported. If no theme is supplied, the 'github' theme is used.
";

#[derive(RustcDecodable, Debug)]
struct Args {
    arg_nvim_port: u16,
    arg_initial_markdown: Option<String>,
    flag_no_browser: bool,
    flag_browser: Option<String>,
    flag_highlight_theme: Option<String>,
}

fn main() {
    env_logger::init().unwrap();

    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());

    let mut server_builder = Server::new();
    if let Some(ref markdown) = args.arg_initial_markdown {
        server_builder.initial_markdown(markdown);
    }

    if let Some(ref theme) = args.flag_highlight_theme {
        server_builder.highlight_theme(theme);
    }

    let server = server_builder.start();

    if !args.flag_no_browser {
        let url = format!("http://localhost:{}", server.http_port());
        if let Some(ref browser) = args.flag_browser {
            browser::open_specific(&url, browser, None).unwrap();
        } else {
            browser::open(&url).unwrap();
        }
    }

    let nvim_port = args.arg_nvim_port;
    let stream = TcpStream::connect(("localhost", nvim_port))
                            .ok()
                            .expect(&format!("no listener on port {}", nvim_port));

    let mut decoder = Decoder::new(BufReader::new(stream));
    loop {
        let msg = <String as Decodable>::decode(&mut decoder);
        match msg {
            Ok(msg) => server.send_markdown(&msg),
            Err(InvalidMarkerRead(UnexpectedEOF)) => {
                // In this case, the remote client probably just hung up.
                break
            },
            Err(err) => panic!(err)
        }
    }
}
