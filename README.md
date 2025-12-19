garmin-fit-playground
=====================

Rust playground for doing analysis on data from my Garmin Edge that I miss in existing
cycling/athlete support software.

## Currently implemented features

* **Rear gear weighted median / mean value** -- for finding out what rear gear is the most used one throughout your ride so you can
optimize your 1x chainring size if you have more of them to swap out. Uses the time in seconds spent in each gear as the weight value
and ignores gears that you were in but were not pedalling (e.g. climbing a hill and then staying in a very light gear when descending 
on the other side of it -- not adjusting for this would skew the result towards lighter gears even though you were not actually utilizing them).

## Howto

Just run `cargo run -- /path/to/activity.fit` and wait for an output similar to:

```
Parsing FIT files using Profile version: "21.171.00"                               
rear gear weighted median: 8.0                                                     
  rear gear weighted mean: 7.0     
```
