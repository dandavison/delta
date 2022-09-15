pub enum Health {
    Healthy,
    Unhealthy(String, String),
}

pub trait Diagnostic {
    fn report(&self) -> String;
    fn diagnose(&self) -> Health;
    fn remedy(&self) -> Option<String>;
}
