#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
extern crate bayespam;
extern crate clap;
extern crate serde;

use bayespam::classifier::Classifier;
use clap::{crate_authors, App, Arg};
use rocket::http::Status;
use rocket::request::FromRequest;
use rocket::Outcome;
use rocket::{request, Request, State};
use rocket_contrib::json::Json;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Deserialize)]
struct CheckMessage {
    text: String,
}

#[derive(Deserialize)]
struct Message {
    text: String,
    is_spam: bool,
}

#[derive(Serialize)]
struct Rating {
    spam: bool,
    score: f32,
}

struct ApiKey(String);

/// Returns true if `key` is a valid API key string.
fn is_valid(key: &str, agent: &str, clients: &Value) -> bool {
    clients[agent] == key
}

#[derive(Debug)]
enum ApiKeyError {
    BadCount,
    Invalid,
}

impl<'a, 'r> FromRequest<'a, 'r> for ApiKey {
    type Error = ApiKeyError;
    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        let clients = request.guard::<State<Value>>().unwrap().inner();
        let keys: Vec<_> = request.headers().get("x-api-key").collect();
        let agents: Vec<_> = request.headers().get("User-Agent").collect();
        if agents.len() == 1 || keys.len() == 1 {
            if is_valid(keys[0], agents[0], clients) {
                Outcome::Success(ApiKey(keys[0].to_string()))
            } else {
                Outcome::Failure((Status::Forbidden, ApiKeyError::Invalid))
            }
        } else {
            Outcome::Failure((Status::BadRequest, ApiKeyError::BadCount))
        }
    }
}

#[post("/check", format = "json", data = "<msg>")]
fn check(_key: ApiKey, msg: Json<CheckMessage>, model_file: State<String>) -> Json<Rating> {
    let mut classifier_file = File::open(&model_file.inner()).unwrap();
    let classifier = Classifier::new_from_pre_trained(&mut classifier_file).unwrap();

    let is_spam = classifier.identify(&msg.text);
    let score = classifier.score(&msg.text);

    let rat = Rating {
        spam: is_spam,
        score,
    };

    Json(rat)
}

#[post("/train", format = "json", data = "<msg>")]
fn train(_key: ApiKey, msg: Json<Message>, model_file: State<String>) {
    let mut classifier_file = File::open(&model_file.inner()).unwrap();
    let mut classifier = Classifier::new_from_pre_trained(&mut classifier_file).unwrap();

    if msg.is_spam {
        classifier.train_spam(&msg.text);
    } else {
        classifier.train_ham(&msg.text);
    }

    let mut file = File::create("model.json").unwrap();
    classifier.save(&mut file, false).unwrap();
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("antispam")
        .version("1.0")
        .author(crate_authors!())
        .about("Antispam is a web service runtime for the bayespam spam checker crate.")
        .arg(Arg::new("config")
            .required(true)
            .short('c')
            .long("config")
            .value_name("CONFIG_FILE")
            .about("Sets a custom config file in json format containing agents and their api-keys as key-value pairs.")
            .takes_value(true))
        .arg(Arg::new("model")
            .required(true)
            .short('m')
            .long("model")
            .value_name("MODEL_FILE")
            .about("Model file.")
            .takes_value(true))
        .get_matches();

    let mut conf_file = File::open(matches.value_of("config").unwrap())?;
    let mut config = String::new();
    conf_file.read_to_string(&mut config)?;
    let clients: Value = serde_json::from_str(&config)?;

    let model = matches.value_of("model").unwrap().to_owned();
    if !Path::new(&model).exists() {
        let mut classifier_file = File::create(&model).unwrap();
        let classifier = Classifier::new();
        classifier.save(&mut classifier_file, false)?;
    }

    rocket::ignite()
        .mount("/", routes![check, train])
        .manage(clients)
        .manage(model)
        .launch();

    Ok(())
}
