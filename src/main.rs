extern crate serde;
#[macro_use]
extern crate serde_derive;

extern crate bson;
extern crate mongodb;

extern crate eom;
extern crate ndarray;

use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{spawn, JoinHandle};
use ndarray::*;
use eom::*;
use eom::traits::*;
use mongodb::ThreadedClient;
use mongodb::db::ThreadedDatabase;

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

struct OutputSetting {
    host: String,
    port: u16,
    db: String,
    collection: String,
}

fn output(setting: OutputSetting, recv: Receiver<Doc>) -> JoinHandle<()> {
    let cli = mongodb::Client::connect(&setting.host, setting.port).unwrap();
    let coll = cli.db(&setting.db).collection(&setting.collection);
    spawn(move || loop {
        match recv.recv() {
            Ok(doc) => {
                coll.insert_one(doc.to_document(), None).unwrap();
            }
            Err(_) => break,
        }
    })
}

fn main() {
    let setting = RunSetting {
        dt: 0.01,
        duration: 1000,
        skip: 10,
    };
    let output_setting = OutputSetting {
        host: "localhost".to_string(),
        port: 27017,
        db: "eom".to_string(),
        collection: "test".to_string(),
    };
    let (s, r) = channel::<Doc>();
    let output_thread = output(output_setting, r);
    run(setting, s);
    output_thread.join().unwrap();
}
