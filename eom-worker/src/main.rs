extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

#[macro_use]
extern crate structopt;

extern crate bson;
extern crate mongodb;
extern crate redis;

extern crate eom;
extern crate ndarray;

use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{spawn, JoinHandle};
use structopt::StructOpt;
use ndarray::*;
use eom::*;
use eom::traits::*;
use mongodb::ThreadedClient;
use mongodb::db::ThreadedDatabase;
use redis::Commands;

#[derive(Serialize)]
struct Doc {
    time: f64,
    data: Vec<f64>,
}

impl Doc {
    fn to_document(&self) -> bson::Document {
        use bson::Bson::*;
        match bson::to_bson(self).unwrap() {
            Document(d) => d,
            _ => unreachable!("Invalid"),
        }
    }
}

#[derive(Deserialize, Debug)]
struct RunSetting {
    pub dt: f64,
    pub duration: usize,
    pub skip: usize,
}

fn run(setting: RunSetting, sender: Sender<Doc>) {
    let eom = ode::Lorenz63::default();
    let mut teo = explicit::RK4::new(eom, setting.dt);
    let ts = adaptor::time_series(arr1(&[1.0, 0.0, 0.0]), &mut teo);
    for (t, v) in ts.take(setting.duration).enumerate() {
        if t % setting.skip == 0 {
            let doc = Doc {
                time: t as f64 * setting.dt,
                data: v.to_vec(),
            };
            sender.send(doc).expect("Failed to send doc");
        }
    }
}

#[derive(Deserialize, Debug)]
struct OutputSetting {
    host: String,
    port: u16,
    db: String,
    collection: String,
}

fn output(setting: OutputSetting, recv: Receiver<Doc>) -> JoinHandle<()> {
    let cli = mongodb::Client::connect(&setting.host, setting.port)
        .expect("Unable to connect to MongoDB");
    let coll = cli.db(&setting.db).collection(&setting.collection);
    spawn(move || loop {
        match recv.recv() {
            Ok(doc) => {
                coll.insert_one(doc.to_document(), None)
                    .expect("Failed to insert document");
            }
            Err(_) => break,
        }
    })
}

#[derive(StructOpt)]
struct RedisSetting {
    #[structopt(long = "host", default_value = "localhost")]
    host: String,
    #[structopt(long = "fifo", default_value = "tasks")]
    fifo: String,
}

fn get_task(setting: &RedisSetting) -> (RunSetting, OutputSetting) {
    let cli = redis::Client::open(format!("redis://{}", setting.host).as_str())
        .expect("Failed to connect to Redis");
    let con = cli.get_connection()
        .expect("Failed to get Redis connection");
    loop {
        eprint!("waiting... ");
        let (_tasks, task): (String, String) =
            con.blpop(&setting.fifo, 0).expect("BLPOP operation fails");
        eprintln!("Get Task!");
        let rs = serde_json::from_str(&task);
        let os = serde_json::from_str(&task);
        match (rs, os) {
            (Ok(rs), Ok(os)) => return (rs, os),
            _ => eprintln!("Failed to parse JSON: {}", task),
        };
    }
}

fn main() {
    let setting = RedisSetting::from_args();
    loop {
        let (rs, os) = get_task(&setting);
        let (s, r) = channel::<Doc>();
        let output_thread = output(os, r);
        run(rs, s);
        output_thread.join().expect("Failed to join output thread");
    }
}
