#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
extern crate bayespam;

#[cfg(test)]
mod tests;

use bayespam::classifier;
use bayespam::classifier::Classifier;
use std::fs::File;

#[get("/hello/<name>/<age>")]
fn hello(name: String, age: u8) -> String {
    format!("Hello, {} year old named {}!", age, name)
}

#[get("/hello/<name>")]
fn hi(name: String) -> String {
    format!("Hi {}", name)
}

#[get("/train/<message>/<is_spam>")]
fn train(message: String, is_spam: bool) -> Result<(), std::io::Error> {
    println!("Receives Message: {}", message);
    println!("Classified as Spam?: {}", is_spam.to_string());

    let mut classifier_file = File::open("model.json")?;
    let mut classifier = Classifier::new_from_pre_trained(&mut classifier_file)?;

    if is_spam {
        classifier.train_spam(&message);
    } else {
        classifier.train_ham(&message);
    }


    classifier.save(&mut classifier_file, true)?;

    Ok(())
}

#[get("/check/<message>")]
fn check(message: String) -> String {
    println!("Receives Message: {}", message);
    classifier::identify(&message).unwrap().to_string()
}

fn main() {
    rocket::ignite()
        .mount("/", routes![hello, hi, check, train])
        .launch();
}
