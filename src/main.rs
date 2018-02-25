extern crate serde;
#[macro_use]
extern crate serde_derive;

extern crate bson;
extern crate mongodb;

extern crate eom;
extern crate ndarray;

use ndarray::*;
use eom::*;
use eom::traits::*;
use mongodb::ThreadedClient;
use mongodb::db::ThreadedDatabase;

struct Setting {
    pub dt: f64,
    pub duration: usize,
    pub skip: usize,
}

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

fn exec(setting: Setting, name: &str) {
    let eom = ode::Lorenz63::default();
    let mut teo = explicit::RK4::new(eom, setting.dt);
    let ts = adaptor::time_series(arr1(&[1.0, 0.0, 0.0]), &mut teo);

    let cli = mongodb::Client::connect("localhost", 27017).unwrap();
    let coll = cli.db("eom").collection(name);

    for (t, v) in ts.take(setting.duration).enumerate() {
        if t % setting.skip == 0 {
            let doc = Doc {
                time: t as f64 * setting.dt,
                data: v.to_vec(),
            }.to_document();
            coll.insert_one(doc, None).unwrap();
        }
    }
}

fn main() {
    let setting = Setting {
        dt: 0.01,
        duration: 1000,
        skip: 10,
    };
    exec(setting, "test");
}
