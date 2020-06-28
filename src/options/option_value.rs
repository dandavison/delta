use crate::config::delta_unreachable;

/// A value associated with a Delta command-line option name.
pub enum OptionValue {
    Boolean(bool),
    Float(f64),
    OptionString(Option<String>),
    String(String),
    Int(usize),
}

/// An OptionValue, tagged according to its provenance/semantics.
pub enum ProvenancedOptionValue {
    GitConfigValue(OptionValue),
    DefaultValue(OptionValue),
}

impl From<bool> for OptionValue {
    fn from(value: bool) -> Self {
        OptionValue::Boolean(value)
    }
}

impl From<OptionValue> for bool {
    fn from(value: OptionValue) -> Self {
        match value {
            OptionValue::Boolean(value) => value,
            _ => delta_unreachable("Error converting OptionValue to bool."),
        }
    }
}

impl From<f64> for OptionValue {
    fn from(value: f64) -> Self {
        OptionValue::Float(value)
    }
}

impl From<OptionValue> for f64 {
    fn from(value: OptionValue) -> Self {
        match value {
            OptionValue::Float(value) => value,
            _ => delta_unreachable("Error converting OptionValue to f64."),
        }
    }
}

impl From<Option<String>> for OptionValue {
    fn from(value: Option<String>) -> Self {
        OptionValue::OptionString(value)
    }
}

impl From<OptionValue> for Option<String> {
    fn from(value: OptionValue) -> Self {
        match value {
            OptionValue::OptionString(value) => value,
            _ => delta_unreachable("Error converting OptionValue to Option<String>."),
        }
    }
}

impl From<String> for OptionValue {
    fn from(value: String) -> Self {
        OptionValue::String(value)
    }
}

impl From<&str> for OptionValue {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}

impl From<OptionValue> for String {
    fn from(value: OptionValue) -> Self {
        match value {
            OptionValue::String(value) => value,
            _ => delta_unreachable("Error converting OptionValue to String."),
        }
    }
}

impl From<usize> for OptionValue {
    fn from(value: usize) -> Self {
        OptionValue::Int(value)
    }
}

impl From<OptionValue> for usize {
    fn from(value: OptionValue) -> Self {
        match value {
            OptionValue::Int(value) => value,
            _ => delta_unreachable("Error converting OptionValue to usize."),
        }
    }
}
