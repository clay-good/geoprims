//! Tides (add-marine-navigation, navigation/tides): predicting heights and
//! highs and lows from a station's pasted NOAA harmonic constituents, and
//! converting a height between the tidal datums of a pasted datum sheet.
//! Nothing is fetched; both pastes are the reader's own copy of the
//! station's pages or its `harcon.json` and `datums.json`.

use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::civil;
use serde_json::Value;

const NOAA_DATUMS: Reference = Reference {
    title: "Tidal datums, NOAA Center for Operational Oceanographic Products and Services (CO-OPS)",
    issuer: "NOAA CO-OPS",
    year: 2026,
    edition: "station datum pages and the metadata API's datums, read 2026-10-20",
    locator: "A station's datum sheet: each datum's height above the station datum (STND), with LAT and HAT and the orthometric datum (NAVD88) where published",
    url: "https://tidesandcurrents.noaa.gov/datum_options.html",
};

/// Rows of a datum sheet that are ranges or time intervals, not heights.
const NOT_HEIGHTS: &[&str] = &["GT", "MN", "DHQ", "DLQ", "HWI", "LWI"];

/// A pasted datum sheet: each datum's height above the station datum, in
/// meters, in the order the sheet gives them.
pub(crate) struct Sheet {
    pub datums: Vec<(String, f64)>,
}

impl Sheet {
    pub fn get(&self, name: &str) -> Option<f64> {
        let want = name.trim().to_ascii_uppercase();
        self.datums
            .iter()
            .find(|(n, _)| *n == want)
            .map(|(_, v)| *v)
    }
    pub fn names(&self) -> String {
        self.datums
            .iter()
            .map(|(n, _)| n.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Reads a NOAA datum sheet: the metadata API's `datums.json`, or the rows of
/// the station page's table (name, value, description). `table_units` is the
/// unit of a pasted table, which carries none; the JSON names its own.
pub(crate) fn parse_sheet(text: &str, table_units: Option<&str>) -> Result<Sheet, ToolError> {
    let bad = |m: &str| ToolError::invalid("/sheet", m);
    let to_m = |units: &str| -> Result<f64, ToolError> {
        match units.to_ascii_lowercase().as_str() {
            "m" | "meters" | "metres" | "metric" => Ok(1.0),
            "ft" | "feet" | "english" => Ok(0.3048),
            other => Err(bad(&format!(
                "The datum sheet's units, {other}, are not meters or feet."
            ))),
        }
    };
    let mut datums: Vec<(String, f64)> = Vec::new();
    let mut push = |name: &str, v: f64, k: f64| {
        let name = name.trim().to_ascii_uppercase();
        if !NOT_HEIGHTS.contains(&name.as_str())
            && v.is_finite()
            && !datums.iter().any(|(n, _)| *n == name)
        {
            datums.push((name, v * k));
        }
    };
    let trimmed = text.trim();
    if trimmed.starts_with('{') {
        let v: Value = serde_json::from_str(trimmed)
            .map_err(|_| bad("The datum sheet looks like JSON but does not parse."))?;
        let k = to_m(
            v["units"]
                .as_str()
                .ok_or_else(|| bad("The datums.json paste has no units."))?,
        )?;
        for d in v["datums"]
            .as_array()
            .ok_or_else(|| bad("The datums.json paste has no datums list."))?
        {
            if let (Some(n), Some(x)) = (d["name"].as_str(), d["value"].as_f64()) {
                push(n, x, k);
            }
        }
        // LAT and HAT sit beside the list, as heights above the station datum.
        for n in ["LAT", "HAT"] {
            if let Some(x) = v[n].as_f64() {
                push(n, x, k);
            }
        }
    } else {
        let k = to_m(table_units.ok_or_else(|| {
            bad("A pasted table carries no units: choose meters or feet for the sheet.")
        })?)?;
        for line in trimmed.lines() {
            let mut words = line.split_whitespace();
            let (Some(name), Some(value)) = (words.next(), words.next()) else {
                continue;
            };
            let is_name = name.chars().all(|c| c.is_ascii_alphanumeric())
                && name.chars().any(|c| c.is_ascii_alphabetic());
            if let (true, Ok(x)) = (is_name, value.parse::<f64>()) {
                push(name, x, k);
            }
        }
    }
    if datums.len() < 2 {
        return Err(bad(
            "The datum sheet needs at least two datums, like MLLW and NAVD88, with their heights above the station datum.",
        ));
    }
    Ok(Sheet { datums })
}

fn m(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Length, "m").expect("m"),
    }
}

pub static DATUMS: ToolDef = ToolDef {
    id: "navigation.tide.datums",
    title: "Tidal datum conversion",
    summary: "Converts a height between the tidal datums of a NOAA station's datum sheet that you paste, such as from MLLW to NAVD88, by the differences the sheet gives.",
    aliases: &["tidal datum converter", "MLLW to NAVD88", "chart datum conversion"],
    keywords: &["tide", "datum", "MLLW", "MHHW", "NAVD88", "MSL", "chart datum"],
    inputs: &[
        Field::new(
            "sheet",
            "Datum sheet",
            "Paste the datum table from the station's NOAA page, or its datums download, like Boston's",
            Kind::Text { max_len: 20_000 },
        )
        .required()
        .core(),
        Field::new(
            "height",
            "Height",
            "Above the from datum, like 3.000 m",
            Kind::Quantity { q: QT::Length, unit: "m" },
        )
        .required()
        .core(),
        Field::new("from", "From datum", "As the sheet names it, like MLLW", Kind::Text { max_len: 16 })
            .required()
            .core(),
        Field::new("to", "To datum", "As the sheet names it, like NAVD88", Kind::Text { max_len: 16 })
            .required()
            .core(),
        Field::new(
            "sheet_units",
            "Sheet units",
            "m or ft, for a pasted table (the download names its own), like m",
            Kind::Choice(&["m", "ft"]),
        ),
    ],
    outputs: &[
        Field::new("converted", "Height above the to datum", "Height + from datum − to datum", Kind::Quantity { q: QT::Length, unit: "m" })
            .precision(Precision::Decimals(3)),
        Field::new("to_datum", "To datum", "", Kind::Text { max_len: 16 }),
        Field::new("from_datum", "From datum", "", Kind::Text { max_len: 16 }),
        Field::new("difference", "To datum above the from datum", "To datum − from datum, above the station datum", Kind::Quantity { q: QT::Length, unit: "m" })
            .precision(Precision::Decimals(3)),
        Field::new("datums", "Datums on the sheet", "Each above the station datum, in meters", Kind::Text { max_len: 2_000 }),
    ],
    errors: &[gp_base::ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Height above the to datum = height above the from datum + (from datum − to datum), each datum's height above the station datum (STND) read from the pasted sheet",
    accuracy: "Exact to the sheet's values, which NOAA publishes to a millimeter or a thousandth of a foot for its datum epoch",
    when_to_use: "Use this to move a height between tidal datums at one station: a charted depth or a tide height on MLLW into NAVD88 for a survey, or a land height into chart datum. Paste the station's datum sheet from NOAA, enter the height and the two datums, and it does the subtraction and shows each datum it used.",
    limitations: "It uses only the sheet you paste, for that station and its datum epoch; heights between stations or across epochs need NOAA's VDatum. It refuses a datum the sheet does not list, and it leaves out the sheet's ranges and intervals (GT, MN, DHQ, DLQ, HWI, LWI), which are not heights.",
    references: &[NOAA_DATUMS],
    examples: &[Example {
        id: "primary",
        title: "Boston, 3.000 m MLLW to NAVD88",
        input: r#"{"sheet":"{\"units\":\"meters\",\"datums\":[{\"name\":\"STND\",\"value\":0.0},{\"name\":\"MHHW\",\"value\":4.205},{\"name\":\"MHW\",\"value\":4.071},{\"name\":\"MTL\",\"value\":2.624},{\"name\":\"MSL\",\"value\":2.66},{\"name\":\"MLW\",\"value\":1.178},{\"name\":\"MLLW\",\"value\":1.074},{\"name\":\"NAVD88\",\"value\":2.752}]}","height":"3.000 m","from":"MLLW","to":"NAVD88"}"#,
        source: "NOAA CO-OPS datums for Boston, MA (8443970), epoch 1983-2001: MLLW 1.074 m and NAVD88 2.752 m above STND, so 3.000 m MLLW is 1.322 m NAVD88",
    }],
    primary_example: "primary",
    visualization: &[Layer { kind: "vector-diagram", map: &[] }],
    related: &[
        Related { id: "geodesy.height.convert", reason: "alternative" },
        Related { id: "navigation.piloting.distance-by-vertical-angle", reason: "next" },
        Related { id: "navigation.sailing.plane", reason: "alternative" },
    ],
    sentence: "That height is {converted} above {to_datum}.",
    limits: &[("batchRows", 10_000)],
    run: run_datums,
    ..ToolDef::BLANK
};

fn run_datums(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let units = ctx.choice("sheet_units")?;
    let text = ctx.text("sheet")?.expect("required");
    let sheet = parse_sheet(&text, units)?;
    let h = ctx
        .req_quantity("height")?
        .to(units::by_symbol(QT::Length, "m").expect("m"));
    let pick = |ctx: &Ctx, field: &str| -> Result<(String, f64), ToolError> {
        let name = ctx.text(field)?.expect("required").to_ascii_uppercase();
        sheet.get(&name).map(|v| (name.clone(), v)).ok_or_else(|| {
            ToolError::invalid(
                &format!("/{field}"),
                format!("The sheet has no {name}. It lists {}.", sheet.names()),
            )
        })
    };
    let (from, from_v) = pick(ctx, "from")?;
    let (to, to_v) = pick(ctx, "to")?;
    let list = sheet
        .datums
        .iter()
        .map(|(n, v)| format!("{n} {v:.3}"))
        .collect::<Vec<_>>()
        .join(", ");
    Ok(Json::obj([
        ("converted", ctx.out("converted", m(h + from_v - to_v))),
        ("to_datum", Json::str(to)),
        ("from_datum", Json::str(from)),
        ("difference", ctx.out("difference", m(to_v - from_v))),
        ("datums", Json::str(format!("{list} (m above STND)"))),
    ]))
}

// ------------------------------------------------------------- tide engine

/// The astronomical arguments a constituent's phase and node factors are built
/// from, in degrees, at one instant: the mean longitudes of the Moon (s), Sun
/// (h), lunar perigee (p), lunar node (N), and solar perigee (pp) from the
/// Meeus polynomials pytides uses, with Schureman's I, ξ, ν, ν′, 2ν″, and
/// P = p − ξ (SP 98, Table 6 notes and equations 224 and 232).
pub(crate) struct Astro {
    /// [T + h − s, s, h, p, N, pp, 90] for the constituent coefficients.
    args: [f64; 7],
    /// Their rates in degrees per hour.
    speeds: [f64; 7],
    i: f64,
    omega: f64,
    big_i: f64,
    xi: f64,
    nu: f64,
    nup: f64,
    nupp: f64,
    big_p: f64,
}

fn poly(c: &[f64], t: f64) -> f64 {
    c.iter().rev().fold(0.0, |acc, k| acc * t + k)
}
fn dpoly(c: &[f64], t: f64) -> f64 {
    c.iter()
        .enumerate()
        .skip(1)
        .rev()
        .fold(0.0, |acc, (i, k)| acc * t + i as f64 * k)
}
fn wrap360(x: f64) -> f64 {
    x.rem_euclid(360.0)
}
fn sd(deg: f64, min: f64, sec: f64) -> f64 {
    deg + min / 60.0 + sec / 3600.0
}

/// Julian date of a Unix time in seconds (UTC, treated as uniform time).
fn julian(unix: f64) -> f64 {
    unix / 86_400.0 + 2_440_587.5
}

impl Astro {
    pub fn at(unix: f64) -> Astro {
        let jd = julian(unix);
        let t = (jd - 2_451_545.0) / 36_525.0;
        // Meeus, as pytides takes it: mean longitudes in Julian centuries.
        const S: [f64; 4] = [
            218.316_459_1,
            481_267.881_342_36,
            -0.001_326_8,
            1.0 / 538_841.0 - 1.0 / 65_194_000.0,
        ];
        const H: [f64; 3] = [280.466_45, 36_000.769_83, 0.000_303_2];
        const P: [f64; 5] = [
            83.353_243,
            4_069.013_711_1,
            -0.010_323_8,
            -1.0 / 80_053.0,
            1.0 / 18_999_000.0,
        ];
        const N: [f64; 5] = [
            125.044_555,
            -1_934.136_184_9,
            0.002_076_2,
            1.0 / 467_410.0,
            -1.0 / 60_616_000.0,
        ];
        const PP: [f64; 4] = [
            280.466_45 - 357.529_1,
            36_000.769_32 - 35_999.050_3,
            0.000_303_2 + 0.000_155_9,
            0.000_000_48,
        ];
        // Meeus 22.3, in units of 10,000 years, rescaled to centuries.
        let obliquity_u = [
            sd(23.0, 26.0, 21.448),
            -sd(0.0, 0.0, 4_680.93),
            -sd(0.0, 0.0, 1.55),
            sd(0.0, 0.0, 1_999.25),
            -sd(0.0, 0.0, 51.38),
            -sd(0.0, 0.0, 249.67),
            -sd(0.0, 0.0, 39.05),
            sd(0.0, 0.0, 7.12),
            sd(0.0, 0.0, 27.87),
            sd(0.0, 0.0, 5.79),
            sd(0.0, 0.0, 2.45),
        ];
        let mut obliquity = [0.0; 11];
        for (k, c) in obliquity_u.iter().enumerate() {
            obliquity[k] = c * 1e-2_f64.powi(k as i32);
        }
        let per_hour = 1.0 / (24.0 * 365.25 * 100.0);
        let (s, h, p, n, pp) = (
            poly(&S, t),
            poly(&H, t),
            poly(&P, t),
            poly(&N, t),
            poly(&PP, t),
        );
        let (ds, dh, dp, dn, dpp) = (
            dpoly(&S, t) * per_hour,
            dpoly(&H, t) * per_hour,
            dpoly(&P, t) * per_hour,
            dpoly(&N, t) * per_hour,
            dpoly(&PP, t) * per_hour,
        );
        let omega = wrap360(poly(&obliquity, t));
        let i: f64 = 5.145;
        // Hour angle as pytides takes it: the fraction of the Julian day.
        let hour = (jd - jd.floor()) * 360.0;
        let (nr, ir, wr) = (n.to_radians(), i.to_radians(), omega.to_radians());
        let big_i = (ir.cos() * wr.cos() - ir.sin() * wr.sin() * nr.cos()).acos();
        let e1 = ((0.5 * (wr - ir)).cos() / (0.5 * (wr + ir)).cos() * (0.5 * nr).tan()).atan()
            - 0.5 * nr;
        let e2 = ((0.5 * (wr - ir)).sin() / (0.5 * (wr + ir)).sin() * (0.5 * nr).tan()).atan()
            - 0.5 * nr;
        let xi = -(e1 + e2);
        let nu = e1 - e2;
        let nup = (2.0 * big_i).sin() * nu.sin();
        let nup = nup.atan2((2.0 * big_i).sin() * nu.cos() + 0.3347);
        let nupp: f64 = 0.5
            * (big_i.sin().powi(2) * (2.0 * nu).sin())
                .atan2(big_i.sin().powi(2) * (2.0 * nu).cos() + 0.0727);
        // pytides wraps the angles, then reads them back; the same here.
        let xi_d = wrap360(xi.to_degrees());
        Astro {
            args: [
                wrap360(hour + h - s),
                wrap360(s),
                wrap360(h),
                wrap360(p),
                wrap360(n),
                wrap360(pp),
                90.0,
            ],
            speeds: [15.0 + dh - ds, ds, dh, dp, dn, dpp, 0.0],
            i,
            omega,
            big_i: big_i.to_degrees(),
            xi: xi_d,
            nu: wrap360(nu.to_degrees()),
            nup: wrap360(nup.to_degrees()),
            nupp: wrap360(nupp.to_degrees()),
            big_p: wrap360(wrap360(p) - xi_d),
        }
    }
}

/// Node factor (f) formulas, Schureman SP 98 equations 65 to 78 and 195 to 235.
#[derive(Clone, Copy)]
enum F {
    One,
    Mm,
    Mf,
    O1,
    J1,
    Oo1,
    M2,
    K1,
    L2,
    K2,
    M1,
    /// f(M2)^(n/2), for M3.
    Modd(f64),
}
/// Equilibrium argument corrections (u), Schureman Table 2.
#[derive(Clone, Copy)]
enum U {
    Zero,
    Mf,
    O1,
    J1,
    Oo1,
    M2,
    K1,
    L2,
    K2,
    M1,
    Modd(f64),
}

fn f_of(k: F, a: &Astro) -> f64 {
    let r = f64::to_radians;
    let (w, i, ib) = (r(a.omega), r(a.i), r(a.big_i));
    let f_m2 = || (0.5 * ib).cos().powi(4) / ((0.5 * w).cos().powi(4) * (0.5 * i).cos().powi(4));
    let f_o1 = || {
        (ib.sin() * (0.5 * ib).cos().powi(2))
            / (w.sin() * (0.5 * w).cos().powi(2) * (0.5 * i).cos().powi(4))
    };
    match k {
        F::One => 1.0,
        F::Mm => {
            (2.0 / 3.0 - ib.sin().powi(2))
                / ((2.0 / 3.0 - w.sin().powi(2)) * (1.0 - 1.5 * i.sin().powi(2)))
        }
        F::Mf => ib.sin().powi(2) / (w.sin().powi(2) * (0.5 * i).cos().powi(4)),
        F::O1 => f_o1(),
        F::J1 => (2.0 * ib).sin() / ((2.0 * w).sin() * (1.0 - 1.5 * i.sin().powi(2))),
        F::Oo1 => {
            ib.sin() * (0.5 * ib).sin().powi(2)
                / (w.sin() * (0.5 * w).sin().powi(2) * (0.5 * i).cos().powi(4))
        }
        F::M2 => f_m2(),
        F::K1 => {
            let nu = r(a.nu);
            let mean = 0.5023 * (2.0 * w).sin() * (1.0 - 1.5 * i.sin().powi(2)) + 0.1681;
            (0.2523 * (2.0 * ib).sin().powi(2) + 0.1689 * (2.0 * ib).sin() * nu.cos() + 0.0283)
                .sqrt()
                / mean
        }
        F::L2 => {
            let p = r(a.big_p);
            f_m2()
                * (1.0 - 12.0 * (0.5 * ib).tan().powi(2) * (2.0 * p).cos()
                    + 36.0 * (0.5 * ib).tan().powi(4))
                .sqrt()
        }
        F::K2 => {
            let nu = r(a.nu);
            let mean = 0.5023 * w.sin().powi(2) * (1.0 - 1.5 * i.sin().powi(2)) + 0.0365;
            (0.2533 * ib.sin().powi(4) + 0.0367 * ib.sin().powi(2) * (2.0 * nu).cos() + 0.0013)
                .sqrt()
                / mean
        }
        F::M1 => {
            // Schureman equation 195: f(M1) = f(O1) / Q_a.
            let p = r(a.big_p);
            let c = ib.cos() / (0.5 * ib).cos().powi(2);
            f_o1() * (0.25 + 1.5 * c * (2.0 * p).cos() + 2.25 * c * c).sqrt()
        }
        F::Modd(n) => f_m2().powf(n / 2.0),
    }
}

/// NOAA's M1 phase convention, measured: with Schureman's composite M1 (u =
/// −ν − Q_a, equations 190 to 196), NOAA's published predictions need a
/// further +20.5° in the argument, the same at every station and year tried
/// (Seattle 2019 to 2029; San Francisco, Boston, and Honolulu 2021 and 2026).
const NOAA_M1_OFFSET: f64 = 20.5;

fn u_of(k: U, a: &Astro) -> f64 {
    let r = f64::to_radians;
    let u_m2 = 2.0 * a.xi - 2.0 * a.nu;
    match k {
        U::Zero => 0.0,
        U::Mf => -2.0 * a.xi,
        U::O1 => 2.0 * a.xi - a.nu,
        U::J1 => -a.nu,
        U::Oo1 => -2.0 * a.xi - a.nu,
        U::M2 => u_m2,
        U::K1 => -a.nup,
        U::L2 => {
            let (ib, p) = (r(a.big_i), r(a.big_p));
            let big_r = (2.0 * p)
                .sin()
                .atan2(1.0 / 6.0 / (0.5 * ib).tan().powi(2) - (2.0 * p).cos());
            u_m2 - big_r.to_degrees()
        }
        U::K2 => -2.0 * a.nupp,
        U::M1 => {
            let (ib, p) = (r(a.big_i), r(a.big_p));
            let qa = (2.0 * p)
                .sin()
                .atan2(3.0 * ib.cos() / (0.5 * ib).cos().powi(2) + (2.0 * p).cos());
            -a.nu - qa.to_degrees() + NOAA_M1_OFFSET
        }
        U::Modd(n) => n / 2.0 * u_m2,
    }
}

/// A constituent: its coefficients on [T + h − s, s, h, p, N, pp, 90] with
/// its f and u, or a sum of other constituents with integer multipliers.
enum Def {
    Base([i8; 7], F, U),
    Compound(&'static [(&'static str, i8)]),
}

/// The 37 constituents of NOAA's harmonic constants, by NOAA's names.
const CONSTITUENTS: [(&str, Def); 37] = [
    ("M2", Def::Base([2, 0, 0, 0, 0, 0, 0], F::M2, U::M2)),
    ("S2", Def::Base([2, 2, -2, 0, 0, 0, 0], F::One, U::Zero)),
    ("N2", Def::Base([2, -1, 0, 1, 0, 0, 0], F::M2, U::M2)),
    ("K1", Def::Base([1, 1, 0, 0, 0, 0, -1], F::K1, U::K1)),
    ("M4", Def::Compound(&[("M2", 2)])),
    ("O1", Def::Base([1, -1, 0, 0, 0, 0, 1], F::O1, U::O1)),
    ("M6", Def::Compound(&[("M2", 3)])),
    ("MK3", Def::Compound(&[("M2", 1), ("K1", 1)])),
    ("S4", Def::Compound(&[("S2", 2)])),
    ("MN4", Def::Compound(&[("M2", 1), ("N2", 1)])),
    ("NU2", Def::Base([2, -1, 2, -1, 0, 0, 0], F::M2, U::M2)),
    ("S6", Def::Compound(&[("S2", 3)])),
    ("MU2", Def::Compound(&[("M2", 2), ("S2", -1)])),
    ("2N2", Def::Base([2, -2, 0, 2, 0, 0, 0], F::M2, U::M2)),
    ("OO1", Def::Base([1, 3, 0, 0, 0, 0, -1], F::Oo1, U::Oo1)),
    ("LAM2", Def::Base([2, 1, -2, 1, 0, 0, 2], F::M2, U::M2)),
    ("S1", Def::Base([1, 1, -1, 0, 0, 0, 0], F::One, U::Zero)),
    ("M1", Def::Base([1, 0, 0, 1, 0, 0, -1], F::M1, U::M1)),
    ("J1", Def::Base([1, 2, 0, -1, 0, 0, -1], F::J1, U::J1)),
    ("MM", Def::Base([0, 1, 0, -1, 0, 0, 0], F::Mm, U::Zero)),
    ("SSA", Def::Base([0, 0, 2, 0, 0, 0, 0], F::One, U::Zero)),
    ("SA", Def::Base([0, 0, 1, 0, 0, 0, 0], F::One, U::Zero)),
    ("MSF", Def::Compound(&[("S2", 1), ("M2", -1)])),
    ("MF", Def::Base([0, 2, 0, 0, 0, 0, 0], F::Mf, U::Mf)),
    ("RHO", Def::Base([1, -2, 2, -1, 0, 0, 1], F::O1, U::O1)),
    ("Q1", Def::Base([1, -2, 0, 1, 0, 0, 1], F::O1, U::O1)),
    ("T2", Def::Base([2, 2, -3, 0, 0, 1, 0], F::One, U::Zero)),
    ("R2", Def::Base([2, 2, -1, 0, 0, -1, 2], F::One, U::Zero)),
    ("2Q1", Def::Base([1, -3, 0, 2, 0, 0, 1], F::O1, U::O1)),
    ("P1", Def::Base([1, 1, -2, 0, 0, 0, 1], F::One, U::Zero)),
    ("2SM2", Def::Compound(&[("S2", 2), ("M2", -1)])),
    (
        "M3",
        Def::Base([3, 0, 0, 0, 0, 0, 0], F::Modd(3.0), U::Modd(3.0)),
    ),
    ("L2", Def::Base([2, 1, 0, -1, 0, 0, 2], F::L2, U::L2)),
    ("2MK3", Def::Compound(&[("M2", 2), ("K1", -1)])),
    ("K2", Def::Base([2, 2, 0, 0, 0, 0, 0], F::K2, U::K2)),
    ("M8", Def::Compound(&[("M2", 4)])),
    ("MS4", Def::Compound(&[("M2", 1), ("S2", 1)])),
];

/// V (degrees), u (degrees), f, and speed (degrees per hour) of a constituent.
pub(crate) fn vuf(name: &str, at_v: &Astro, at_fu: &Astro) -> Option<(f64, f64, f64, f64)> {
    let (_, def) = CONSTITUENTS.iter().find(|(n, _)| *n == name)?;
    Some(match def {
        Def::Base(c, f, u) => {
            let dot = |x: &[f64; 7]| c.iter().zip(x).map(|(k, v)| f64::from(*k) * v).sum::<f64>();
            (
                dot(&at_v.args),
                u_of(*u, at_fu),
                f_of(*f, at_fu),
                dot(&at_v.speeds),
            )
        }
        Def::Compound(members) => {
            let (mut v, mut u, mut f, mut sp) = (0.0, 0.0, 1.0, 0.0);
            for (m, k) in *members {
                let (mv, mu, mf, ms) = vuf(m, at_v, at_fu)?;
                let k = f64::from(*k);
                v += k * mv;
                u += k * mu;
                sp += k * ms;
                f *= mf.powf(k.abs());
            }
            (v, u, f, sp)
        }
    })
}

/// A constituent's node factor f (at 2 July 00:00 UTC) and equilibrium
/// argument V₀ + u (V₀ at 1 January 00:00 UTC, u at 2 July), in degrees from
/// 0 to 360, for a year: the values a prediction for that year uses.
pub fn year_factors(name: &str, year: i64) -> Option<(f64, f64)> {
    let (y0, mid) = year_bounds(year);
    let (v0, u, f, _) = vuf(name, &Astro::at(y0), &Astro::at(mid))?;
    Some((f, (v0 + u).rem_euclid(360.0)))
}

/// The names of all 37, in NOAA's order.
pub(crate) fn constituent_names() -> impl Iterator<Item = &'static str> {
    CONSTITUENTS.iter().map(|(n, _)| *n)
}

// ----------------------------------------------------- constituent parsing

/// One pasted constituent: NOAA name, amplitude (m), Greenwich phase κ (°),
/// and speed (°/h).
pub(crate) struct Row {
    pub name: String,
    pub amp: f64,
    pub phase: f64,
    pub speed: f64,
}

fn noaa_name(s: &str) -> String {
    s.trim().to_ascii_uppercase()
}

/// Reads NOAA's `harcon.json`, or the rows of the station page's table
/// (number, name, amplitude, phase, speed, description). A pasted table names
/// neither its units nor its phase reference, so those come from the reader;
/// local phases convert to Greenwich by κ = κ′ − speed × UTC offset (hours).
pub(crate) fn parse_constituents(
    text: &str,
    table_units: Option<&str>,
    local: bool,
    utc_offset_h: Option<f64>,
) -> Result<Vec<Row>, ToolError> {
    let bad = |m: &str| ToolError::invalid("/constituents", m);
    let to_m = |u: &str| -> Result<f64, ToolError> {
        match u.to_ascii_lowercase().as_str() {
            "m" | "meters" | "metres" | "metric" => Ok(1.0),
            "ft" | "feet" | "english" => Ok(0.3048),
            other => Err(bad(&format!(
                "The constituents' units, {other}, are not meters or feet."
            ))),
        }
    };
    let mut rows: Vec<Row> = Vec::new();
    let trimmed = text.trim();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        let v: Value = serde_json::from_str(trimmed)
            .map_err(|_| bad("The constituents look like JSON but do not parse."))?;
        let k = match v["units"].as_str() {
            Some(u) => to_m(u)?,
            None => to_m(table_units.ok_or_else(|| {
                bad("The harcon.json paste has no units: choose meters or feet.")
            })?)?,
        };
        let list = v["HarmonicConstituents"]
            .as_array()
            .or_else(|| v.as_array())
            .ok_or_else(|| bad("The paste has no HarmonicConstituents list."))?;
        for c in list {
            let (Some(name), Some(amp), Some(speed)) = (
                c["name"].as_str(),
                c["amplitude"].as_f64(),
                c["speed"].as_f64(),
            ) else {
                continue;
            };
            let phase = if local {
                c["phase_local"].as_f64().map(|p| (p, true))
            } else {
                c["phase_GMT"].as_f64().map(|p| (p, false))
            };
            let Some((p, is_local)) = phase.or_else(|| c["phase_GMT"].as_f64().map(|p| (p, false)))
            else {
                continue;
            };
            let p = if is_local {
                p - speed * utc_offset_h.unwrap_or(0.0)
            } else {
                p
            };
            rows.push(Row {
                name: noaa_name(name),
                amp: amp * k,
                phase: p.rem_euclid(360.0),
                speed,
            });
        }
    } else {
        let k = to_m(table_units.ok_or_else(|| {
            bad("A pasted table carries no units: choose meters or feet for the constituents.")
        })?)?;
        if local && utc_offset_h.is_none() {
            return Err(ToolError::invalid(
                "/utc_offset",
                "Local phases need the station's standard-time UTC offset, like -5 h for Eastern Standard Time.",
            ));
        }
        for line in trimmed.lines() {
            let w: Vec<&str> = line.split_whitespace().collect();
            // [number] name amplitude phase speed [description...]
            let start = usize::from(w.first().is_some_and(|x| x.parse::<u32>().is_ok()));
            if w.len() < start + 4 {
                continue;
            }
            let (Ok(amp), Ok(phase), Ok(speed)) = (
                w[start + 1].parse::<f64>(),
                w[start + 2].parse::<f64>(),
                w[start + 3].parse::<f64>(),
            ) else {
                continue;
            };
            let p = if local {
                phase - speed * utc_offset_h.unwrap_or(0.0)
            } else {
                phase
            };
            rows.push(Row {
                name: noaa_name(w[start]),
                amp: amp * k,
                phase: p.rem_euclid(360.0),
                speed,
            });
        }
    }
    if rows.is_empty() {
        return Err(bad(
            "No constituents were found. Paste the station's harmonic constituents table or its harcon.json.",
        ));
    }
    for r in &rows {
        if !constituent_names().any(|n| n == r.name) {
            return Err(bad(&format!(
                "{} is not one of NOAA's 37 standard constituents; the tool predicts from those only.",
                r.name
            )));
        }
        if !(r.amp.is_finite()
            && r.amp >= 0.0
            && r.amp < 100.0
            && r.speed.is_finite()
            && r.phase.is_finite())
        {
            return Err(bad(&format!(
                "{} has an amplitude, phase, or speed out of range.",
                r.name
            )));
        }
    }
    Ok(rows)
}

// ------------------------------------------------------------- prediction

/// Days from 1970-01-01 of 1 January of a year, and Unix seconds of its
/// first instant and of NOAA's mid-year (2 July, 00:00 UTC).
fn year_bounds(year: i64) -> (f64, f64) {
    let y0 = civil::days_from_civil(year, 1, 1) as f64 * 86_400.0;
    let mid = civil::days_from_civil(year, 7, 2) as f64 * 86_400.0;
    (y0, mid)
}

/// Precomputed terms for one calendar year: f·H, speed, and V₀ + u − κ.
struct YearTerms {
    start: f64,
    end: f64,
    terms: Vec<(f64, f64, f64)>,
}

fn year_terms(rows: &[Row], year: i64) -> YearTerms {
    let (y0, mid) = year_bounds(year);
    let a0 = Astro::at(y0);
    let am = Astro::at(mid);
    let terms = rows
        .iter()
        .map(|r| {
            let (v0, u, f, _) = vuf(&r.name, &a0, &am).expect("checked when parsed");
            (f * r.amp, r.speed, v0 + u - r.phase)
        })
        .collect();
    YearTerms {
        start: y0,
        end: civil::days_from_civil(year + 1, 1, 1) as f64 * 86_400.0,
        terms,
    }
}

/// Height above mean sea level (m) and its rate (m/h) at Unix time `t`, with
/// f and u held at the year's middle as NOAA does.
fn height(y: &YearTerms, t: f64) -> (f64, f64) {
    let hrs = (t - y.start) / 3_600.0;
    let (mut h, mut dh) = (0.0, 0.0);
    for (a, sp, ph) in &y.terms {
        let x = (sp * hrs + ph).to_radians();
        h += a * libm::cos(x);
        dh -= a * sp.to_radians() * libm::sin(x);
    }
    (h, dh)
}

/// The same with f, u, and V taken at the instant itself (as XTide does).
fn height_instant(rows: &[Row], t: f64) -> f64 {
    let a = Astro::at(t);
    rows.iter()
        .map(|r| {
            let (v, u, f, _) = vuf(&r.name, &a, &a).expect("checked when parsed");
            f * r.amp * libm::cos((v + u - r.phase).to_radians())
        })
        .sum()
}

// ------------------------------------------------------------ predict tool

const SCHUREMAN: Reference = Reference {
    title: "Manual of Harmonic Analysis and Prediction of Tides, Special Publication No. 98",
    issuer: "Schureman, P., U.S. Coast and Geodetic Survey (NOAA)",
    year: 1958,
    edition: "Revised 1940 edition, reprinted 1958",
    locator: "Table 2 (constituent arguments and node factors) and paragraphs 121 to 122, equations 190 to 196 (the M1 composite)",
    url: "https://tidesandcurrents.noaa.gov/publications/SpecialPubNo98.pdf",
};
const NOAA_HARCON: Reference = Reference {
    title: "Harmonic constituents and tide predictions, NOAA CO-OPS",
    issuer: "NOAA CO-OPS",
    year: 2026,
    edition: "station harmonic constituents and the predictions API, read 2026-10-20",
    locator: "A station's 37 harmonic constituents with Greenwich and local phases; NOAA's published predictions are authoritative",
    url: "https://api.tidesandcurrents.noaa.gov/mdapi/prod/",
};

const MAX_DAYS: f64 = 31.0;

const EXTREME_ROW: &[Field] = &[
    Field::new("time", "Time", "", Kind::Text { max_len: 40 }),
    Field::new("type", "High or low", "", Kind::Text { max_len: 4 }),
    Field::new(
        "height",
        "Height",
        "",
        Kind::Quantity {
            q: QT::Length,
            unit: "ft",
        },
    )
    .precision(Precision::Decimals(2)),
];
const SERIES_ROW: &[Field] = &[
    Field::new("time", "Time", "", Kind::Text { max_len: 40 }),
    Field::new(
        "height",
        "Height",
        "",
        Kind::Quantity {
            q: QT::Length,
            unit: "ft",
        },
    )
    .precision(Precision::Decimals(3)),
];

pub static PREDICT: ToolDef = ToolDef {
    id: "navigation.tide.predict",
    title: "Tide prediction from harmonic constituents",
    summary: "Predicts the tide height and the times of high and low water at a NOAA station from the harmonic constituents and datum sheet you paste, by Schureman's harmonic method as NOAA applies it. NOAA's published predictions are authoritative.",
    aliases: &["tide calculator", "tide prediction", "high and low tide times"],
    keywords: &["tide", "high water", "low water", "harmonic", "constituents", "MLLW", "NOAA"],
    inputs: &[
        Field::new("constituents", "Harmonic constituents", "Paste the harmonic constituents table from the station's NOAA page, or its constituents download, like Boston's (8443970)", Kind::Text { max_len: 20_000 }).required().core(),
        Field::new("datum_sheet", "Datum sheet", "Paste the station's datum table or datums download, like Boston's (8443970), for heights above chart datum", Kind::Text { max_len: 20_000 }).core(),
        Field::new("start", "Start", "Date and time, like 2026-10-21 or 2026-10-21T06:00", Kind::Text { max_len: 40 }).required().core(),
        Field::new("days", "Days", "How many days to predict, up to 31, like 1", Kind::Number { min: 0.0, max: 10_000.0 }).core(),
        Field::new("datum", "Datum", "Heights above this datum on the sheet, like MLLW (the default)", Kind::Text { max_len: 16 }).core(),
        Field::new("interval", "Interval", "Between heights in the series, like 6 min (the default) or 1 h", Kind::Quantity { q: QT::Time, unit: "min" }),
        Field::new("utc_offset", "Time zone", "UTC offset for the times, like -05:00 for the station's standard time; default UTC", Kind::Text { max_len: 10 }),
        Field::new("units", "Paste units", "m or ft, for a pasted table (the download names its own), like ft", Kind::Choice(&["m", "ft"])),
        Field::new("phases", "Phase reference", "gmt (the default) or local, as the pasted table shows them, like gmt", Kind::Choice(&["gmt", "local"])),
        Field::new("partial", "Partial constituents", "acknowledge, to predict from fewer than NOAA's 37", Kind::Choice(&["acknowledge"])),
        Field::new("node_factors", "Node factors", "mid-year (NOAA's practice, the default) or instant, like mid-year", Kind::Choice(&["mid-year", "instant"])),
    ],
    outputs: &[
        Field::new("high", "Highest predicted", "In the window", Kind::Quantity { q: QT::Length, unit: "ft" }).precision(Precision::Decimals(2)),
        Field::new("high_time", "Time of the highest", "", Kind::Text { max_len: 40 }),
        Field::new("low", "Lowest predicted", "In the window", Kind::Quantity { q: QT::Length, unit: "ft" }).precision(Precision::Decimals(2)),
        Field::new("low_time", "Time of the lowest", "", Kind::Text { max_len: 40 }),
        Field::new("extremes", "Highs and lows", "Each high and low water in the window", Kind::List { items: EXTREME_ROW, min: 0, max: 200 }),
        Field::new("series", "Heights", "At the chosen interval", Kind::List { items: SERIES_ROW, min: 1, max: 10_000 }),
        Field::new("datum_used", "Datum", "What the heights are above", Kind::Text { max_len: 40 }),
        Field::new("method", "Method", "Constituents used and the node-factor year", Kind::Text { max_len: 400 }),
        Field::new("notice", "Notice", "", Kind::Text { max_len: 160 }),
    ],
    errors: &[gp_base::ErrorCode::InvalidInput, gp_base::ErrorCode::LimitExceeded],
    warnings: &["PARTIAL_CONSTITUENTS", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "h(t) = (MSL − datum) + Σ f·H·cos(a·t + V₀ + u − κ): V₀ at 1 January 00:00 UTC of each year from Meeus mean longitudes, f and u at 2 July (NOAA's middle of the year) from Schureman SP 98 Table 2, NOAA's speeds a and Greenwich phases κ from the paste; NOAA's M1 is Schureman's composite (equations 190 to 196) with a measured +20.5° convention offset; highs and lows where dh/dt = 0, found to 1 second",
    accuracy: "Within 0.02 ft of NOAA's own predictions at the stations tested (Boston, Seattle, San Francisco, Key West, Honolulu), from the same 37 constituents; NOAA's published predictions are authoritative",
    when_to_use: "Use this to predict tides offline from a NOAA station's harmonic constituents: paste the constituents and the datum sheet from the station's page once, then predict any day. It gives the heights at your interval and the times and heights of high and low water, above chart datum. Check it against NOAA's published predictions before relying on it.",
    limitations: "It predicts the astronomical tide only: wind, pressure, and river flow move the real water, sometimes by feet. It uses NOAA's 37 standard constituents, so stations NOAA predicts with more (Anchorage uses 120) or with subordinate-station corrections are not matched. Windows are at most 31 days. Times are UTC or the fixed offset you give, without daylight time.",
    references: &[SCHUREMAN, NOAA_HARCON],
    examples: &[Example {
        id: "primary",
        title: "Boston, 21 October 2026",
        input: r#"{"constituents":"{\"units\":\"feet\",\"HarmonicConstituents\":[{\"name\":\"M2\",\"amplitude\":4.498,\"phase_GMT\":109.2,\"speed\":28.984104},{\"name\":\"S2\",\"amplitude\":0.682,\"phase_GMT\":146.2,\"speed\":30.0},{\"name\":\"N2\",\"amplitude\":1.001,\"phase_GMT\":76.7,\"speed\":28.43973},{\"name\":\"K1\",\"amplitude\":0.469,\"phase_GMT\":205.6,\"speed\":15.041069},{\"name\":\"M4\",\"amplitude\":0.075,\"phase_GMT\":29.0,\"speed\":57.96821},{\"name\":\"O1\",\"amplitude\":0.381,\"phase_GMT\":187.1,\"speed\":13.943035},{\"name\":\"M6\",\"amplitude\":0.095,\"phase_GMT\":281.3,\"speed\":86.95232},{\"name\":\"MK3\",\"amplitude\":0.02,\"phase_GMT\":231.8,\"speed\":44.025173},{\"name\":\"S4\",\"amplitude\":0.007,\"phase_GMT\":119.3,\"speed\":60.0},{\"name\":\"MN4\",\"amplitude\":0.036,\"phase_GMT\":13.7,\"speed\":57.423832},{\"name\":\"NU2\",\"amplitude\":0.217,\"phase_GMT\":84.0,\"speed\":28.512583},{\"name\":\"S6\",\"amplitude\":0.007,\"phase_GMT\":246.8,\"speed\":90.0},{\"name\":\"MU2\",\"amplitude\":0.033,\"phase_GMT\":53.3,\"speed\":27.968208},{\"name\":\"2N2\",\"amplitude\":0.135,\"phase_GMT\":53.7,\"speed\":27.895355},{\"name\":\"OO1\",\"amplitude\":0.013,\"phase_GMT\":218.7,\"speed\":16.139101},{\"name\":\"LAM2\",\"amplitude\":0.066,\"phase_GMT\":140.1,\"speed\":29.455626},{\"name\":\"S1\",\"amplitude\":0.016,\"phase_GMT\":131.6,\"speed\":15.0},{\"name\":\"M1\",\"amplitude\":0.016,\"phase_GMT\":223.2,\"speed\":14.496694},{\"name\":\"J1\",\"amplitude\":0.033,\"phase_GMT\":210.0,\"speed\":15.5854435},{\"name\":\"MM\",\"amplitude\":0.0,\"phase_GMT\":0.0,\"speed\":0.5443747},{\"name\":\"SSA\",\"amplitude\":0.072,\"phase_GMT\":97.5,\"speed\":0.0821373},{\"name\":\"SA\",\"amplitude\":0.161,\"phase_GMT\":136.3,\"speed\":0.0410686},{\"name\":\"MSF\",\"amplitude\":0.0,\"phase_GMT\":0.0,\"speed\":1.0158958},{\"name\":\"MF\",\"amplitude\":0.0,\"phase_GMT\":0.0,\"speed\":1.0980331},{\"name\":\"RHO\",\"amplitude\":0.016,\"phase_GMT\":179.3,\"speed\":13.471515},{\"name\":\"Q1\",\"amplitude\":0.062,\"phase_GMT\":172.1,\"speed\":13.398661},{\"name\":\"T2\",\"amplitude\":0.059,\"phase_GMT\":120.1,\"speed\":29.958933},{\"name\":\"R2\",\"amplitude\":0.013,\"phase_GMT\":13.1,\"speed\":30.041067},{\"name\":\"2Q1\",\"amplitude\":0.007,\"phase_GMT\":211.3,\"speed\":12.854286},{\"name\":\"P1\",\"amplitude\":0.154,\"phase_GMT\":204.8,\"speed\":14.958931},{\"name\":\"2SM2\",\"amplitude\":0.01,\"phase_GMT\":110.3,\"speed\":31.015896},{\"name\":\"M3\",\"amplitude\":0.007,\"phase_GMT\":149.2,\"speed\":43.47616},{\"name\":\"L2\",\"amplitude\":0.223,\"phase_GMT\":160.4,\"speed\":29.528479},{\"name\":\"2MK3\",\"amplitude\":0.023,\"phase_GMT\":204.7,\"speed\":42.92714},{\"name\":\"K2\",\"amplitude\":0.194,\"phase_GMT\":146.1,\"speed\":30.082138},{\"name\":\"M8\",\"amplitude\":0.02,\"phase_GMT\":241.3,\"speed\":115.93642},{\"name\":\"MS4\",\"amplitude\":0.03,\"phase_GMT\":74.0,\"speed\":58.984104}]}","datum_sheet":"{\"units\":\"feet\",\"datums\":[{\"name\":\"STND\",\"value\":0.0},{\"name\":\"MHHW\",\"value\":13.8},{\"name\":\"MHW\",\"value\":13.35},{\"name\":\"DTL\",\"value\":8.66},{\"name\":\"MTL\",\"value\":8.61},{\"name\":\"MSL\",\"value\":8.73},{\"name\":\"MLW\",\"value\":3.86},{\"name\":\"MLLW\",\"value\":3.53},{\"name\":\"NAVD88\",\"value\":9.03}]}","start":"2026-10-21","days":1,"interval":"60 min"}"#,
        source: "NOAA CO-OPS predictions for Boston, MA (8443970), 2026-10-21 GMT: low 1.515 ft at 05:46, high 8.551 ft at 12:06, low 1.928 ft at 18:05 (MLLW)",
    }],
    primary_example: "primary",
    visualization: &[Layer { kind: "table-only", map: &[] }],
    related: &[
        Related { id: "navigation.tide.datums", reason: "next" },
        Related { id: "navigation.sailing.plane", reason: "alternative" },
        Related { id: "time.sun.position", reason: "alternative" },
    ],
    sentence: "The highest tide is {high} at {high_time}, and the lowest is {low} at {low_time}.",
    limits: &[("batchRows", 100)],
    run: run_predict,
    ..ToolDef::BLANK
};

fn run_predict(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let units = ctx.choice("units")?;
    let local = ctx.choice("phases")? == Some("local");
    let offset_min = match ctx.text("utc_offset")? {
        Some(s) => civil::parse_offset(&s).map_err(|m| ToolError::invalid("/utc_offset", m))?,
        None => 0,
    };
    let offset_h = f64::from(offset_min) / 60.0;
    let text = ctx.text("constituents")?.expect("required");
    let rows = parse_constituents(&text, units, local, (local).then_some(offset_h))?;
    // NOAA's 37: the ones the paste lacks, named.
    let missing: Vec<&str> = constituent_names()
        .filter(|n| !rows.iter().any(|r| r.name == *n))
        .collect();
    if !missing.is_empty() {
        if ctx.choice("partial")? != Some("acknowledge") {
            return Err(ToolError::invalid(
                "/partial",
                format!(
                    "The paste lacks {} of NOAA's 37 constituents: {}. Paste the full table, or set partial to acknowledge to predict without them.",
                    missing.len(),
                    missing.join(", ")
                ),
            ));
        }
        ctx.warnings.push(Warning::new(
            "PARTIAL_CONSTITUENTS",
            format!(
                "Predicted without {}: {}.",
                missing.len(),
                missing.join(", ")
            ),
        ));
    }
    // Heights above the chosen datum: MSL − datum from the sheet, else MSL.
    let datum_name = ctx
        .text("datum")?
        .unwrap_or_else(|| "MLLW".into())
        .to_ascii_uppercase();
    let (z0, datum_used) = match ctx.text("datum_sheet")? {
        Some(s) => {
            let sheet = parse_sheet(&s, units)?;
            let msl = sheet.get("MSL").ok_or_else(|| {
                ToolError::invalid(
                    "/datum_sheet",
                    "The datum sheet has no MSL row, which the prediction is measured from.",
                )
            })?;
            let d = sheet.get(&datum_name).ok_or_else(|| {
                ToolError::invalid(
                    "/datum",
                    format!("The sheet has no {datum_name}. It lists {}.", sheet.names()),
                )
            })?;
            (msl - d, datum_name.clone())
        }
        None => (0.0, "MSL (no datum sheet given)".to_owned()),
    };
    let start = civil::parse_stamp(&ctx.text("start")?.expect("required"))
        .map_err(|m| ToolError::invalid("/start", m))?;
    // A start without a zone is in the output time zone.
    let t0 = start.days as f64 * 86_400.0 + start.secs
        - f64::from(start.offset.unwrap_or(offset_min)) * 60.0;
    let days = ctx.number("days")?.unwrap_or(1.0);
    if days.is_nan() || days <= 0.0 {
        return Err(ToolError::invalid("/days", "Days must be more than zero."));
    }
    if days > MAX_DAYS {
        return Err(ToolError::new(
            gp_base::ErrorCode::LimitExceeded,
            format!("A prediction covers at most {MAX_DAYS} days; ask for {MAX_DAYS} or fewer."),
        )
        .at("/days"));
    }
    let step = match ctx.quantity("interval")? {
        Some(q) => q.to(units::by_symbol(QT::Time, "s").expect("s")),
        None => 360.0,
    };
    if !(60.0..=86_400.0).contains(&step) {
        return Err(ToolError::invalid(
            "/interval",
            "The interval is from 1 minute to 1 day.",
        ));
    }
    let t1 = t0 + days * 86_400.0;
    let samples = ((t1 - t0) / step).floor() as usize + 1;
    if samples > 10_000 {
        return Err(ToolError::new(
            gp_base::ErrorCode::LimitExceeded,
            "That is more than 10,000 heights; use a longer interval or fewer days.",
        )
        .at("/interval"));
    }
    let instant = ctx.choice("node_factors")? == Some("instant");
    // Terms for each calendar year the window touches.
    let year_of = |t: f64| civil::civil_from_days((t / 86_400.0).floor() as i64).0;
    let years: Vec<YearTerms> = (year_of(t0)..=year_of(t1))
        .map(|y| year_terms(&rows, y))
        .collect();
    let terms_at = |t: f64| {
        years
            .iter()
            .find(|y| t >= y.start && t < y.end)
            .expect("window years")
    };
    let h_at = |t: f64| {
        if instant {
            height_instant(&rows, t)
        } else {
            height(terms_at(t), t).0
        }
    };
    let dh_at = |t: f64| {
        if instant {
            (height_instant(&rows, t + 1.0) - height_instant(&rows, t - 1.0)) / 2.0 * 3_600.0
        } else {
            height(terms_at(t), t).1
        }
    };
    // List heights use the unit the reader gets for the highest and lowest.
    let out_u = ctx.output_unit("high");
    let unit = out_u.symbol;
    let len = |v: f64| {
        Q {
            value: v,
            unit: units::by_symbol(QT::Length, "m").expect("m"),
        }
        .to(out_u)
    };
    let stamp = |t: f64| {
        let local = t + f64::from(offset_min) * 60.0;
        let days = (local / 86_400.0).floor();
        civil::iso(
            days as i64,
            (local - days * 86_400.0).round(),
            &civil::offset_text(offset_min, true),
        )
    };
    let q = |v: f64| Json::obj([("value", Json::Num(len(v))), ("unit", Json::str(unit))]);
    // The series.
    let mut series = Vec::with_capacity(samples);
    for k in 0..samples {
        let t = t0 + k as f64 * step;
        series.push(Json::obj([
            ("time", Json::str(stamp(t))),
            ("height", q(z0 + h_at(t))),
        ]));
    }
    // Highs and lows: sign changes of dh/dt on a 6-minute grid, then bisection to a second.
    let mut extremes = Vec::new();
    let mut extreme_times = Vec::new();
    let (mut hi, mut lo) = ((f64::NEG_INFINITY, t0), (f64::INFINITY, t0));
    let grid = 360.0;
    let mut a = t0;
    let mut da = dh_at(a);
    while a < t1 {
        let b = (a + grid).min(t1);
        let db = dh_at(b);
        if da > 0.0 && db <= 0.0 || da < 0.0 && db >= 0.0 {
            let (mut l, mut r) = (a, b);
            while r - l > 0.5 {
                let m = 0.5 * (l + r);
                if (dh_at(m) > 0.0) == (da > 0.0) {
                    l = m;
                } else {
                    r = m;
                }
            }
            let t = (0.5 * (l + r)).round();
            let h = z0 + h_at(t);
            let kind = if da > 0.0 { "H" } else { "L" };
            extremes.push(Json::obj([
                ("time", Json::str(stamp(t))),
                ("type", Json::str(kind)),
                ("height", q(h)),
            ]));
            extreme_times.push(t);
        }
        a = b;
        da = db;
    }
    // The window's highest and lowest, at an extreme or at an end.
    let mut consider = |t: f64| {
        let h = z0 + h_at(t);
        if h > hi.0 {
            hi = (h, t);
        }
        if h < lo.0 {
            lo = (h, t);
        }
    };
    consider(t0);
    consider(t1);
    for t in &extreme_times {
        consider(*t);
    }
    let fu_year = if instant {
        "f, u, and V at each instant".to_owned()
    } else {
        let ys: Vec<String> = (year_of(t0)..=year_of(t1)).map(|y| y.to_string()).collect();
        format!("f and u for the middle of {}", ys.join(" and "))
    };
    Ok(Json::obj([
        ("high", q(hi.0)),
        ("high_time", Json::str(stamp(hi.1))),
        ("low", q(lo.0)),
        ("low_time", Json::str(stamp(lo.1))),
        ("extremes", Json::Arr(extremes)),
        ("series", Json::Arr(series)),
        ("datum_used", Json::str(datum_used)),
        (
            "method",
            Json::str(format!("{} constituents; {fu_year}", rows.len())),
        ),
        (
            "notice",
            Json::str(
                "NOAA's published predictions are authoritative. Astronomical tide only: weather moves the real water.",
            ),
        ),
    ]))
}
