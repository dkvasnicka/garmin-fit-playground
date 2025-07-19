use average::WeightedMean;
use fitparser::{self, Value};
use itertools::Itertools;
use std::{env, fs::File};
use weighted_median::{Data, weighted_median};

struct InGear {
    pub gear: u8,
    pub duration: i64,
}

impl Data for InGear {
    fn get_value(&self) -> f64 {
        self.gear as f64
    }

    fn get_weight(&self) -> f64 {
        self.duration as f64
    }
}

fn main() {
    println!(
        "Parsing FIT files using Profile version: {:?}",
        fitparser::profile::VERSION
    );
    let args: Vec<String> = env::args().collect();
    let mut fp = File::open(&args[1]).unwrap();

    let mut rear_gears = fitparser::from_reader(&mut fp)
        .unwrap()
        .into_iter()
        .filter_map(|data| {
            let gear_change_data = data
                .fields()
                .iter()
                .find(|f| f.name() == "gear_change_data");
            if let Some(gcd_val) = gear_change_data {
                let ts_field = data
                    .fields()
                    .iter()
                    .find(|f| f.name() == "timestamp")
                    .unwrap();

                // println!("{:?}", data);
                let gear_nums = match gcd_val.value() {
                    Value::UInt32(v) => v.to_be_bytes(),
                    _ => panic!("unsupported gear data"),
                };
                let timestamp = match ts_field.value() {
                    Value::Timestamp(d) => d.timestamp(),
                    _ => panic!("got a gear change event without timestamp"),
                };

                Some((timestamp, gear_nums[3]))
            } else {
                None
            }
        })
        .dedup_by(|l, r| l.1 == r.1)
        .tuple_windows()
        .map(|(prev, next)| InGear {
            gear: prev.1,
            duration: next.0 - prev.0,
        })
        .collect_vec();

    println!(
        "rear gear weighted median: {:?}",
        weighted_median(rear_gears.as_mut_slice()).unwrap()
    );

    let wm: WeightedMean = rear_gears
        .iter()
        .map(|ig| (ig.get_value(), ig.get_weight()))
        .collect();

    println!("  rear gear weighted mean: {:?}", wm.mean().round());
}
