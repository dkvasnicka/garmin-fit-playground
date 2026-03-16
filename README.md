garmin-fit-playground
=====================

Rust playground for doing analysis on data from my Garmin Edge that I miss in existing
cycling/athlete support software.

## Currently implemented features

* **Rear gear weighted median / mean value** -- for finding out what rear gear is the most used one throughout your ride so you can
optimize your 1x chainring size if you have more of them to swap out. Uses the time in seconds spent in each gear as the weight value
and ignores gears that you were in but were not pedalling (e.g. climbing a hill and then staying in a very light gear when descending 
on the other side of it -- not adjusting for this would skew the result towards lighter gears even though you were not actually utilizing them).

* **Hypothetical bigger chainring analysis** -- if you supply your current chainring size and a hypothetical bigger one as additional command line arguments
the program will check all segments of the ride where you were in your lightest gear and thus having a bigger chainring would require you to output
more power with no lighter gear to escape to. Use this if you see your avg/median rear gears still tend towards the smaller portion of your rear
cassette and you want to know if you could still power through the steep bits if you optimized for the overall efficiency by using a bigger ring up front.

## Howto

Just run `cargo run -- /path/to/activity.fit 40 42` and wait for an output similar to:

```
Parsing FIT files using Profile version: "21.171.00"
rear gear weighted median: 8.0
  rear gear weighted mean: 7.603996499416572

--- Chainring comparison: 40T -> 42T (factor: 1.050) ---
Lightest gear used in 3 section(s) while pedalling, total 574s

  Section 1 (68:31-76:37, 486s): avg 219W -> would need 230W (+11W)

  Section 2 (66:11-66:42, 31s): avg 265W -> would need 279W (+13W)

  Section 3 (59:28-60:25, 57s): avg 221W -> would need 233W (+11W)
```
