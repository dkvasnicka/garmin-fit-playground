use average::WeightedMean;
use fitparser::{self, Value};
use itertools::Itertools;
use std::{env, fs::File};
use weighted_median::{Data, weighted_median};

struct InGear {
    pub gear: u8,
    pub duration: i64,
    pub pedalling: u8, // 0/1
}

#[derive(Clone, Debug)]
struct FITField {
    pub name: String,
    pub value: Value,
    pub median_cadence: Option<u8>,
    pub timestamp: i64,
}

impl Data for InGear {
    fn get_value(&self) -> f64 {
        self.gear as f64
    }

    fn get_weight(&self) -> f64 {
        (self.duration * self.pedalling as i64) as f64
    }
}

fn main() {
    println!(
        "Parsing FIT files using Profile version: {:?}",
        fitparser::profile::VERSION
    );
    let args: Vec<String> = env::args().collect();
    let mut fp = File::open(&args[1]).unwrap();

    let chunked_evts = fitparser::from_reader(&mut fp)
        .unwrap()
        .into_iter()
        .filter_map(|data| {
            data.fields()
                .iter()
                .find(|f| f.name() == "gear_change_data" || f.name() == "cadence")
                .map(|f| {
                    let ts_field = data
                        .fields()
                        .iter()
                        .find(|f| f.name() == "timestamp")
                        .expect(&format!("Event without timestamp: {:?}", data.fields()));

                    FITField {
                        name: f.name().to_string(),
                        value: f.value().clone(),
                        timestamp: match ts_field.value() {
                            Value::Timestamp(d) => d.timestamp(),
                            _ => panic!("Never gonna happen :-]"),
                        },
                        median_cadence: None,
                    }
                })
        })
        .chunk_by(|field| field.name.clone());
    let relevant_events = chunked_evts.into_iter().skip_while(|f| f.0 == "cadence");

    let mut rear_gears = relevant_events
        .tuples::<(_, _)>()
        .map(|(g, c)| {
            let mut cadences =
                c.1.map(|f| match f.value {
                    Value::UInt8(v) => v.to_be(),
                    _ => panic!("invalid cadence data"),
                })
                .collect_vec();
            cadences.sort();
            let median_key = (cadences.len() - 1) / 2;
            let median_c = cadences.get(median_key).expect("invalid key");
            let final_gearshift = g.1.last().unwrap();

            FITField {
                median_cadence: Some(*median_c),
                ..final_gearshift
            }
        })
        .tuple_windows()
        .map(|(prev, next)| {
            let gear_nums = match prev.value {
                Value::UInt32(v) => v.to_be_bytes(),
                _ => panic!("unsupported gear data"),
            };
            let gear = gear_nums[3];
            InGear {
                gear,
                duration: next.timestamp - prev.timestamp,
                pedalling: (prev.median_cadence.unwrap() > 10) as u8,
            }
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

    println!("  rear gear weighted mean: {:?}", wm.mean());
}
