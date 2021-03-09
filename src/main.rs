#![feature(bool_to_option)]

use actix_web::{http::StatusCode, web, App, HttpResponse, HttpServer};
use itertools::FoldWhile::{Continue, Done};
use itertools::Itertools;
use num::Bounded;
use rand::distributions::Alphanumeric;
use rand::Rng;
use regex;
use serde::Deserialize;
use std::cmp::Ordering;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::io::Write;
use std::path::Path;

static PASTE_FOLDER: &'static str = "pastes/";
static FILENAME_LENGTH: usize = 4;

#[derive(Deserialize)]
struct HtmlData {
    code: String,
}

fn create_filename() -> String {
    let mut rng = rand::thread_rng();

    let filename = loop {
        let name: String = std::iter::repeat(())
            .map(|()| rng.sample(Alphanumeric))
            .map(char::from)
            .take(FILENAME_LENGTH)
            .collect();
        if !Path::new(&format!("{}{}", PASTE_FOLDER, name)).exists() {
            break name;
        }
    };

    filename
}

async fn file(filename: web::Path<String>) -> HttpResponse {
    match fs::read_to_string(format!("{}{}", PASTE_FOLDER, filename)) {
        Ok(content) => HttpResponse::Ok().body(content),
        Err(_) => HttpResponse::NotFound().body("Paste not found"),
    }
}

async fn receive_html(html: web::Json<HtmlData>) -> HttpResponse {
    let filename = create_filename();
    let complete_path = format!("{}{}", PASTE_FOLDER, filename);
    let mut file = fs::File::create(complete_path.clone()).expect("Couldn't create file");

    let re = regex::Regex::new(std::concat!(
        r"(?P<span_open><span id.*? class.*?>)([ ]*)",
        r"(?P<line_nb>[\d]+)([ ]*)",
        r"(?P<span_close></span>)",
        r"(?P<spaces>[ ]*)",
        r"(?P<not_newline>.?)"
    ))
    .unwrap();

    let spaces_to_remove = re
        .captures_iter(html.code.as_str())
        .filter_map(|m| match (m["not_newline"].len(), m["spaces"].len()) {
            (0, _) => None,    // Invalid
            (_, l) => Some(l), // Valid
        })
        .fold_while(usize::max_value(), |min, new| match new {
            0 => Done(0), // Lower bound, stop
            _ => Continue(new.min(min)),
        })
        .into_inner();

    let code = re
        .replace_all(html.code.as_str(), |m: &regex::Captures| {
            format!(
                "{}{}{}{}{}",
                &m["span_open"],
                &m["line_nb"],
                &m["span_close"],
                m["spaces"].get(spaces_to_remove..).unwrap_or_default(),
                &m["not_newline"]
            )
        })
        .to_string();

    println!("TO REM {}", spaces_to_remove);

    // (!m["not_newline"].is_empty()) // Filter invalid T
    // .then(|| match m["spaces"].len())) // T -> R (valid values only)

    let re = regex::Regex::new(r"(.LineNr \{)").unwrap();
    let code = re
        .replace(
            code.as_str(),
            r"${1} position: relative; left: -0.5em; text-align: right;
    display: inline-block; width: 2em; padding-right: 0.5em;
    -webkit-touch-callout: none; -webkit-user-select: none; -khtml-user-select: none;
    -moz-user-select: none; -ms-user-select: none; user-select: none;",
        )
        .to_string();

    // let code = regex::Regex::new(r"font-size: 1em").unwrap().replace(code.as_str(), r"font-size: 16px").to_string();

    let re = regex::Regex::new(r"(background-color:) #000000").unwrap();
    let code = re.replace_all(code.as_str(), "${1} #202020").to_string();

    let re = regex::Regex::new(r"white-space: pre-wrap;").unwrap();
    let code = re.replace_all(code.as_str(), "").to_string();

    let pre = r#"${1}
      mix-blend-mode: lighten;
        -webkit-font-smoothing: antialiased;
        -webkit-font-smoothing: subpixel-antialiased;
        text-rendering: optimizeLegibility;
        image-rendering: pixelated;
      backface-visibility: hidden;
      backface-visibility: unset;
      white-space: pre-wrap;"#;

    let re = regex::Regex::new(r"(pre \{)").unwrap();
    let code = re.replace(code.as_str(), pre).to_string();

    let re = regex::Regex::new(r"Consolas").unwrap();
    let code = re.replace(code.as_str(), r"Noto Sans Mono").to_string();
    let code = re.replace(code.as_str(), r"Noto Sans Mono").to_string();

    let re = regex::Regex::new(r"font-size: 1em").unwrap();
    let code = re.replace(code.as_str(), r"font-size: 16px").to_string();

    // println!("{}", code);

    println!("Done");

    write!(file, "{}", code);

    HttpResponse::Found()
        .header(actix_web::http::header::LOCATION, filename)
        .finish()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if !Path::new("pastes/").exists() {
        match fs::create_dir("pastes") {
            Ok(_) => (),
            Err(_) => panic!("No folder /pastes, and couldn't create it"),
        }
    }

    HttpServer::new(|| {
        App::new()
            .route("/", web::post().to(receive_html))
            .route("/{filename}", web::get().to(file))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
