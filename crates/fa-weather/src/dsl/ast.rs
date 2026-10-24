use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agg {
    Min,
    Max,
    Sum,
    Avg,
    Any,
}

impl Agg {
    pub fn name(self) -> &'static str {
        match self {
            Agg::Min => "min",
            Agg::Max => "max",
            Agg::Sum => "sum",
            Agg::Avg => "avg",
            Agg::Any => "any",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    TempC,
    FeelsC,
    PrecipProb,
    PrecipMm,
    SnowCm,
    GustKmh,
    WindKmh,
    Uv,
    Code,
}

impl Field {
    pub const ALL: [Field; 9] = [
        Field::TempC,
        Field::FeelsC,
        Field::PrecipProb,
        Field::PrecipMm,
        Field::SnowCm,
        Field::GustKmh,
        Field::WindKmh,
        Field::Uv,
        Field::Code,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Field::TempC => "temp_c",
            Field::FeelsC => "feels_c",
            Field::PrecipProb => "precip_prob",
            Field::PrecipMm => "precip_mm",
            Field::SnowCm => "snow_cm",
            Field::GustKmh => "gust_kmh",
            Field::WindKmh => "wind_kmh",
            Field::Uv => "uv",
            Field::Code => "code",
        }
    }

    /// Open-Meteo hourly series name.
    pub fn series(self) -> &'static str {
        match self {
            Field::TempC => "temperature_2m",
            Field::FeelsC => "apparent_temperature",
            Field::PrecipProb => "precipitation_probability",
            Field::PrecipMm => "precipitation",
            Field::SnowCm => "snowfall",
            Field::GustKmh => "wind_gusts_10m",
            Field::WindKmh => "wind_speed_10m",
            Field::Uv => "uv_index",
            Field::Code => "weather_code",
        }
    }

    /// Human label for the `{field}` placeholder.
    pub fn label(self) -> &'static str {
        match self {
            Field::TempC => "temperature",
            Field::FeelsC => "feels-like temperature",
            Field::PrecipProb => "rain chance",
            Field::PrecipMm => "precipitation",
            Field::SnowCm => "snowfall",
            Field::GustKmh => "wind gusts",
            Field::WindKmh => "wind speed",
            Field::Uv => "UV index",
            Field::Code => "weather code",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Window {
    Next(u32),
    Today,
    Tonight,
    Tomorrow,
}

impl Window {
    /// Text for the `{when}` placeholder.
    pub fn label(self) -> String {
        match self {
            Window::Next(n) => format!("next {n} h"),
            Window::Today => "today".into(),
            Window::Tonight => "tonight".into(),
            Window::Tomorrow => "tomorrow".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Le,
    Lt,
    Ge,
    Gt,
    Eq,
    Ne,
}

impl Op {
    pub fn symbol(self) -> &'static str {
        match self {
            Op::Le => "<=",
            Op::Lt => "<",
            Op::Ge => ">=",
            Op::Gt => ">",
            Op::Eq => "==",
            Op::Ne => "!=",
        }
    }

    pub fn test(self, a: f64, b: f64) -> bool {
        const EPS: f64 = 1e-9;
        match self {
            Op::Le => a <= b + EPS,
            Op::Lt => a < b - EPS,
            Op::Ge => a >= b - EPS,
            Op::Gt => a > b + EPS,
            Op::Eq => (a - b).abs() <= EPS,
            Op::Ne => (a - b).abs() > EPS,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cond {
    pub agg: Agg,
    pub field: Field,
    pub window: Window,
    pub op: Op,
    pub value: f64,
}

/// `or` of `and` groups (`and` binds tighter).
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub groups: Vec<Vec<Cond>>,
}

impl fmt::Display for Cond {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let w = match self.window {
            Window::Next(n) => format!("next {n}h"),
            Window::Today => "today".into(),
            Window::Tonight => "tonight".into(),
            Window::Tomorrow => "tomorrow".into(),
        };
        write!(
            f,
            "{}({}, {}) {} {}",
            self.agg.name(),
            self.field.name(),
            w,
            self.op.symbol(),
            self.value
        )
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, g) in self.groups.iter().enumerate() {
            if i > 0 {
                f.write_str(" or ")?;
            }
            for (j, c) in g.iter().enumerate() {
                if j > 0 {
                    f.write_str(" and ")?;
                }
                write!(f, "{c}")?;
            }
        }
        Ok(())
    }
}
