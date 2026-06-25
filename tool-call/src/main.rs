use chrono::{DateTime, Local};
use clap::Parser;

#[derive(Parser)]
struct Args {
    command: String,
}

fn date_time() {
    let lt = Local::now();
}

fn get_weather() {}

fn main() {
    let args = Args::parse();
    let is_even = |val: i32| val % 2 == 0;
    println!("is_even? 1: {}", is_even(1));
    println!("is_even? 2: {}", is_even(2));
    println!("command : {}", args.command);
    if args.command == "weather" {
        get_weather();
    }
    if args.command == "datetime" {
        date_time();
    }
}
