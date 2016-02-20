#![feature(io)]

// https://registry.npmjs.org/grunt/-/grunt-0.1.0.tgz
// https://registry.npmjs.org/-/all

extern crate rustc_serialize;
extern crate hyper;
extern crate scoped_pool;
extern crate tar;
extern crate flate2;

use flate2::read::GzDecoder;
use scoped_pool::Pool;
use hyper::Client;
use hyper::header::Connection;
use hyper::status::StatusCode;
use rustc_serialize::json::{Parser, JsonEvent, StackElement};
use std::io::prelude::*;
use std::fs::File;
use std::io;
use tar::Archive;

pub fn fetch(url: &str) -> Box<Read> {
    let client = Client::new();
    let res = client.get(url)
        .header(Connection::close())
        .send().unwrap();
    
    assert_eq!(res.status, StatusCode::Ok);

    Box::new(res)
}

// pub fn fetch_json(url: &str) -> serde_json::Result<Value> {
//     serde_json::from_str(&fetch_string(url))
// }

fn dump_gyp<T: Read>(name: &str, input: &mut T) {
    println!("dumping: {}", name);
    
    let d = GzDecoder::new(input).unwrap();
    let mut a = Archive::new(d);

    for file in a.entries().unwrap() {
        // Make sure there wasn't an I/O error
        let mut file = file.unwrap();

        // Inspect metadata about the file
        let path = if let Ok(value) = file.header().path() {
            value.to_string_lossy().into_owned()
        } else {
            "".to_owned()
        };
        let size = file.header().size().unwrap() as usize;
        
        if path.ends_with(".gyp") {
            let mut out = File::create(format!("out/{}@{}", name, path.replace("/", "~"))).unwrap();
            io::copy(&mut file, &mut out).expect("File writing failed.");
            
            // let mut s = String::with_capacity(size);
            // file.read_to_string(&mut s).unwrap();
            println!("{}@{} size {}", name, path.replace("/", "~"), size);
        }
        // println!("{}", file.header().size().unwrap());

        // files implement the Read trait
        // let mut s = String::new();
        // file.read_to_string(&mut s).unwrap();
        // println!("{}", s);
    }
}

fn main() {
    let read = fetch("https://registry.npmjs.org/-/all");

    let pool = Pool::new(20);

    pool.scoped(|scope| {
        let mut p = Parser::new(read.chars().filter(|x| x.is_ok()).map(|x| x.unwrap()));
        // let mut p = Parser::new("{\"a\": \"b\"}".chars());
        // let mut nested = 0;
        // let mut root_key = false;
        loop {
            let event = if let Some(e) = p.next() {
                e
            } else {
                break;
            };
            
            // println!("n {:?} {:?}", p.stack().len(), p.stack().top());
            
            if p.stack().len() == 3 && 
                p.stack().get(1) == StackElement::Key("dist-tags") &&
                p.stack().get(2) == StackElement::Key("latest") {
                let pkg = if let StackElement::Key(v) = p.stack().get(0) {
                    v.to_owned()
                } else {
                    "".to_owned()
                };
                let version = if let JsonEvent::StringValue(v) = event {
                    v
                } else {
                    "0.0.0".to_owned()
                };
                
                // if pkg == "3drotate" {
                scope.execute(move || {
                    dump_gyp(&pkg, &mut fetch(&format!("https://registry.npmjs.org/{}/-/{}-{}.tgz", pkg, pkg, version)));
                });
                // }
            }
            // match event {
            //     JsonEvent::ObjectStart => {
            //         nested += 1;
            //         root_key = nested == 1;
            //     }
            //     JsonEvent::ArrayStart => {
            //         nested += 1;
            //         root_key = false;
            //     }
            //     JsonEvent::ArrayEnd => {
            //         nested -= 1;
            //         root_key = nested == 1;
            //     }
            //     JsonEvent::ObjectEnd => {
            //         nested -= 1;
            //         root_key = nested == 1;
            //     }
            //     JsonEvent::StringValue(value) => {
            //         if root_key {
            //             println!("found package. {:?}", value);
            //         }
            //         if nested == 1 {
            //             root_key = !root_key;
            //         }
            //     }
            //     // Other values
            //     _ => {
            //         if nested == 1 {
            //             root_key = !root_key;
            //         }    
            //     }
            // }
            // ::std::thread::sleep(::std::time::Duration::from_millis(100));
            // if let JsonEvent::StringValue(value) = event {
            //     if value == "bluebird" {
            //         println!("oh!");
            //         scope.execute(move || {
            //             println!("dumping an archive...");
            //             let pkg = "bufferutil";
            //             dump_gyp(pkg, &mut fetch(&format!("https://registry.npmjs.org/{}/-/{}-1.2.1.tgz", pkg, pkg)));
            //         });
            //     }
            // }
        }
    });

    // https://github.com/alexcrichton/tar-rs#reading-an-archive
    // https://github.com/reem/rust-scoped-pool
    // https://gist.github.com/creationix/1821394
}
