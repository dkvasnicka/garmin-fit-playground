use average::WeightedMean;
use fitparser::{self, Value};
use itertools::Itertools;
use std::{collections::BTreeMap, env, fs::File};
use weighted_median::{Data, weighted_median};

struct InGear {
    pub gear: u8,
    pub duration: i64,
    pub pedalling: u8, // 0/1
    pub start_ts: i64,
    pub end_ts: i64,
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

struct PowerData {
    pub power: u16,
    pub cadence: u8,
}

fn main() {
    println!(
        "Parsing FIT files using Profile version: {:?}",
        fitparser::profile::VERSION
    );
    let args: Vec<String> = env::args().collect();
    let mut fp = File::open(&args[1]).unwrap();

    let chainring_args: Option<(u8, u8)> = if args.len() >= 4 {
        Some((
            args[2]
                .parse::<u8>()
                .expect("current_chainring must be a number"),
            args[3]
                .parse::<u8>()
                .expect("hypothetical_chainring must be a number"),
        ))
    } else {
        None
    };

    let all_records = fitparser::from_reader(&mut fp).unwrap();

    // Extract power data: timestamp -> watts
    let power_data: BTreeMap<i64, PowerData> = all_records
        .iter()
        .filter_map(|r| {
            let ts = r
                .fields()
                .iter()
                .find(|f| f.name() == "timestamp")
                .and_then(|f| match f.value() {
                    Value::Timestamp(d) => Some(d.timestamp()),
                    _ => None,
                })?;
            let cadence =
                r.fields()
                    .iter()
                    .find(|f| f.name() == "cadence")
                    .and_then(|f| match f.value() {
                        Value::UInt8(v) => Some(*v),
                        _ => None,
                    })?;
            let power = r
                .fields()
                .iter()
                .find(|f| f.name() == "power")
                .and_then(|f| match f.value() {
                    Value::UInt16(v) => Some(*v),
                    _ => None,
                })?;
            Some((ts, PowerData { power, cadence }))
        })
        .collect();

    let ride_start_ts = power_data.keys().next().copied().unwrap_or(0);

    let chunked_evts = all_records
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
                start_ts: prev.timestamp,
                end_ts: next.timestamp,
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

    // Chainring comparison analysis
    if let Some((current, hypothetical)) = chainring_args {
        let factor = hypothetical as f64 / current as f64;
        let lightest_sections: Vec<&InGear> = rear_gears
            .iter()
            .filter(|g| g.gear == 1 && g.pedalling == 1)
            .collect();

        println!(
            "\n--- Chainring comparison: {}T -> {}T (factor: {:.3}) ---",
            current, hypothetical, factor
        );

        if lightest_sections.is_empty() {
            println!("Lightest gear was NEVER used while pedalling.");
            println!("The bigger chainring is automatically better -- you have headroom");
            println!("and bigger cogs = better chain efficiency.");
        } else {
            let total_time: i64 = lightest_sections.iter().map(|s| s.duration).sum();
            println!(
                "Lightest gear used in {} section(s) while pedalling, total {}s",
                lightest_sections.len(),
                total_time
            );

            for (i, section) in lightest_sections.iter().enumerate() {
                let elapsed_start = section.start_ts - ride_start_ts;
                let elapsed_end = section.end_ts - ride_start_ts;

                print!(
                    "\n  Section {} ({}:{:02}-{}:{:02}, {}s): ",
                    i + 1,
                    elapsed_start / 60,
                    elapsed_start % 60,
                    elapsed_end / 60,
                    elapsed_end % 60,
                    section.duration
                );

                let section_power: Vec<u16> = power_data
                    .range(section.start_ts..=section.end_ts)
                    .map(|(_, PowerData { power, cadence: _ })| power.to_owned())
                    .collect();
                let section_cadence: Vec<u8> = power_data
                    .range(section.start_ts..=section.end_ts)
                    .map(|(_, PowerData { power: _, cadence })| cadence.to_owned())
                    .collect();

                if section_power.is_empty() {
                    println!("no power data for this section");
                } else {
                    let avg_power: f64 = section_power.iter().map(|&w| w as f64).sum::<f64>()
                        / section_power.len() as f64;
                    let avg_cadence: f64 = section_cadence.iter().map(|&w| w as f64).sum::<f64>()
                        / section_cadence.len() as f64;
                    let hypothetical_power = avg_power * factor;
                    println!(
                        "avg {:.0}W @ {:.0} RPM -> would need {:.0}W ({:.0}W Δ)",
                        avg_power,
                        avg_cadence,
                        hypothetical_power,
                        hypothetical_power - avg_power
                    );
                }
            }
        }
    }
}
