use clap::Parser;
use once_cell::sync::Lazy;

pub static ARGS: Lazy<Args> = Lazy::new(Args::parse);

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Send email with given name to everyone who should have received it in the past.
    /// Should be used to send a email manually if it was added later.
    #[arg(short, long, value_name = "TEMPLATE_NAME")]
    pub send_mail_to_oldies: Option<String>,

    /// If set, no emails will be sent, only log what would be done.
    #[arg(short, long)]
    pub dry_run: bool,
}
