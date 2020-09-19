#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
extern crate bayespam;
extern crate serde;

use bayespam::classifier::Classifier;
use rocket_contrib::json::Json;
use serde::{Deserialize, Serialize};
use std::fs::File;

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

#[get("/check/<message>")]
fn check(message: String) -> Json<Rating> {
    let mut classifier_file = File::open("model.json").unwrap();
    let classifier = Classifier::new_from_pre_trained(&mut classifier_file).unwrap();

    let is_spam = classifier.identify(&message);
    let score = classifier.score(&message);

    let rat = Rating {
        spam: is_spam,
        score,
    };

    Json(rat)
}

#[post("/train", format = "json", data = "<msg>")]
fn train(msg: Json<Message>) {
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
    rocket::ignite().mount("/", routes![check, train]).launch();
}
