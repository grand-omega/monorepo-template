pub mod mailer;
pub mod routes;
pub mod service;
pub mod templates;

pub use mailer::{DynMailer, Mailer, NoopMailer, SmtpMailer};
