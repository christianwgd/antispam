#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
extern crate bayespam;
extern crate serde;

use bayespam::classifier::Classifier;
use rocket_contrib::json::Json;
use serde::{Deserialize, Serialize};
use std::fs::File;
use rocket::request::FromRequest;
use rocket::{Request, request, Response};
use rocket::Outcome;
use rocket::http::Status;
use rocket::http::route::Error;

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
fn is_valid(key: &str) -> bool {
    key == "valid_api_key"
}

#[derive(Debug)]
enum ApiKeyError {
    BadCount,
    Missing,
    Invalid,
}

impl<'a, 'r> FromRequest<'a, 'r> for ApiKey {
    type Error = ApiKeyError;
    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {

        let keys: Vec<_> = request.headers().get("x-api-key").collect();
        match keys.len() {
            0 => Outcome::Failure((Status::BadRequest, ApiKeyError::Missing)),
            1 if is_valid(keys[0]) => Outcome::Success(ApiKey(keys[0].to_string())),
            1 => Outcome::Failure((Status::Forbidden, ApiKeyError::Invalid)),
            _ => Outcome::Failure((Status::BadRequest, ApiKeyError::BadCount)),
        }
    }
}


#[post("/check", format = "json", data = "<msg>")]
fn check(key: ApiKey, msg: Json<CheckMessage>) -> Json<Rating> {
    let mut classifier_file = File::open("model.json").unwrap();
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
fn train(key: ApiKey, msg: Json<Message>) {
    let mut classifier_file = File::open("model.json").unwrap();
    let mut classifier = Classifier::new_from_pre_trained(&mut classifier_file).unwrap();

    if msg.is_spam {
        classifier.train_spam(&msg.text);
    } else {
        classifier.train_ham(&msg.text);
    }

    let mut file = File::create("model.json").unwrap();
    classifier.save(&mut file, false).unwrap();
}

fn main() {
    rocket::ignite()
        .mount("/", routes![check, train])
        .launch();
}
