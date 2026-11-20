use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    version,
    about = "A minimal terminal UI renderer driven by a JSON config"
)]
pub struct Args {
    #[arg(help = "Path to the JSON config file")]
    pub config: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Args::command().debug_assert();
    }

    #[test]
    fn cli_config_is_positional() {
        let args = Args::try_parse_from(["rust-tui", "config.json"]).expect("args should parse");
        assert_eq!(args.config, "config.json");
    }

    #[test]
    fn cli_config_is_required() {
        assert!(Args::try_parse_from(["rust-tui"]).is_err());
    }
}
